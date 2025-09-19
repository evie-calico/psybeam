use std::fmt::Write;
use std::{env, fs};
use wayland_client::Connection;

mod bindings;
mod wayland;

pub struct Psybeam {
    pub config: bindings::SurfaceConfig,
    pub running: bool,
    pub resources: wayland::PsybeamResources,
    pub layout: espy::interpreter::Tuple<espy::Value<'static>>,
    pub swash_cache: cosmic_text::SwashCache,
    pub font_system: cosmic_text::FontSystem,
}

impl Psybeam {
    pub fn new(
        config: bindings::SurfaceConfig,
        layout: espy::interpreter::Tuple<espy::Value<'static>>,
    ) -> Self {
        Self {
            config,
            running: true,
            resources: wayland::PsybeamResources::Partial(wayland::PsybeamPartial::default()),
            layout,
            swash_cache: cosmic_text::SwashCache::new(),
            font_system: cosmic_text::FontSystem::new(),
        }
    }
}

struct Libs {
    std: espystandard::StdLib,
    psybeam: bindings::PsybeamLib,
}

impl espy::Extern for Libs {
    fn index<'host>(
        &'host self,
        index: espy::Value<'host>,
    ) -> Result<espy::Value<'host>, espy::Error<'host>> {
        let index = index.into_str()?;
        match &*index {
            "std" => Ok(espy::Value::borrow(&self.std)),
            "psybeam" => Ok(espy::Value::borrow(&self.psybeam)),
            _ => Err(espy::Error::IndexNotFound {
                index: index.into(),
                container: espy::Value::borrow(self),
            }),
        }
    }
    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "psybeam libraries")
    }
}

fn main() -> anyhow::Result<()> {
    static LIBS: Libs = Libs {
        std: espystandard::StdLib,
        psybeam: bindings::PsybeamLib,
    };

    let source = fs::read_to_string(
        env::args()
            .nth(1)
            .ok_or_else(|| anyhow::anyhow!("expected one argument"))?,
    )?;
    // TODO: espy Errors need to implement Error.
    let program = espy::Program::try_from(source.as_str()).unwrap();
    let function = program.eval().unwrap().into_function().unwrap();
    let config = function
        .piped(espy::Value::borrow(&LIBS))
        .eval()
        .unwrap()
        .into_tuple()
        .unwrap();
    let surface = config.find_value("surface").cloned().map_or(
        bindings::SurfaceConfig {
            height: 32,
            exclusive_height: None,
            bottom: false,
        },
        |surface_config| {
            let surface_config = surface_config.into_tuple().unwrap();
            let height = surface_config
                .find_value("height")
                .cloned()
                .map_or(32, |height| height.into_i64().unwrap() as u32);
            let exclusive_height = surface_config
                .find_value("exclusive_height")
                .cloned()
                .map(|exclusive_height| exclusive_height.into_i64().unwrap() as i32);
            let bottom = surface_config
                .find_value("anchor")
                .cloned()
                .is_some_and(|anchor| match &*anchor.into_str().unwrap() {
                    "top" => false,
                    "bottom" => true,
                    _ => todo!(),
                });
            bindings::SurfaceConfig {
                height,
                exclusive_height,
                bottom,
            }
        },
    );
    let layout = config
        .find_value("layout")
        .unwrap()
        .clone()
        .into_tuple()
        .unwrap();

    let mut psybeam = Psybeam::new(surface, layout);

    let connection = Connection::connect_to_env()?;
    let mut event_queue = connection.new_event_queue();
    let qh = event_queue.handle();
    let display = connection.display();
    let _ = display.get_registry(&qh, ());

    while psybeam.running {
        event_queue.blocking_dispatch(&mut psybeam)?;
    }

    Ok(())
}
