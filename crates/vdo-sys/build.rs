use std::{env, path};

fn populated_bindings(dst: &path::PathBuf) {
    let library = pkg_config::Config::new().probe("vdo").unwrap();
    let mut bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .generate_comments(false)
        .allowlist_recursively(false)
        .allowlist_function("^(vdo.*)$")
        .allowlist_type("^(_?Vdo.*)$")
        .allowlist_var("^(VDO_ERROR.*)$")
        .blocklist_function("vdo_stream_enqueue_buffer")
        .default_enum_style(bindgen::EnumVariation::NewType {
            is_global: false,
            is_bitfield: true,
        })
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .layout_tests(false);
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
