use axparameter::{error::ParameterError, parameter::Parameter, types::ControlWord};
use log::{error, info};

const APP_NAME: &str = "axparameter_example";

fn main() {
    acap_logging::init_logger();

    let main_loop = glib::MainLoop::new(None, false);
    glib::unix_signal_add_once(15, {
        let main_loop = main_loop.clone();
        move || main_loop.quit()
    });
    glib::unix_signal_add_once(2, {
        let main_loop = main_loop.clone();
        move || main_loop.quit()
    });

    info!("Starting {APP_NAME}");

    let parameter = Parameter::new(APP_NAME).unwrap();

    if let Err(err) = parameter.add("Parameter", None, "param".to_string()) {
        if !err.matches::<ParameterError>(ParameterError::ParamAdded) {
            error!("Failed {}", err);
            return;
        }
    }

    if let Err(err) = parameter.add("ParameterTwo", None, "param_two".to_string()) {
        if !err.matches::<ParameterError>(ParameterError::ParamAdded) {
            error!("Failed {}", err);
            return;
        }
    }

    if let Err(err) = parameter.add("ParameterThree", None, "param_three".to_string()) {
        if !err.matches::<ParameterError>(ParameterError::ParamAdded) {
            error!("Failed {}", err);
            return;
        }
    }

    if let Err(err) = parameter.add(
        "ParameterFour",
        Some(ControlWord::ReadOnly),
        "param_four".to_string(),
    ) {
        if !err.matches::<ParameterError>(ParameterError::ParamAdded) {
            error!("Failed {}", err);
            return;
        }
    }

    let value: String = match parameter.get("Parameter") {
        Ok(value) => value,
        Err(err) => {
            error!("Failed {}", err);
            return;
        }
    };

    info!("The value of \"Parameter\" is \"{}\"", value);

    let value: String = match parameter.get("ParameterFour") {
        Ok(value) => value,
        Err(err) => {
            error!("Failed {}", err);
            return;
        }
    };

    info!("The value of \"ParameterFour\" is \"{}\"", value);

    if let Err(err) = parameter.remove("ParameterThree") {
        error!("Failed {}", err);
        return;
    }

    if let Err(err) = parameter.set("Parameter", "param_set".to_string(), true) {
        error!("Failed {}", err);
        return;
    }

    if let Ok(()) = parameter.set("ParameterFour", "param_four_set".to_string(), true) {
        error!("Failed ParameterFour should be ReadOnly");
        return;
    }

    if let Ok(list) = parameter.list() {
        for param in list {
            info!("Parameter in list: \"{}\"", param);
        }
    } else {
        error!("Failed {}", value);
        return;
    }

    if let Err(err) = parameter.register_callback("Parameter", |_name, _value| {
        info!("In Parameter callback");
    }) {
        error!("Failed {}", err);
        return;
    }

    main_loop.run();
}

#[cfg(test)]
mod tests;
