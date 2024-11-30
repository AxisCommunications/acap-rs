#![forbid(unsafe_code)]
//! A simple example application demonstrating how to use a build script to generate files
//! dynamically.
//!
//! All applications require a program, but for this example it doesn't need to do anything,
//! hence the empty main function.
fn main() {}

// TODO: Figure out how to resolve paths on host
// It is not particularly interesting for this app, but once the reverse proxy example is added it
// becomes feasible to serve the embedded web page also when testing on host.
#[cfg(not(target_arch = "x86_64"))]
#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    fn package_dir() -> PathBuf {
        env::current_exe().unwrap().parent().unwrap().to_path_buf()
    }

    fn additional_files_dir() -> PathBuf {
        package_dir()
    }
    fn lib_dir() -> PathBuf {
        package_dir().join("lib")
    }

    fn html_dir() -> PathBuf {
        package_dir().join("html")
    }

    #[test]
    fn additional_files_are_installed() {
        assert!(additional_files_dir().join("bar").is_file());
    }

    #[test]
    fn lib_files_are_installed() {
        assert!(lib_dir().join("libfoo.so").is_file())
    }

    #[test]
    fn html_files_are_installed() {
        assert!(html_dir().join("index.html").is_file())
    }
}
