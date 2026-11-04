#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]
#![allow(clippy::useless_transmute)]
// bindgen 0.73 generates raw bitfield accessors that have no `# Safety` section and that offset
// pointers by a `usize` casted to an `isize`.
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::ptr_offset_with_cast)]
// bindgen 0.73 allows `unnecessary_transmutes` on bitfield accessors, a lint the pinned toolchain
// does not know yet.
#![allow(unknown_lints)]

#[cfg(not(any(target_arch = "x86_64", target_os = "macos")))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(any(target_arch = "x86_64", target_os = "macos"))]
include!("bindings.rs");
