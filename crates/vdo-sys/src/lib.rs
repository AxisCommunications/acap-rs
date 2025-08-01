// Fairly nasty binding for VdoResolutionSet due to "Flexible Array Member"
// https://rust-lang.github.io/rust-bindgen/using-fam.html
// https://github.com/rust-lang/rust-bindgen/issues/1680#issuecomment-554296347
#![allow(clippy::missing_safety_doc)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use glib_sys::*;
use gobject_sys::*;

// Information from glib types
// https://docs.gtk.org/glib/types.html
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

#[cfg(not(any(target_arch = "x86_64", target_os = "macos")))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(any(target_arch = "x86_64", target_os = "macos"))]
include!("bindings.rs");
