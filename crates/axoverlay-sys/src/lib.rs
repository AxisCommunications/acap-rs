#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

use glib_sys::*;
use libc::timeval;

type guchar = ::std::os::raw::c_uchar;
type gfloat = ::std::os::raw::c_float;
type gint = ::std::os::raw::c_int;

#[cfg(not(target_arch = "x86_64"))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(target_arch = "x86_64")]
include!("bindings.rs");
