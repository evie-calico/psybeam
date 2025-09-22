use std::{rc::Rc, time::Duration};

use crate::{UserWidget, Widget, WidgetRefreshRate};

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

impl espy::Extern for Widget {
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

    fn as_static(&self) -> Option<espy::Value<'static>> {
        Some(espy::Value::owned(Rc::new(self.clone())))
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::write!(f, "{self:?}")
    }
}

impl espy::ExternOwned for Widget {
    fn index<'host>(
        self: Rc<Self>,
        index: espy::Value<'host>,
    ) -> Result<espy::Value<'host>, espy::Error<'host>> {
        Err(espy::Error::IndexNotFound {
            index,
            container: espy::Value::owned(self),
        })
    }

    fn any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::write!(f, "{self:?}")
    }
}

pub struct PsybeamLib;

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
            "widget" => Ok(espy::Value::borrow(&WidgetLib)),
            _ => Err(espy::Error::IndexNotFound {
                index: index.into(),
                container: espy::Value::borrow(self),
            }),
        }
    }

    fn as_static(&self) -> Option<espy::Value<'static>> {
        Some(espy::Value::borrow(&PsybeamLib))
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

    fn as_static(&self) -> Option<espy::Function<'static>> {
        Some(espy::Function::borrow(&CommandFn))
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

    fn as_static(&self) -> Option<espy::Value<'static>> {
        Some(espy::Value::borrow(&ColorLib))
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

    fn as_static(&self) -> Option<espy::Function<'static>> {
        Some(espy::Function::borrow(&ColorHexFn))
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

    fn as_static(&self) -> Option<espy::Function<'static>> {
        Some(espy::Function::borrow(&LabelColorFn))
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::write!(f, "{self:?}")
    }
}

#[derive(Debug)]
struct WidgetLib;

impl espy::Extern for WidgetLib {
    fn index<'host>(
        &'host self,
        index: espy::Value<'host>,
    ) -> Result<espy::Value<'host>, espy::Error<'host>> {
        let index = index.into_str()?;
        match &*index {
            "new" => Ok(espy::Function::borrow(&WidgetNewFn).into()),
            "refresh" => Ok(espy::Value::borrow(&WidgetRefreshLib)),
            "spacer" => Ok(espy::Value::borrow(&Widget::Spacer)),
            _ => Err(espy::Error::IndexNotFound {
                index: index.into(),
                container: espy::Value::borrow(self),
            }),
        }
    }

    fn as_static(&self) -> Option<espy::Value<'static>> {
        Some(espy::Value::borrow(&WidgetLib))
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::write!(f, "psybeam.widget module")
    }
}

#[derive(Debug)]
struct WidgetRefreshLib;

impl espy::Extern for WidgetRefreshLib {
    fn index<'host>(
        &'host self,
        index: espy::Value<'host>,
    ) -> Result<espy::Value<'host>, espy::Error<'host>> {
        let index = index.into_str()?;
        match &*index {
            "timer" => Ok(espy::Value::borrow(&WidgetRefreshTimerLib)),
            _ => Err(espy::Error::IndexNotFound {
                index: index.into(),
                container: espy::Value::borrow(self),
            }),
        }
    }

    fn as_static(&self) -> Option<espy::Value<'static>> {
        Some(espy::Value::borrow(&WidgetRefreshLib))
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::write!(f, "psybeam.widget.refresh module")
    }
}

#[derive(Debug)]
struct WidgetRefreshTimerLib;

impl espy::Extern for WidgetRefreshTimerLib {
    fn index<'host>(
        &'host self,
        index: espy::Value<'host>,
    ) -> Result<espy::Value<'host>, espy::Error<'host>> {
        let index = index.into_str()?;
        match &*index {
            "s" => Ok(espy::Function::borrow(&WidgetRefreshTimerSecondsFn).into()),
            "ms" => Ok(espy::Function::borrow(&WidgetRefreshTimerMillisecondsFn).into()),
            _ => Err(espy::Error::IndexNotFound {
                index: index.into(),
                container: espy::Value::borrow(self),
            }),
        }
    }

    fn as_static(&self) -> Option<espy::Value<'static>> {
        Some(espy::Value::borrow(&WidgetRefreshTimerLib))
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::write!(f, "psybeam.widget.refresh.timer module")
    }
}

#[derive(Debug)]
struct WidgetRefreshTimerSecondsFn;

impl espy::ExternFn for WidgetRefreshTimerSecondsFn {
    fn call<'host>(
        &'host self,
        argument: espy::Value<'host>,
    ) -> Result<espy::Value<'host>, espy::Error<'host>> {
        Ok(espy::Value::owned(Rc::new(WidgetRefreshRate::Timer(
            Duration::from_secs(argument.into_i64()?.try_into()?),
        ))))
    }

    fn as_static(&self) -> Option<espy::Function<'static>> {
        Some(espy::Function::borrow(&WidgetRefreshTimerSecondsFn))
    }
}

#[derive(Debug)]
struct WidgetRefreshTimerMillisecondsFn;

impl espy::ExternFn for WidgetRefreshTimerMillisecondsFn {
    fn call<'host>(
        &'host self,
        argument: espy::Value<'host>,
    ) -> Result<espy::Value<'host>, espy::Error<'host>> {
        Ok(espy::Value::owned(Rc::new(WidgetRefreshRate::Timer(
            Duration::from_millis(argument.into_i64()?.try_into()?),
        ))))
    }

    fn as_static(&self) -> Option<espy::Function<'static>> {
        Some(espy::Function::borrow(&WidgetRefreshTimerMillisecondsFn))
    }
}

#[derive(Debug)]
struct WidgetNewFn;

impl espy::ExternFn for WidgetNewFn {
    fn call<'host>(
        &'host self,
        argument: espy::Value<'host>,
    ) -> Result<espy::Value<'host>, espy::Error<'host>> {
        let title = argument.find("title".into())?.into_str()?;
        let width = argument.find("width".into())?.into_i64()?.try_into()?;
        let refresh = argument
            .find("refresh".into())?
            .downcast_extern::<WidgetRefreshRate>()
            .ok_or_else(|| espy::Error::Other("expected widget refresh rate".into()))?
            .clone();
        let draw = argument
            .find("draw".into())?
            .into_function()?
            .as_static()
            .ok_or_else(|| {
                espy::Error::Other(
                    "expected static draw method (no mutable or external state)".into(),
                )
            })?;
        Ok(espy::Value::owned(Rc::new(Widget::User(UserWidget {
            title,
            width,
            refresh,
            draw,
        }))))
    }

    fn as_static(&self) -> Option<espy::Function<'static>> {
        Some(espy::Function::borrow(&WidgetNewFn))
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::write!(f, "psybeam.widget.new function")
    }
}
