use super::error::ParameterError;

pub enum ControlWord {
    // Hide the parameter in the list of parameters in the ACAP settings interface.
    Hidden,
    // The changes on the parameter last until the ACAP is restarted or the parameter is reloaded.
    NoSync,
    // The parameter cannot be modified by axparameter nor param.cgi, and the parameter is grayed
    // out in the ACAP settings interface.
    ReadOnly,
}

pub trait ParameterValue {
    fn to_param_type() -> String;
    fn from_param_string(param: String) -> Result<Self, glib::error::Error>
    where
        Self: Sized;
    fn to_param_string(&self) -> String;
}

impl ParameterValue for String {
    fn to_param_type() -> String {
        String::from("string")
    }

    fn from_param_string(param: String) -> Result<Self, glib::error::Error> {
        Ok(param)
    }

    fn to_param_string(&self) -> String {
        self.to_string()
    }
}

impl ParameterValue for bool {
    fn to_param_type() -> String {
        String::from("bool:no,yes")
    }

    fn from_param_string(param: String) -> Result<Self, glib::error::Error> {
        match param.as_str() {
            "yes" => Ok(true),
            "no" => Ok(false),
            _ => Err(glib::error::Error::new::<ParameterError>(
                ParameterError::ParamGet,
                "Unable to convert param to bool",
            )),
        }
    }

    fn to_param_string(&self) -> String {
        match self {
            true => String::from("yes"),
            false => String::from("no"),
        }
    }
}

macro_rules! impl_param_value_int {
    (for $($t:ty),+) => {
        $(impl ParameterValue for $t {
            fn to_param_type() -> String {
                format!("int:min={};max={}", <$t>::MIN, <$t>::MAX)
            }

            fn from_param_string(param: String) -> Result<Self, glib::error::Error> {
                param
                    .parse::<$t>()
                    .map_err(|_| {
                        glib::error::Error::new::<ParameterError>(
                            ParameterError::ParamGet,
                            &format!("Unable to convert {} to {}", param, stringify!($t)))
                    })
            }

            fn to_param_string(&self) -> String {
                format!("{}", self)
            }
        })*
    }
}

impl_param_value_int!(for u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
