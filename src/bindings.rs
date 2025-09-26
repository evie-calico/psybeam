use crate::{UserWidget, Widget, WidgetRefreshRate};
use espy::extern_impl;
use std::{rc::Rc, time::Duration};

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

impl espy::ExternOwned for WidgetRefreshRate {
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

extern_impl! {
    #[espy(debug = "psybeam module")]
    pub struct Lib {
        color: espy::Value::borrow(&ColorLib),
        command: espy::Function::borrow(&CommandFn),
        read_to_string: espy::Function::borrow(&ReadToStringFn),
        label_color: espy::Function::borrow(&LabelColorFn),
        widget: espy::Value::borrow(&WidgetLib),
    }
}

extern_impl! {
    #[espy(debug = "psybeam.command function")]
    fn CommandFn<'host>(&self, argument) {
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
}

extern_impl! {
    #[espy(debug = "psybeam.read_to_string function")]
    fn ReadToStringFn<'host>(&self, argument) {
        Ok(espy::Value::String(
            std::fs::read_to_string(&*argument.into_str()?)?.into(),
        ))
    }
}

extern_impl! {
    #[espy(debug = "psybeam.color module")]
    struct ColorLib {
        hex: espy::Function::borrow(&ColorHexFn),
    }
}

extern_impl! {
    #[espy(debug = "psybeam.color.hex function")]
    fn ColorHexFn<'host>(&self, argument) {
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
}

extern_impl! {
    #[espy(debug = "psybeam.label_color function")]
    fn LabelColorFn<'host>(&self, argument) {
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
}

extern_impl! {
    #[espy(debug = "psybeam.widget module")]
    struct WidgetLib {
        new: espy::Function::borrow(&WidgetNewFn),
        refresh: espy::Value::borrow(&WidgetRefreshLib),
        spacer: espy::Value::borrow(&Widget::Spacer),
    }
}

extern_impl! {
    #[espy(debug = "psybeam.widget.refresh module")]
    struct WidgetRefreshLib {
        timer: espy::Value::borrow(&WidgetRefreshTimerLib),
    }
}

extern_impl! {
    #[espy(debug = "psybeam.widget.refresh.timer module")]
    struct WidgetRefreshTimerLib {
        s: espy::Function::borrow(&WidgetRefreshTimerSecondsFn),
        ms: espy::Function::borrow(&WidgetRefreshTimerMillisecondsFn),
    }
}

extern_impl! {
    #[espy(debug = "psybeam.widget.refresh.timer.seconds function")]
    fn WidgetRefreshTimerSecondsFn<'host>(&self, argument) {
        Ok(espy::Value::owned(Rc::new(WidgetRefreshRate::Timer(
            Duration::from_secs(argument.into_i64()?.try_into()?),
        ))))
    }
}

extern_impl! {
    #[espy(debug = "psybeam.widget.refresh.timer.milliseconds function")]
    fn WidgetRefreshTimerMillisecondsFn<'host>(&self, argument) {
        Ok(espy::Value::owned(Rc::new(WidgetRefreshRate::Timer(
            Duration::from_millis(argument.into_i64()?.try_into()?),
        ))))
    }
}

extern_impl! {
    #[espy(debug = "psybeam.widget.new function")]
    fn WidgetNewFn<'host>(&self, argument) {
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
}
