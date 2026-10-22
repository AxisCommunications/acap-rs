#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

pub use libc::timespec;

// SDK 1.15 declares the error pointer mutable; the ABI is the same.
pub type mdb_on_error_t =
    Option<unsafe extern "C" fn(error: *const mdb_error_t, user_data: *mut libc::c_void)>;

#[cfg(not(any(target_arch = "x86_64", target_os = "macos")))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(any(target_arch = "x86_64", target_os = "macos"))]
include!("./bindings.rs");
