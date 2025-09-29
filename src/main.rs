use std::{env, fs, rc::Rc};
use wayland_client::Connection;

mod bindings;
mod wayland;

#[derive(Clone)]
pub struct SurfaceConfig {
    /// Allocate this many rows for drawing.
    pub height: u32,
    /// Request exclusive residence from the compositor.
    ///
    /// This can be used to implement margins,
    /// or to allow windows to draw over the bar.
    ///
    /// Defaults to `height` if not specified.
    pub exclusive_height: Option<i32>,
    /// Anchor the bar to the bottom of the output,
    /// instead of the top.
    pub bottom: bool,
}

#[derive(Clone, Debug)]
pub enum WidgetRefreshRate {
    Timer(std::time::Duration),
    Framerate,
}

#[derive(Clone, Debug)]
pub enum Widget {
    Spacer,
    User(UserWidget),
}

#[derive(Clone, Debug)]
pub struct UserWidget {
    pub title: Rc<str>,
    pub width: u32,
    pub refresh: WidgetRefreshRate,
    pub draw: espy::Function<'static>,
}

pub struct Psybeam {
    pub config: SurfaceConfig,
    pub running: bool,
    pub resources: wayland::PsybeamResources,
    pub layout: Box<[Widget]>,
    pub swash_cache: cosmic_text::SwashCache,
    pub font_system: cosmic_text::FontSystem,
}

impl Psybeam {
    pub fn new(config: SurfaceConfig, layout: Box<[Widget]>) -> Self {
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

espy::extern_impl! {
    #[espy(debug = "psybeam libraries")]
    struct Libs {
        std: espy::Value::borrow(&espystandard::Lib),
        psybeam: espy::Value::borrow(&bindings::Lib),
    }
}

fn main() -> anyhow::Result<()> {
    let path = env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("expected one argument"))?;
    let source = fs::read_to_string(&path)?;
    // TODO: espy Errors need to implement Error.
    // TODO: shortcut function to skip lexer?
    let ast = espy::parser::Block::new(&mut espy::lexer::Lexer::from(source.as_str()).peekable());
    espy::diagnostics::for_each(&source, &ast, |error| {
        for comment in [error.primary]
            .into_iter()
            .chain(error.secondary.into_iter())
        {
            eprintln!("error: {}", comment.message);
            if let Some((first, last)) = comment.range {
                let snippet = espy::diagnostics::expand_to_snippet(first, last, &source);
                let ((line, column), (_, _)) =
                    espy::diagnostics::find_location(first, last, &source);
                eprintln!("  |> {path}:{line}:{column}");
                eprintln!("{snippet}");
            } else {
                eprintln!("  |> {path}");
            }
            eprintln!();
        }
    });

    let program = espy::Program::try_from(source.as_str()).unwrap();
    let function = program.eval().unwrap().into_function().unwrap();
    let config = function
        .piped(espy::Value::borrow(&Libs))
        .eval()
        .unwrap()
        .into_tuple()
        .unwrap();
    let surface = config.find_value("surface").cloned().map_or(
        SurfaceConfig {
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
            SurfaceConfig {
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
        .unwrap()
        .values()
        .map(|value| value.downcast_extern::<Widget>().unwrap().clone())
        .collect::<Box<[Widget]>>();

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
