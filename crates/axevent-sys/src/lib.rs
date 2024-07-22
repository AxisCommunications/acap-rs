#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

use glib_sys::*;

type gchar = ::std::os::raw::c_char;
type gdouble = ::std::os::raw::c_double;

type gint = ::std::os::raw::c_int;
type guint = ::std::os::raw::c_uint;

#[cfg(not(target_arch = "x86_64"))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(target_arch = "x86_64")]
include!("bindings.rs");
