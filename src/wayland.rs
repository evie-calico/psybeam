use crate::bindings::SurfaceConfig;
use std::{fs::File, os::unix::io::AsFd};
use wayland_client::{
    Connection, Dispatch, QueueHandle, delegate_noop,
    protocol::{wl_buffer, wl_compositor, wl_output, wl_registry, wl_shm, wl_shm_pool, wl_surface},
};

use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};

pub struct Psybeam {
    pub config: SurfaceConfig,
    pub running: bool,

    base_surface: Option<wl_surface::WlSurface>,
    buffer: Option<wl_buffer::WlBuffer>,
    layer_shell: Option<ZwlrLayerShellV1>,
    layer_surface: Option<ZwlrLayerSurfaceV1>,
    wl_output: Option<wl_output::WlOutput>,
}

impl Psybeam {
    pub fn new(config: SurfaceConfig) -> Self {
        Self {
            config,
            running: true,
            base_surface: None,
            buffer: None,
            layer_shell: None,
            layer_surface: None,
            wl_output: None,
        }
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
                    let compositor =
                        registry.bind::<wl_compositor::WlCompositor, _, _>(name, 1, qh, ());
                    let surface = compositor.create_surface(qh, ());
                    state.base_surface = Some(surface);

                    state.attempt_init(qh);
                }
                "wl_output" => {
                    let wl_output = registry.bind::<wl_output::WlOutput, _, _>(name, 1, qh, ());
                    state.wl_output = Some(wl_output);

                    state.attempt_init(qh);
                }
                "wl_shm" => {
                    let shm = registry.bind::<wl_shm::WlShm, _, _>(name, 1, qh, ());

                    let (init_w, init_h) = (500, state.config.height);

                    let mut file = tempfile::tempfile().unwrap();
                    draw(&mut file, (init_w, init_h));
                    let pool = shm.create_pool(file.as_fd(), (init_w * init_h * 4) as i32, qh, ());
                    let buffer = pool.create_buffer(
                        0,
                        init_w as i32,
                        init_h as i32,
                        (init_w * 4) as i32,
                        wl_shm::Format::Argb8888,
                        qh,
                        (),
                    );
                    state.buffer = Some(buffer.clone());

                    state.attempt_init(qh);
                }
                "zwlr_layer_shell_v1" => {
                    let layer_shell = registry.bind::<ZwlrLayerShellV1, _, _>(name, 1, qh, ());
                    state.layer_shell = Some(layer_shell);
                    state.attempt_init(qh);
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

fn draw(tmp: &mut File, (buf_x, buf_y): (u32, u32)) {
    use std::{cmp::min, io::Write};
    let mut buf = std::io::BufWriter::new(tmp);
    for y in 0..buf_y {
        for x in 0..buf_x {
            let a = 0xFF;
            let r = min(((buf_x - x) * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
            let g = min((x * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
            let b = min(((buf_x - x) * 0xFF) / buf_x, (y * 0xFF) / buf_y);
            buf.write_all(&[b as u8, g as u8, r as u8, a as u8])
                .unwrap();
        }
    }
    buf.flush().unwrap();
}

impl Psybeam {
    fn attempt_init(&mut self, qh: &QueueHandle<Psybeam>) {
        if self.layer_surface.is_some() {
            return;
        }
        let Some(base_surface) = &self.base_surface else {
            return;
        };
        let Some(layer_shell) = &self.layer_shell else {
            return;
        };
        let Some(wl_output) = &self.wl_output else {
            return;
        };
        let Some(buffer) = &self.buffer else {
            return;
        };
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
        layer_surface.set_size(500, self.config.height);
        layer_surface.set_exclusive_zone(
            self.config
                .exclusive_height
                .unwrap_or(self.config.height as i32),
        );
        base_surface.attach(Some(buffer), 0, 0);
        base_surface.commit();

        self.layer_surface = Some(layer_surface);
    }
}

impl Dispatch<wl_output::WlOutput, ()> for Psybeam {
    fn event(
        _: &mut Self,
        _: &wl_output::WlOutput,
        _: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrLayerShellV1, ()> for Psybeam {
    fn event(
        _: &mut Self,
        _: &ZwlrLayerShellV1,
        _: zwlr_layer_shell_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
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
                state.base_surface.as_ref().unwrap().commit();
            }
            zwlr_layer_surface_v1::Event::Closed => {
                state.running = false;
            }
            _ => unreachable!(),
        }
    }
}
