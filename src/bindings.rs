use std::{cell::RefCell, rc::Rc};

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

pub struct SurfaceLib(RefCell<SurfaceConfig>);

impl espy::Extern for SurfaceLib {
    fn any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn index<'host>(
        &'host self,
        index: espy::Value<'host>,
    ) -> Result<espy::Value<'host>, espy::Error<'host>> {
        struct HeightFn;
        impl espy::ExternFn for HeightFn {
            fn call<'host>(
                &'host self,
                argument: espy::Value<'host>,
            ) -> Result<espy::Value<'host>, espy::Error<'host>> {
                argument
                    .get(0)?
                    .downcast_extern::<SurfaceLib>()
                    .ok_or_else(|| "first argument must be surface lib".to_string())
                    .map_err(|s| espy::Error::Other(s.into()))?
                    .0
                    .borrow_mut()
                    .height = argument.get(1)?.into_i64()? as u32;
                Ok(().into())
            }
        }

        struct ExclusiveHeightFn;
        impl espy::ExternFn for ExclusiveHeightFn {
            fn call<'host>(
                &'host self,
                argument: espy::Value<'host>,
            ) -> Result<espy::Value<'host>, espy::Error<'host>> {
                argument
                    .get(0)?
                    .downcast_extern::<SurfaceLib>()
                    .unwrap()
                    .0
                    .borrow_mut()
                    .exclusive_height = Some(argument.get(1)?.into_i64()? as i32);
                Ok(().into())
            }
        }

        struct TopFn;
        impl espy::ExternFn for TopFn {
            fn call<'host>(
                &'host self,
                argument: espy::Value<'host>,
            ) -> Result<espy::Value<'host>, espy::Error<'host>> {
                argument
                    .downcast_extern::<SurfaceLib>()
                    .unwrap()
                    .0
                    .borrow_mut()
                    .bottom = false;
                Ok(().into())
            }
        }

        struct BottomFn;
        impl espy::ExternFn for BottomFn {
            fn call<'host>(
                &'host self,
                argument: espy::Value<'host>,
            ) -> Result<espy::Value<'host>, espy::Error<'host>> {
                argument
                    .downcast_extern::<SurfaceLib>()
                    .unwrap()
                    .0
                    .borrow_mut()
                    .bottom = true;
                Ok(().into())
            }
        }

        let index = index.into_str()?;
        match &*index {
            "height" => Ok(espy::Function::borrow(&HeightFn)
                .piped(espy::Value::borrow(self))
                .into()),
            "exclusive_height" => Ok(espy::Function::borrow(&ExclusiveHeightFn)
                .piped(espy::Value::borrow(self))
                .into()),
            "top" => Ok(espy::Function::borrow(&TopFn)
                .piped(espy::Value::borrow(self))
                .into()),
            "bottom" => Ok(espy::Function::borrow(&BottomFn)
                .piped(espy::Value::borrow(self))
                .into()),
            _ => Err(espy::Error::IndexNotFound {
                index: index.into(),
                container: espy::Value::borrow(self),
            }),
        }
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "psybeam.surface module")
    }
}

pub struct PsybeamLib {
    pub surface_config: SurfaceLib,
}

impl PsybeamLib {
    pub fn new() -> Self {
        Self {
            surface_config: SurfaceLib(RefCell::new(SurfaceConfig {
                height: 16,
                exclusive_height: None,
                bottom: false,
            })),
        }
    }
    pub fn surface_config(&self) -> SurfaceConfig {
        (*self.surface_config.0.borrow()).clone()
    }
}

impl espy::Extern for PsybeamLib {
    fn index<'host>(
        &'host self,
        index: espy::Value<'host>,
    ) -> Result<espy::Value<'host>, espy::Error<'host>> {
        let index = index.into_str()?;
        match &*index {
            "color" => Ok(espy::Value::borrow(&ColorLib)),
            "command" => Ok(espy::Function::borrow(&CommandFn).into()),
            "label_color" => Ok(espy::Function::borrow(&LabelColorFn).into()),
            "spacer" => Ok(espy::Value::borrow(&SpacerWidget)),
            "surface" => Ok(espy::Value::borrow(&self.surface_config)),
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
