#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

use glib_sys::{gboolean, gpointer, GError, GList, GQuark};

type gchar = ::std::os::raw::c_char;

#[cfg(not(any(target_arch = "x86_64", target_os = "macos")))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(any(target_arch = "x86_64", target_os = "macos"))]
include!("bindings.rs");

// Due to an issue specified in https://github.com/rust-lang/rust-bindgen/issues/1966
// where bindgen will use std::underlying_type_t<Enum> to determine the type of enum,
// we have to setup bindings for AXParameterErrorCode ourselves.
//
// ax_parameter_error_quark will be setup by rust-bindgen
pub type AXParameterErrorCode = ::std::os::raw::c_int;
pub const AX_PARAMETER_INVALID_ARG_ERROR: AXParameterErrorCode = 0;
pub const AX_PARAMETER_FILE_FD_ERROR: AXParameterErrorCode = 1;
pub const AX_PARAMETER_FILE_LOCK_ERROR: AXParameterErrorCode = 2;
pub const AX_PARAMETER_FILE_OPEN_ERROR: AXParameterErrorCode = 3;
pub const AX_PARAMETER_FILE_FORMAT_ERROR: AXParameterErrorCode = 4;
pub const AX_PARAMETER_FILE_CREATE_ERROR: AXParameterErrorCode = 5;
pub const AX_PARAMETER_FILE_WRITE_ERROR: AXParameterErrorCode = 6;
pub const AX_PARAMETER_FILE_LINK_ERROR: AXParameterErrorCode = 7;
pub const AX_PARAMETER_PARAM_LIST_ERROR: AXParameterErrorCode = 8;
pub const AX_PARAMETER_PARAM_GET_ERROR: AXParameterErrorCode = 9;
pub const AX_PARAMETER_PARAM_PATH_ERROR: AXParameterErrorCode = 10;
pub const AX_PARAMETER_PARAM_SYNC_ERROR: AXParameterErrorCode = 11;
pub const AX_PARAMETER_PARAM_EXIST_ERROR: AXParameterErrorCode = 12;
pub const AX_PARAMETER_PARAM_ADDED_ERROR: AXParameterErrorCode = 13;
pub const AX_PARAMETER_PARAM_READ_GROUP_ERROR: AXParameterErrorCode = 14;
pub const AX_PARAMETER_PARAM_SET_ERROR: AXParameterErrorCode = 15;
pub const AX_PARAMETER_DBUS_SETUP_ERROR: AXParameterErrorCode = 16;
pub const AX_PARAMETER_FILE_UNLINK_ERROR: AXParameterErrorCode = 17;
pub const AX_PARAMETER_FILE_PATH_ERROR: AXParameterErrorCode = 18;
pub const AX_PARAMETER_FILE_SYNC_ERROR: AXParameterErrorCode = 19;
pub const AX_PARAMETER_FILE_RENAME_ERROR: AXParameterErrorCode = 20;
