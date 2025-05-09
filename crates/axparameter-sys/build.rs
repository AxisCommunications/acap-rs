use std::{env, path};

fn populated_bindings(dst: &path::PathBuf) {
    let library = pkg_config::Config::new().probe("axparameter").unwrap();
    let mut bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .generate_comments(false)
        .allowlist_recursively(false)
        .allowlist_function("^(_?ax.*)$")
        .allowlist_type("^(_?AX.*)$")
        .blocklist_type("^(_?AXParameterErrorCode.*)$")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .layout_tests(false);

    // By default, the "__mips_hard_float" macro is defined, while we want soft floats.
    // What ends up happening is that the wrong header file gets included which results
    // in clang complaining that the header does not exist. Thus, the macro has to be unset,
    // and the correct macro which includes the correct header file needs to be set instead.
    if env::var("TARGET").unwrap() == "mipsel-unknown-linux-gnu" {
        bindings = bindings.clang_args(&["-D", "__mips_soft_float=1", "-U", "__mips_hard_float"]);
    }

    for path in library.include_paths {
        bindings = bindings.clang_args(&["-I", path.to_str().unwrap()]);
    }
    bindings.generate().unwrap().write_to_file(dst).unwrap();
}

fn main() {
    let dst = path::PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");
    if env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default() != "x86_64" {
        populated_bindings(&dst);
    }
}
