//! Fairly nasty binding for VdoResolutionSet due to "Flexible Array Member"
//! https://rust-lang.github.io/rust-bindgen/using-fam.html
//! https://github.com/rust-lang/rust-bindgen/issues/1680#issuecomment-554296347
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use glib_sys::*;
use gobject_sys::*;

type gchar = core::ffi::c_char;
type guchar = core::ffi::c_uchar;
type gdouble = core::ffi::c_double;

type gint = core::ffi::c_int;
type guint = core::ffi::c_uint;
type gint16 = i16;
type guint16 = u16;
type gint32 = i32;
type guint32 = u32;
type gint64 = i64;
type guint64 = u64;

type gsize = usize;
type gssize = isize;

#[cfg(not(target_arch = "x86_64"))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(target_arch = "x86_64")]
include!("bindings.rs");
