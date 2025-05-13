use bindgen::callbacks::{EnumVariantValue, ParseCallbacks};
use std::{env, path};

fn to_title_case(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    first.to_uppercase().collect::<String>()
                        + &chars.collect::<String>().to_lowercase()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

#[derive(Debug)]
struct MyParseCallbacks;

impl ParseCallbacks for MyParseCallbacks {
    fn enum_variant_name(
        &self,
        enum_name: Option<&str>,
        original_variant_name: &str,
        variant_value: EnumVariantValue,
    ) -> Option<String> {
        if let Some(enum_name) = enum_name {
            let new_variant_name = match enum_name {
                "AXSerialParity" => original_variant_name.trim_start_matches("AX_SERIAL_PARITY_"),
                "AXSerialPortmode" => match original_variant_name {
                    "AX_SERIAL_RS485_4" => "RS485_FOUR",
                    _ => original_variant_name.trim_start_matches("AX_SERIAL_"),
                },
                "AXSerialDatabits" => match variant_value {
                    EnumVariantValue::Boolean(_) => panic!(),
                    EnumVariantValue::Signed(_) => panic!(),
                    EnumVariantValue::Unsigned(n) => match n {
                        7 => "Seven",
                        8 => "Eight",
                        _ => panic!(),
                    },
                },
                "AXSerialStopbits" => match variant_value {
                    EnumVariantValue::Boolean(_) => panic!(),
                    EnumVariantValue::Signed(_) => panic!(),
                    EnumVariantValue::Unsigned(n) => match n {
                        1 => "One",
                        2 => "Two",
                        _ => panic!(),
                    },
                },
                _ => original_variant_name.trim_start_matches("AX_SERIAL_"),
            };
            Some(to_title_case(new_variant_name))
        } else {
            None
        }
    }
}
fn populated_bindings(dst: &path::PathBuf) {
    let library = pkg_config::Config::new().probe("axserialport").unwrap();
    let mut bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .rustified_enum(".*")
        .generate_comments(false)
        .allowlist_recursively(false)
        .allowlist_function("^(ax_.*)$")
        .allowlist_type("^(_?AX.*)$")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .parse_callbacks(Box::new(MyParseCallbacks))
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
