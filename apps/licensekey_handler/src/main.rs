#![no_main]
#![no_std]
//! A simple example application demonstrating how the licensekey crate may be used

extern crate alloc;
use core::ffi::CStr;
use libc::sleep;

const APP_ID: i32 = 0;
const MAJOR_VERSION: i32 = 1;
const MINOR_VERSION: i32 = 0;

fn check_license_status(app_name: &CStr) {
    match licensekey::verify(app_name, APP_ID, MAJOR_VERSION, MINOR_VERSION) {
        Ok(()) => {}
        Err(_) => {}
    }
}

#[no_mangle]
pub extern "C" fn main(_argc: isize, _argv: *const *const u8) -> isize {
    loop {
        check_license_status(c"licensekey_handler");
        unsafe {
            sleep(300);
        }
    }
}
