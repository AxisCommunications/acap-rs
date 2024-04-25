//! A simple example application demonstrating how the licensekey crate may be used

use log::{info, warn};
use std::ffi::{CStr, CString};
use std::os::unix::ffi::OsStrExt;

mod app_logging;

const APP_ID: i32 = 0;
const MAJOR_VERSION: i32 = 1;
const MINOR_VERSION: i32 = 0;

fn check_license_status(app_name: &CStr) {
    match licensekey::verify(app_name, APP_ID, MAJOR_VERSION, MINOR_VERSION) {
        Ok(()) => info!("License key is valid"),
        Err(e) => warn!("License key is invalid because {e}"),
    }
}

fn main() {
    app_logging::init_logger();
    let app_name = CString::new(
        std::env::current_exe()
            .unwrap()
            .file_name()
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    loop {
        check_license_status(&app_name);
        std::thread::sleep(std::time::Duration::from_secs(300));
    }
}
