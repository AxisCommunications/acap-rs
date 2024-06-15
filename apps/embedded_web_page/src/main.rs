//! A simple example application demonstrating how to bundle an embedded web page.
//!
//! All applications require a program, but for this example it doesn't need to do anything,
//! hence the empty main function.
fn main() {}

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    fn html_dir() -> PathBuf {
        env::current_exe().unwrap().parent().unwrap().join("html")
    }
    #[test]
    fn html_files_are_installed() {
        assert!(html_dir().join("index.html").is_file())
    }

    #[test]
    fn html_nested_files_are_installed() {
        assert!(html_dir().join("css").join("main.css").is_file())
    }
}
