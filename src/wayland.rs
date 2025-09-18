use crate::bindings::SurfaceConfig;
use std::io::Write;
use std::os::unix::io::AsFd;
use wayland_client::{
    Connection, Dispatch, QueueHandle, delegate_noop,
    protocol::{
        wl_buffer, wl_callback, wl_compositor, wl_output, wl_registry, wl_shm, wl_shm_pool,
        wl_surface,
    },
};

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

pub struct Psybeam {
    pub config: SurfaceConfig,
    pub running: bool,
    resources: PsybeamResources,
}

impl Psybeam {
    pub fn new(config: SurfaceConfig) -> Self {
        Self {
            config,
            running: true,
            resources: PsybeamResources::Partial(PsybeamPartial::default()),
        }
    }

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
            match &interface[..] {
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

        let mut file = std::io::BufWriter::new(tempfile::tempfile().unwrap());
        let pool_size = width as usize * height as usize * size_of::<u32>();

        let message = "meow, world!";

        for _ in 0..(pool_size / message.len()) {
            let _ = file.write(message.as_bytes());
        }
        for _ in 0..(pool_size % message.len()) {
            let _ = file.write(&[0]);
        }
        let file = file.into_inner().unwrap();
        let pool = wl_shm.create_pool(file.as_fd(), pool_size as i32, qh, ());
        let buffer = pool.create_buffer(
            0,
            width as i32,
            height as i32,
            (width * 4) as i32,
            wl_shm::Format::Argb8888,
            qh,
            (),
        );
        let layer_surface = layer_shell.get_layer_surface(
            base_surface,
            Some(wl_output),
            zwlr_layer_shell_v1::Layer::Bottom,
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
        base_surface.attach(Some(&buffer), 0, 0);
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
        match event {
            wl_callback::Event::Done { callback_data } => {
                let base_surface = &mut state.final_resources().base_surface;
                base_surface.frame(qh, ());
                base_surface.commit();
                println!("{callback_data}");
            }
            _ => todo!(),
        }
    }
}
