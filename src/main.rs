use std::{env, fs};
use wayland_client::Connection;

mod bindings;
mod wayland;

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
    let libs = Libs {
        std: espystandard::StdLib,
        psybeam: bindings::PsybeamLib,
    };

    for arg in env::args().skip(1) {
        let source = fs::read_to_string(arg)?;
        // TODO: espy Errors need to implement Error.
        let program = espy::Program::try_from(source.as_str()).unwrap();
        let function = program.eval().unwrap().into_function().unwrap();
        let layout = function
            .piped(espy::Value::borrow(&libs))
            .eval()
            .unwrap()
            .into_tuple()
            .unwrap();

        for widget in layout.values() {
            if widget.downcast_extern::<bindings::SpacerWidget>().is_some() {
                print!("<--> ")
            } else {
                let draw = |instruction: &espy::Value| {
                    if let Some(bindings::Label {
                        text,
                        red,
                        green,
                        blue,
                        alpha: _,
                    }) = instruction.downcast_extern()
                    {
                        print!("\x1B[38;2;{red};{green};{blue}m{text}\x1B[0m ",);
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
        println!();
    }

    let connection = Connection::connect_to_env()?;
    let mut event_queue = connection.new_event_queue();
    let qh = event_queue.handle();
    let display = connection.display();
    let registry = display.get_registry(&qh, ());

    let mut psybeam = wayland::Psybeam::new(bindings::SurfaceConfig {
        height: 16,
        exclusive_height: Some(0),
        bottom: true,
    });

    while psybeam.running {
        event_queue.blocking_dispatch(&mut psybeam)?;
    }

    Ok(())
}
