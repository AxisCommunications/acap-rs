extern crate proc_macro;
use std::{env, process::Command};

use proc_macro::TokenStream;

fn commit_id() -> Option<String> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        std::str::from_utf8(&output.stdout)
            .unwrap()
            .trim()
            .to_owned(),
    )
}

fn version() -> Option<String> {
    env::var("CARGO_PKG_VERSION").ok()
}

/// Return a version for use in CLIs as a string literal
#[proc_macro]
pub fn version_with_commit_id(_item: TokenStream) -> TokenStream {
    let version = version().unwrap_or("unknown version".to_owned());
    let commit_id = commit_id().unwrap_or("unknown commit id".to_string());
    format!(r#""{version} ({commit_id})""#).parse().unwrap()
}
