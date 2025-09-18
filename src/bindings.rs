use std::rc::Rc;

#[derive(Debug)]
pub struct Label {
    pub text: Rc<str>,

    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl espy::ExternOwned for Label {
    fn index<'host>(
        self: Rc<Self>,
        index: espy::Value<'host>,
    ) -> Result<espy::Value<'host>, espy::Error<'host>> {
        Err(espy::Error::IndexNotFound {
            index,
            container: espy::Value::Owned(self),
        })
    }

    fn any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

pub struct SpacerWidget;

impl espy::Extern for SpacerWidget {
    fn index<'host>(
        &'host self,
        index: espy::Value<'host>,
    ) -> Result<espy::Value<'host>, espy::Error<'host>> {
        Err(espy::Error::IndexNotFound {
            index,
            container: espy::Value::borrow(self),
        })
    }

    fn any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::write!(f, "psybeam.spacer widget")
    }
}

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

pub struct PsybeamLib;

impl espy::Extern for PsybeamLib {
    fn index<'host>(
        &'host self,
        index: espy::Value<'host>,
    ) -> Result<espy::Value<'host>, espy::Error<'host>> {
        static COMMAND: CommandFn = CommandFn;
        static COLOR: ColorLib = ColorLib;
        static LABEL_COLOR: LabelColorFn = LabelColorFn;
        static SPACER: SpacerWidget = SpacerWidget;

        let index = index.into_str()?;
        match &*index {
            "color" => Ok(espy::Value::borrow(&COLOR)),
            "command" => Ok(espy::Function::borrow(&COMMAND).into()),
            "label_color" => Ok(espy::Function::borrow(&LABEL_COLOR).into()),
            "spacer" => Ok(espy::Value::borrow(&SPACER)),
            _ => Err(espy::Error::IndexNotFound {
                index: index.into(),
                container: espy::Value::borrow(self),
            }),
        }
    }
    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "psybeam module")
    }
}

struct CommandFn;

impl espy::ExternFn for CommandFn {
    fn call<'host>(
        &'host self,
        argument: espy::Value<'host>,
    ) -> Result<espy::Value<'host>, espy::Error<'host>> {
        let mut command = std::process::Command::new(&*argument.get(0)?.into_str()?);
        for argument in argument.into_tuple()?.values().skip(1) {
            command.arg(&*argument.clone().into_str()?);
        }
        // TODO: espystandard or espy should provide a result type.
        let (status, stdout, stderr) = command.output().map_or_else(
            |e| (255, Rc::from(""), Rc::from(e.to_string().as_str())),
            |output| {
                (
                    output.status.code().unwrap_or(255),
                    String::from_utf8_lossy(&output.stdout).into(),
                    String::from_utf8_lossy(&output.stderr).into(),
                )
            },
        );
        Ok(espy::Value::Tuple(
            [
                (Rc::from("status"), espy::Value::I64(status as i64)),
                (Rc::from("stdout"), espy::Value::String(stdout)),
                (Rc::from("stderr"), espy::Value::String(stderr)),
            ]
            // TODO: espy doesn't rexport Tuple
            .into(),
        ))
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::write!(f, "psybeam.command function")
    }
}

struct ColorLib;

impl espy::Extern for ColorLib {
    fn index<'host>(
        &'host self,
        index: espy::Value<'host>,
    ) -> Result<espy::Value<'host>, espy::Error<'host>> {
        static HEX: ColorHexFn = ColorHexFn;

        let index = index.into_str()?;
        match &*index {
            "hex" => Ok(espy::Function::borrow(&HEX).into()),
            _ => Err(espy::Error::IndexNotFound {
                index: index.into(),
                container: espy::Value::borrow(self),
            }),
        }
    }
    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "color module")
    }
}

struct ColorHexFn;

impl espy::ExternFn for ColorHexFn {
    fn call<'host>(
        &'host self,
        argument: espy::Value<'host>,
    ) -> Result<espy::Value<'host>, espy::Error<'host>> {
        let argument = argument.into_str()?;
        let argument = argument.strip_prefix('#').unwrap_or(&argument);
        match argument.len() {
            6 => Ok(espy::Value::from(
                (u32::from_str_radix(argument, 16)? << 8) as i64,
            )),
            8 => Ok(espy::Value::from(u32::from_str_radix(argument, 16)? as i64)),
            _ => Err(espy::Error::Other(
                "expected a string with exactly six characters".into(),
            )),
        }
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "psybeam.color.hex function")
    }
}

#[derive(Debug)]
struct LabelColorFn;

impl espy::ExternFn for LabelColorFn {
    fn call<'host>(
        &'host self,
        argument: espy::Value<'host>,
    ) -> Result<espy::Value<'host>, espy::Error<'host>> {
        let text = argument.get(0)?.into_str()?;
        let [red, green, blue, alpha] = (argument.get(1)?.into_i64()? as u32).to_be_bytes();
        Ok(espy::Value::owned(Rc::new(Label {
            text,
            red,
            green,
            blue,
            alpha,
        })))
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::write!(f, "{self:?}")
    }
}
