#![forbid(unsafe_code)]
//! A simple app that inspects the environment it runs in

use std::{collections::HashMap, env, io::IsTerminal};

use log::info;

fn main() {
    acap_logging::init_logger();
    info!("args: {:?}", env::args().collect::<Vec<_>>());
    info!("vars: {:#?}", env::vars().collect::<HashMap<_, _>>());
    info!("current_dir: {:?}", env::current_dir());
    info!("current_exe: {:?}", env::current_exe());
    info!("temp_dir: {:?}", env::temp_dir());
    info!("stdin is terminal: {}", std::io::stdin().is_terminal());
    info!("stdout is terminal: {}", std::io::stdout().is_terminal());
    info!("stderr is terminal: {}", std::io::stderr().is_terminal());
}

#[cfg(not(target_arch = "x86_64"))]
#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    // None of these are officially guaranteed by the ACAP framework,
    // but they seem to work in practice.

    const PACKAGE_NAME: &str = "inspect_env";
    fn package_dir() -> PathBuf {
        PathBuf::from("/usr/local/packages").join(PACKAGE_NAME)
    }

    #[test]
    fn selected_vars_are_set_as_expected() {
        assert_eq!(env::var("G_SLICE").unwrap(), "always-malloc");
    }

    #[test]
    fn current_dir_is_set_as_expected() {
        assert_eq!(env::current_dir().unwrap(), package_dir())
    }

    #[test]
    fn current_exe_is_set_as_expected() {
        assert_eq!(
            env::current_exe().unwrap(),
            package_dir().join(PACKAGE_NAME)
        )
    }
}
