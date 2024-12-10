use std::{env, fs, path, path::Path};

use serde_json::json;

fn generate_additional(out_dir: &Path) {
    let additional = out_dir.join("additional-files");
    match fs::create_dir(&additional) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }
    .unwrap();

    let bar = additional.join("bar");
    fs::write(bar, "Bravo").unwrap()
}
fn generate_lib(out_dir: &Path) {
    let lib = out_dir.join("lib");
    match fs::create_dir(&lib) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }
    .unwrap();

    let libfoo = lib.join("libfoo.so");
    fs::write(libfoo, "Foxtrot").unwrap();
}

fn generate_license(out_dir: &Path) {
    // TODO: Consider using this to demonstrate how to customize the behavior of `cargo-about`.
    let license = out_dir.join("LICENSE");
    fs::write(license, "Third Party Software Licenses\n").unwrap();
}

fn generate_html(out_dir: &Path) {
    let html = out_dir.join("html");
    match fs::create_dir(&html) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }
    .unwrap();

    let index_in = "build/index.html";
    let index_out = html.join("index.html");
    let content = fs::read_to_string(index_in).unwrap().replace(
        "{timestamp}",
        &format!(
            "{}",
            env::var_os("SOURCE_DATE_EPOCH")
                .map(|s| s.to_str().unwrap().parse().unwrap())
                .unwrap_or_else(|| std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs())
        ),
    );
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
    println!("cargo:rerun-if-changed={index_in}");
    fs::write(index_out, content).unwrap();
}

fn generate_manifest(out_dir: &Path) {
    let manifest_out = out_dir.join("manifest.json");
    let content = json!({
        "schemaVersion": "1.2",
        "acapPackageConf": {
            "setup": {
                "appName": "using_a_build_script",
                "vendor": "Axis Communications",
                "runMode": "never",
                "version": "0.0.0"
            },
            "configuration": {
                "settingPage": "index.html"
            }
        }
    });
    fs::write(
        manifest_out,
        serde_json::to_string_pretty(&content).unwrap(),
    )
    .unwrap();
}

fn main() {
    let out_dir = path::PathBuf::from(env::var("OUT_DIR").unwrap());
    match fs::create_dir(&out_dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }
    .unwrap();
    generate_additional(&out_dir);
    generate_lib(&out_dir);
    generate_license(&out_dir);
    generate_html(&out_dir);
    generate_manifest(&out_dir);
}
