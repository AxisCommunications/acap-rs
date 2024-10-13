use std::{env, path};

fn populated_bindings(dst: &path::PathBuf) {
    let library = pkg_config::Config::new().probe("licensekey").unwrap();
    let mut bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // None of the foreign functions construct this enum so rustifying it is safe.
        .rustified_enum("LicenseKeyState")
        .layout_tests(false);
    for path in library.include_paths {
        bindings = bindings.clang_args(&["-F", (path.to_str().unwrap())]);
    }
    bindings.generate().unwrap().write_to_file(dst).unwrap();
}

fn main() {
    let dst = path::PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");
    if env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default() != "x86_64" {
        populated_bindings(&dst);
    }
}
