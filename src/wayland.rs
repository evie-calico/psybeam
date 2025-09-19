use crate::{Psybeam, bindings};
use cosmic_text::{Attrs, Buffer, Color, Metrics};
use std::io::Write;
use std::iter;
use std::os::unix::io::AsFd;
use wayland_client::protocol::{
    wl_buffer, wl_callback, wl_compositor, wl_output, wl_registry, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};

#[derive(Default)]
pub struct PsybeamPartial {
    wl_output: Option<wl_output::WlOutput>,
    wl_shm: Option<wl_shm::WlShm>,
    base_surface: Option<wl_surface::WlSurface>,
    layer_shell: Option<ZwlrLayerShellV1>,
    width: Option<u32>,
}

pub struct PsybeamPartialRef<'a>(&'a mut Psybeam, &'a QueueHandle<Psybeam>);

impl std::ops::Deref for PsybeamPartialRef<'_> {
    type Target = PsybeamPartial;
    fn deref(&self) -> &Self::Target {
        let PsybeamResources::Partial(partial) = &self.0.resources else {
            unreachable!();
        };
        partial
    }
}

impl std::ops::DerefMut for PsybeamPartialRef<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let PsybeamResources::Partial(partial) = &mut self.0.resources else {
            unreachable!();
        };
        partial
    }
}

impl Drop for PsybeamPartialRef<'_> {
    fn drop(&mut self) {
        self.0.attempt_init(self.1);
    }
}

pub struct PsybeamFinal {
    wl_output: wl_output::WlOutput,
    wl_shm: wl_shm::WlShm,
    base_surface: wl_surface::WlSurface,
    layer_shell: ZwlrLayerShellV1,
    width: u32,

    layer_surface: ZwlrLayerSurfaceV1,
}

pub enum PsybeamResources {
    Partial(PsybeamPartial),
    Final(PsybeamFinal),
}

impl Psybeam {
    fn resources<'a>(&'a mut self, qh: &'a QueueHandle<Self>) -> Option<PsybeamPartialRef<'a>> {
        match &self.resources {
            PsybeamResources::Partial(_) => Some(PsybeamPartialRef(self, qh)),
            PsybeamResources::Final(_) => None,
        }
    }

    fn final_resources(&mut self) -> &mut PsybeamFinal {
        let PsybeamResources::Final(final_resources) = &mut self.resources else {
            unreachable!();
        };
        final_resources
    }

    fn draw(&mut self, qh: &QueueHandle<Self>) {
        let width = self.final_resources().width;
        let height = self.config.height;

        let mut buffer = Buffer::new(
            &mut self.font_system,
            Metrics {
                font_size: 14.0,
                line_height: height as f32,
            },
        );
        let mut buffer = buffer.borrow_with(&mut self.font_system);
        buffer.set_size(Some(width as f32), Some(height as f32));
        let attrs = Attrs::new();
        buffer.shape_until_scroll(true);

        let pool_size = width as usize * height as usize * size_of::<u32>();
        let mut canvas: Box<[u8]> = iter::repeat_n(0, pool_size).collect();
        let mut cursor = 0;

        for widget in self.layout.values() {
            if widget.downcast_extern::<bindings::SpacerWidget>().is_some() {
            } else {
                let mut draw = |instruction: &espy::Value| {
                    if let Some(bindings::Label {
                        text,
                        red,
                        green,
                        blue,
                        alpha,
                    }) = instruction.downcast_extern()
                    {
                        buffer.set_text(text, &attrs, cosmic_text::Shaping::Advanced);
                        let mut furthest_right = 0;
                        buffer.draw(
                            &mut self.swash_cache,
                            Color::rgba(*red, *green, *blue, *alpha),
                            |x, y, w, h, color| {
                                if color.a() == 0 {
                                    return;
                                }
                                let x = x + cursor;
                                furthest_right = furthest_right.max(x + w as i32);
                                for y in y..(y + h as i32) {
                                    for x in x..(x + w as i32) {
                                        let pos =
                                            (x + y * width as i32) as usize * size_of::<u32>();
                                        if let Some(dest) =
                                            canvas.get_mut(pos..(pos + size_of::<u32>()))
                                        {
                                            let a = color.a();

                                            let encode = |x: u8| (x as u16 * a as u16 / 255) as u8;
                                            dest[0] = encode(color.b());
                                            dest[1] = encode(color.g());
                                            dest[2] = encode(color.r());
                                            dest[3] = color.a();
                                        }
                                    }
                                }
                            },
                        );
                        cursor = furthest_right;
                    } else {
                        eprintln!("unrecognized drawing instruction: {instruction:?}");
                    }
                };
                match widget.clone().into_function().unwrap().eval() {
                    // Unit represents no drawing instructions.
                    Ok(espy::Value::Unit) => (),
                    Ok(espy::Value::Tuple(instructions)) => instructions.values().for_each(draw),
                    Ok(instruction) => draw(&instruction),
                    Err(e) => {
                        eprintln!("widget renderer failed: {e:?}");
                    }
                }
            }
        }

        let resources = self.final_resources();
        let base_surface = &mut resources.base_surface;
        base_surface.frame(qh, ());
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(&canvas).unwrap();
        let pool = resources
            .wl_shm
            .create_pool(file.as_fd(), pool_size as i32, qh, ());
        let buffer = pool.create_buffer(
            0,
            width as i32,
            height as i32,
            (width * 4) as i32,
            wl_shm::Format::Argb8888,
            qh,
            (),
        );
        base_surface.damage(0, 0, width as i32, height as i32);
        base_surface.attach(Some(&buffer), 0, 0);
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for Psybeam {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name, interface, ..
        } = event
        {
            match &*interface {
                "wl_compositor" => {
                    if let Some(mut resources) = state.resources(qh) {
                        let compositor =
                            registry.bind::<wl_compositor::WlCompositor, _, _>(name, 1, qh, ());
                        let surface = compositor.create_surface(qh, ());
                        resources.base_surface = Some(surface);
                    }
                }
                "wl_output" => {
                    if let Some(mut resources) = state.resources(qh) {
                        let wl_output = registry.bind::<wl_output::WlOutput, _, _>(name, 1, qh, ());
                        resources.wl_output = Some(wl_output);
                    }
                }
                "wl_shm" => {
                    if let Some(mut resources) = state.resources(qh) {
                        let shm = registry.bind::<wl_shm::WlShm, _, _>(name, 1, qh, ());
                        resources.wl_shm = Some(shm);
                    }
                }
                "zwlr_layer_shell_v1" => {
                    if let Some(mut resources) = state.resources(qh) {
                        let layer_shell = registry.bind::<ZwlrLayerShellV1, _, _>(name, 1, qh, ());
                        resources.layer_shell = Some(layer_shell);
                    }
                }
                _ => {}
            }
        }
    }
}

// Ignore events from these object types in this example.
delegate_noop!(Psybeam: ignore wl_compositor::WlCompositor);
delegate_noop!(Psybeam: ignore wl_surface::WlSurface);
delegate_noop!(Psybeam: ignore wl_shm::WlShm);
delegate_noop!(Psybeam: ignore wl_shm_pool::WlShmPool);
delegate_noop!(Psybeam: ignore wl_buffer::WlBuffer);
delegate_noop!(Psybeam: ignore zwlr_layer_shell_v1::ZwlrLayerShellV1);

impl Psybeam {
    fn attempt_init(&mut self, qh: &QueueHandle<Psybeam>) {
        let PsybeamResources::Partial(PsybeamPartial {
            wl_output: Some(wl_output),
            wl_shm: Some(wl_shm),
            base_surface: Some(base_surface),
            layer_shell: Some(layer_shell),
            width: Some(width),
        }) = &self.resources
        else {
            return;
        };
        let width = *width;
        let height = self.config.height;

        let layer_surface = layer_shell.get_layer_surface(
            base_surface,
            Some(wl_output),
            zwlr_layer_shell_v1::Layer::Top,
            "beam".into(),
            qh,
            (),
        );
        layer_surface.set_anchor(if self.config.bottom {
            zwlr_layer_surface_v1::Anchor::Bottom
        } else {
            zwlr_layer_surface_v1::Anchor::Top
        });
        layer_surface.set_size(width, height);
        layer_surface.set_exclusive_zone(
            self.config
                .exclusive_height
                .unwrap_or(self.config.height as i32),
        );
        base_surface.frame(qh, ());
        base_surface.commit();

        self.resources = PsybeamResources::Final(PsybeamFinal {
            wl_output: wl_output.clone(),
            wl_shm: wl_shm.clone(),
            base_surface: base_surface.clone(),
            layer_shell: layer_shell.clone(),
            width,
            layer_surface,
        })
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for Psybeam {
    fn event(
        state: &mut Self,
        surface: &ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width: _,
                height: _,
            } => {
                surface.ack_configure(serial);
                state.final_resources().base_surface.commit();
            }
            zwlr_layer_surface_v1::Event::Closed => {
                state.running = false;
            }
            _ => unreachable!(),
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for Psybeam {
    fn event(
        state: &mut Self,
        _: &wl_output::WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        // TODO: scale
        if let wl_output::Event::Mode { width, .. } = event {
            if let Some(mut resources) = state.resources(qh) {
                resources.width = Some(width as u32);
            } else {
                state.final_resources().width = width as u32;
            }
        }
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for Psybeam {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            state.draw(qh);
            state.final_resources().base_surface.frame(qh, ());
            state.final_resources().base_surface.commit();
        }
    }
}
