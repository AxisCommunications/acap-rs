use std::{env, path};

fn main() {
    let mut dir = path::PathBuf::from(env::var("OUT_DIR").unwrap()).join("additional-files");
    match std::fs::create_dir(&dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }
    .unwrap();

    dir.push("html");
    match std::fs::create_dir(&dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }
    .unwrap();

    let reg = dir.join("index.html");
    let content = include_str!("templates/index.html").replace(
        "{timestamp}",
        &format!(
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ),
    );
    std::fs::write(reg, content).unwrap();
    println!("cargo:rerun-if-changed=templates/index.html");
}
