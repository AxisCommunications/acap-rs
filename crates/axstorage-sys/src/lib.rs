#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]
#![allow(clippy::useless_transmute)]

use glib_sys::*;

pub type gchar = ::std::os::raw::c_char;
pub type gdouble = ::std::os::raw::c_double;

pub type gint = ::std::os::raw::c_int;
pub type guint = ::std::os::raw::c_uint;

#[cfg(not(any(target_arch = "x86_64", target_os = "macos")))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(any(target_arch = "x86_64", target_os = "macos"))]
include!("bindings.rs");
