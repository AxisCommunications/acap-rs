use axparameter::{error::ParameterError, parameter::Parameter};

use crate::APP_NAME;

const TEST_CONF_PATH: &str = "/etc/dynamic/axparameter_example.conf";
const TEST_DEF_PATH: &str = "/etc/dynamic/param/axparameter_example.conf";

fn param_in_deffile(file_content: &str, param_name: &str) -> Result<(), String> {
    let len = file_content.len();

    let i = match file_content[..].find("param") {
        Some(i) => i,
        None => return Err("Couldn't find string \"param\"".to_string()),
    };
    let i = match file_content[i..len].find(param_name) {
        Some(i) => i,
        None => return Err(format!("Couldn't find string \"{param_name}\"")),
    };
    let i = match file_content[i..len].find("{") {
        Some(i) => i,
        None => return Err("Couldn't find string \"{\"".to_string()),
    };
    let i = match file_content[i..len].find("mount") {
        Some(i) => i,
        None => return Err("Couldn't find string \"mount\"".to_string()),
    };
    let _i = match file_content[i..len].find("}") {
        Some(i) => i,
        None => return Err("Couldn't find string \"}\"".to_string()),
    };

    Ok(())
}

fn setup() -> Parameter {
    Parameter::new(APP_NAME).expect("Failed to setup parameter")
}

fn test_add(parameter_global: &Parameter) {
    parameter_global
        .add("TestParam1", None, String::new())
        .expect("Failed to add TestParam1");
    parameter_global
        .add("TestParam2", None, "TestValue2".to_string())
        .expect("Failed to add TestParam2");

    let def_content =
        std::fs::read_to_string(TEST_DEF_PATH).expect("Failed to read {TEST_DEF_PATH}");
    let conf_content =
        std::fs::read_to_string(TEST_CONF_PATH).expect("Failed to read {TEST_CONF_PATH}");

    param_in_deffile(def_content.as_str(), "TestParam1")
        .expect("Couldn't find TestParam1 in def file after add");
    param_in_deffile(def_content.as_str(), "TestParam2")
        .expect("Couldn't find TestParam2 in def file after add");

    assert!(
        conf_content.contains("TestParam1"),
        "Couldn't find TestParam1 in conf file after add"
    );
    assert!(
        conf_content.contains("TestParam2"),
        "Couldn't find TestParam2 in conf file after add"
    );
}

fn test_remove(parameter_global: &Parameter) {
    parameter_global
        .remove("TestParam1")
        .expect("Failed to remove TestParam1");
    parameter_global
        .remove("TestParam2")
        .expect("Failed to remove TestParam2");

    let def_content =
        std::fs::read_to_string(TEST_DEF_PATH).expect("Failed to read {TEST_DEF_PATH}");
    let conf_content =
        std::fs::read_to_string(TEST_CONF_PATH).expect("Failed to read {TEST_CONF_PATH}");

    assert!(
        !def_content.contains("TestParam1"),
        "Found TestParam1 in def file after remove"
    );
    assert!(
        !def_content.contains("TestParam2"),
        "Found TestParam2 in def file after remove"
    );

    assert!(
        !conf_content.contains("TestParam1"),
        "Found TestParam1 in conf file after remove"
    );
    assert!(
        !conf_content.contains("TestParam2"),
        "Found TestParam2 in conf file after remove"
    );
}

fn test_set_get(parameter_global: &Parameter) {
    parameter_global
        .set(
            "TestParam1",
            "Test Value 1 set from unittest-rs".to_string(),
            false,
        )
        .expect("Failed to set TestParam1");

    parameter_global
        .set(
            "TestParam2",
            "Test Value 2 set from unittest-rs".to_string(),
            true,
        )
        .expect("Failed to set TestParam1");

    let testvalue1: String = parameter_global
        .get("TestParam1")
        .expect("Failed to get TestParam1");
    assert_eq!(
        testvalue1,
        String::from("Test Value 1 set from unittest-rs"),
        "Expected \"Test Value 1 set from unittest-rs\", got: {testvalue1}"
    );

    let testvalue2: String = parameter_global
        .get("TestParam2")
        .expect("Failed to get TestParam2");
    assert_eq!(
        testvalue2,
        String::from("Test Value 2 set from unittest-rs"),
        "Expected \"Test Value 2 set from unittest-rs\", got: {testvalue2}"
    );
}

fn test_list(parameter_global: &Parameter) {
    let param_list = parameter_global
        .list()
        .expect("Failed to get list of parameters");

    assert!(
        param_list.contains(&"TestParameter".to_string()),
        "TestParameter is missing"
    );
    assert!(param_list.contains(&"w".to_string()), "w is missing");
}

fn fail_new() {
    let glib_err =
        Parameter::new("").expect_err("Passing empty appname to Parameter should not be allowed!");
    let param_err = glib_err
        .kind::<ParameterError>()
        .expect("The received error is not a ParameterError: {err}");
    assert_eq!(
        param_err,
        ParameterError::InvalidArg,
        "Expecting ParameterError::InvalidArg, got ParameterError::{:?}",
        param_err
    );
}

fn fail_add(parameter_global: &Parameter) {
    // No name
    let glib_err = parameter_global
        .add("", None, "initial_value".to_string())
        .expect_err("Adding parameter with empty name should not be allowed");
    let param_err = glib_err
        .kind::<ParameterError>()
        .expect("The received error is not a ParameterError: {err}");
    assert_eq!(
        param_err,
        ParameterError::InvalidArg,
        "Expecting ParameterError::InvalidArg, got ParameterError::{:?}",
        param_err
    );

    // Adding existing parameter
    let glib_err = parameter_global
        .add("TestParameter", None, "initial_value".to_string())
        .expect_err("Adding existing parameter should not be allowed");
    let param_err = glib_err
        .kind::<ParameterError>()
        .expect("The received error is not a ParameterError: {err}");
    assert_eq!(
        param_err,
        ParameterError::ParamAdded,
        "Expecting ParameterError::ParamAdded, got ParameterError::{:?}",
        param_err
    );

    // There are no files for this application, thus a file error will occur
    let pa = Parameter::new("app_name")
        .expect("Unable to create parameter for application \"app_name\"");
    let glib_err = pa
        .add("name", None, "initial_value".to_string())
        .expect_err("\"name\" should not be added since there are no files to \"app_name\"");
    let param_err = glib_err
        .kind::<ParameterError>()
        .expect("The received error is not a ParameterError: {err}");
    assert_eq!(
        param_err,
        ParameterError::FilePath,
        "Expecting ParameterError::FilePath, got ParameterError::{:?}",
        param_err
    );
}

fn fail_remove(parameter_global: &Parameter) {
    // No name
    let glib_err = parameter_global
        .remove("")
        .expect_err("Removing parameter with no name should not be allowed");
    let param_err = glib_err
        .kind::<ParameterError>()
        .expect("The received error is not a ParameterError: {err}");
    assert_eq!(
        param_err,
        ParameterError::InvalidArg,
        "Expecting ParameterError::InvalidArg, got ParameterError::{:?}",
        param_err
    );

    // Parameter does not exist
    let glib_err = parameter_global
        .remove("does_not_exist")
        .expect_err("Removing parameter that does not exist should not be allowed");
    let param_err = glib_err
        .kind::<ParameterError>()
        .expect("The received error is not a ParameterError: {err}");
    assert_eq!(
        param_err,
        ParameterError::ParamExist,
        "Expecting ParameterError::ParamExist, got ParameterError::{:?}",
        param_err
    );

    let pa = Parameter::new("app_name")
        .expect("Unable to create parameter for application \"app_name\"");
    let glib_err = pa
        .remove("name")
        .expect_err("\"name\" should not be remove since there are no files to \"app_name\"");
    let param_err = glib_err
        .kind::<ParameterError>()
        .expect("The received error is not a ParameterError: {err}");
    assert_eq!(
        param_err,
        ParameterError::FilePath,
        "Expecting ParameterError::FilePath, got ParameterError::{:?}",
        param_err
    );
}

fn fail_set(parameter_global: &Parameter) {
    // No name
    let glib_err = parameter_global
        .set("", "value".to_string(), true)
        .expect_err("Updating parameter with no name should not be allowed");
    let param_err = glib_err
        .kind::<ParameterError>()
        .expect("The received error is not a ParameterError: {err}");
    assert_eq!(
        param_err,
        ParameterError::InvalidArg,
        "Expecting ParameterError::InvalidArg, got ParameterError::{:?}",
        param_err
    );

    let glib_err = parameter_global
        .set("name", "value".to_string(), true)
        .expect_err("Updating parameter that does not exist should not be allowed");
    let param_err = glib_err
        .kind::<ParameterError>()
        .expect("The received error is not a ParameterError: {err}");
    assert_eq!(
        param_err,
        ParameterError::ParamSet,
        "Expecting ParameterError::ParamSet, got ParameterError::{:?}",
        param_err
    );
}

fn fail_get(parameter_global: &Parameter) {
    // No name
    let glib_err = parameter_global
        .get::<String>("")
        .expect_err("Getting parameter with no name should not be allowed");
    let param_err = glib_err
        .kind::<ParameterError>()
        .expect("The received error is not a ParameterError: {err}");
    assert_eq!(
        param_err,
        ParameterError::InvalidArg,
        "Expecting ParameterError::InvalidArg, got ParameterError::{:?}",
        param_err
    );

    let glib_err = parameter_global
        .get::<String>("name")
        .expect_err("Getting parameter that does not exist should not be allowed");
    let param_err = glib_err
        .kind::<ParameterError>()
        .expect("The received error is not a ParameterError: {err}");
    assert_eq!(
        param_err,
        ParameterError::ParamGet,
        "Expecting ParameterError::ParamGet, got ParameterError::{:?}",
        param_err
    );
}

#[test]
fn test_all() {
    let parameter_global = setup();

    test_add(&parameter_global);
    test_set_get(&parameter_global);
    test_remove(&parameter_global);
    test_list(&parameter_global);

    fail_new();
    fail_add(&parameter_global);
    fail_remove(&parameter_global);
    fail_set(&parameter_global);
    fail_get(&parameter_global);
}
