//! A safe warpper around the larod-sys bindings to the larod C library.
//!
//! # Gotchas
//! Many of the C functions return either a bool or a pointer to some object.
//! Additionally, one of the out arguments is the pointer to the larodError
//! struct. If the normal return type is true, or not NULL in the case of a
//! pointer, the pointer to the larodError struct is expected to be NULL. This
//! represents two potentially conflicting indicators of whether the function
//! succeeded.
//!
//! Crucially, objects pointed to by returned pointers *AND* a non-NULL pointer
//! to a larodError struct need to be dealocated. That is handled appropriately
//! by copying the larodError data into a Rust LarodError struct and
//! dealocating the larodError object if it is non-NULL.
//!
//! # TODOs:
//! - [ ] [larodDisconnect](https://axiscommunications.github.io/acap-documentation/docs/api/src/api/larod/html/larod_8h.html#ab8f97b4b4d15798384ca25f32ca77bba)
//!     indicates it may fail to "kill a session." What are the implications if it fails to kill a session? Can we clear the sessions?

use core::{num, slice};
use larod_sys::*;
use std::{
    collections::HashMap,
    ffi::{c_char, c_int, CStr, CString},
    ptr::{self, slice_from_raw_parts},
    slice::from_raw_parts,
};

type Result<T> = std::result::Result<T, LarodError>;

macro_rules! try_func {
    ($func:ident $(,)?) => {{
        let mut error: *mut larodError = ptr::null_mut();
        let success = $func(&mut error);
        (success, LarodError::from(error))
    }};
    ($func:ident, $($arg:expr),+ $(,)?) => {{
        let mut error: *mut larodError = ptr::null_mut();
        let success = $func($( $arg ),+, &mut error);
        (success, LarodError::from(error))
    }}
}

// Define our error types. These may be customized for our error handling cases.
// Now we will be able to write our own errors, defer to an underlying error
// implementation, or do something in between.
#[derive(Debug, Clone, Default)]
pub struct LarodError {
    pub msg: String,
    pub code: LarodErrorCode,
}

impl LarodError {
    /// Convert from liblarod larodError to LarodError
    /// If larodError is not NULL, it must be dealocated by calling larodClearError
    fn from(e: *mut larodError) -> Self {
        if e.is_null() {
            Self::default()
        } else {
            let le = unsafe { *e };
            let msg: String = unsafe {
                CStr::from_ptr(le.msg)
                    .to_str()
                    .unwrap_or("Error message invalid")
                    .into()
            };
            let code: LarodErrorCode = le.code.into();
            // unsafe {
            //     larodClearError(&mut e);
            // }
            Self { msg, code }
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone)]
enum LarodErrorCode {
    NONE,
    JOB,
    LOAD_MODEL,
    FD,
    MODEL_NOT_FOUND,
    PERMISSION,
    CONNECTION,
    CREATE_SESSION,
    KILL_SESSION,
    INVALID_CHIP_ID,
    INVALID_ACCESS,
    DELETE_MODEL,
    TENSOR_MISMATCH,
    VERSION_MISMATCH,
    ALLOC,
    POWER_NOT_AVAILABLE,
    INVALID_TYPE,
    MAX_ERRNO,
}

impl Default for LarodErrorCode {
    fn default() -> Self {
        LarodErrorCode::NONE
    }
}

impl From<larodErrorCode> for LarodErrorCode {
    fn from(code: larodErrorCode) -> LarodErrorCode {
        match code {
            larodErrorCode_LAROD_ERROR_NONE => LarodErrorCode::NONE,
            larodErrorCode_LAROD_ERROR_JOB => LarodErrorCode::JOB,
            larodErrorCode_LAROD_ERROR_LOAD_MODEL => LarodErrorCode::LOAD_MODEL,
            larodErrorCode_LAROD_ERROR_FD => LarodErrorCode::FD,
            larodErrorCode_LAROD_ERROR_MODEL_NOT_FOUND => LarodErrorCode::MODEL_NOT_FOUND,
            larodErrorCode_LAROD_ERROR_PERMISSION => LarodErrorCode::PERMISSION,
            larodErrorCode_LAROD_ERROR_CONNECTION => LarodErrorCode::CONNECTION,
            larodErrorCode_LAROD_ERROR_CREATE_SESSION => LarodErrorCode::CREATE_SESSION,
            larodErrorCode_LAROD_ERROR_KILL_SESSION => LarodErrorCode::KILL_SESSION,
            larodErrorCode_LAROD_ERROR_INVALID_CHIP_ID => LarodErrorCode::INVALID_CHIP_ID,
            larodErrorCode_LAROD_ERROR_INVALID_ACCESS => LarodErrorCode::INVALID_ACCESS,
            larodErrorCode_LAROD_ERROR_DELETE_MODEL => LarodErrorCode::DELETE_MODEL,
            larodErrorCode_LAROD_ERROR_TENSOR_MISMATCH => LarodErrorCode::TENSOR_MISMATCH,
            larodErrorCode_LAROD_ERROR_VERSION_MISMATCH => LarodErrorCode::VERSION_MISMATCH,
            larodErrorCode_LAROD_ERROR_ALLOC => LarodErrorCode::ALLOC,
            larodErrorCode_LAROD_ERROR_POWER_NOT_AVAILABLE => LarodErrorCode::POWER_NOT_AVAILABLE,
            larodErrorCode_LAROD_ERROR_MAX_ERRNO => LarodErrorCode::MAX_ERRNO,
            _ => unreachable!(),
        }
    }
}

pub struct LarodMap {
    raw: *mut larodMap,
}

impl LarodMap {
    fn new() -> Result<Self> {
        let (map, e): (*mut larodMap, LarodError) = unsafe { try_func!(larodCreateMap) };
        if map.is_null() {
            Err(e)
        } else {
            debug_assert!(
                matches!(e.code, LarodErrorCode::NONE),
                "larodCreateMap allocated a map AND returned an error!"
            );
            Ok(Self { raw: map })
        }
    }

    fn set_string(&mut self, k: &str, v: &str) -> Result<()> {
        let Ok(key_cstr) = CString::new(k.as_bytes()) else {
            return Err(LarodError {
                msg: String::from("Could not allocate set_string key CString"),
                code: LarodErrorCode::ALLOC,
            });
        };
        let Ok(value_cstr) = CString::new(v.as_bytes()) else {
            return Err(LarodError {
                msg: String::from("Could not allocate set_string value CString"),
                code: LarodErrorCode::ALLOC,
            });
        };
        let (success, e): (bool, LarodError) = unsafe {
            try_func!(
                larodMapSetStr,
                self.raw,
                key_cstr.as_ptr(),
                value_cstr.as_ptr(),
            )
        };
        if success {
            debug_assert!(
                matches!(e.code, LarodErrorCode::NONE),
                "larodMapSetStr indicated success AND returned an error!"
            );
            Ok(())
        } else {
            Err(e)
        }
    }
    fn set_int(&mut self, k: &str, v: i64) -> Result<()> {
        let Ok(key_cstr) = CString::new(k.as_bytes()) else {
            return Err(LarodError {
                msg: String::from("Could not allocate set_string key CString"),
                code: LarodErrorCode::ALLOC,
            });
        };
        let (success, e): (bool, LarodError) =
            unsafe { try_func!(larodMapSetInt, self.raw, key_cstr.as_ptr(), v) };
        if success {
            debug_assert!(
                matches!(e.code, LarodErrorCode::NONE),
                "larodMapSetInt indicated success AND returned an error!"
            );
            Ok(())
        } else {
            Err(e)
        }
    }
    fn set_int_arr2(&mut self, k: &str, v: (i64, i64)) -> Result<()> {
        let Ok(key_cstr) = CString::new(k.as_bytes()) else {
            return Err(LarodError {
                msg: String::from("Could not allocate set_string key CString"),
                code: LarodErrorCode::ALLOC,
            });
        };
        let (success, e): (bool, LarodError) =
            unsafe { try_func!(larodMapSetIntArr2, self.raw, key_cstr.as_ptr(), v.0, v.1) };

        if success {
            debug_assert!(
                matches!(e.code, LarodErrorCode::NONE),
                "larodMapSetIntArr2 indicated success AND returned an error!"
            );
            Ok(())
        } else {
            Err(e)
        }
    }
    fn set_int_arr4(&mut self, k: &str, v: (i64, i64, i64, i64)) -> Result<()> {
        let Ok(key_cstr) = CString::new(k.as_bytes()) else {
            return Err(LarodError {
                msg: String::from("Could not allocate set_string key CString"),
                code: LarodErrorCode::ALLOC,
            });
        };
        let (success, e): (bool, LarodError) = unsafe {
            try_func!(
                larodMapSetIntArr4,
                self.raw,
                key_cstr.as_ptr(),
                v.0,
                v.1,
                v.2,
                v.3
            )
        };

        if success {
            debug_assert!(
                matches!(e.code, LarodErrorCode::NONE),
                "larodMapSetIntArr4 indicated success AND returned an error!"
            );
            Ok(())
        } else {
            Err(e)
        }
    }

    fn get_string(&self, k: &str) -> Result<String> {
        let Ok(key_cstr) = CString::new(k) else {
            return Err(LarodError {
                msg: String::from("Could not allocate set_string key CString"),
                code: LarodErrorCode::ALLOC,
            });
        };
        let (c_str_ptr, e): (*const c_char, LarodError) =
            unsafe { try_func!(larodMapGetStr, self.raw, key_cstr.as_ptr()) };
        let c_str = unsafe { CStr::from_ptr(c_str_ptr) };
        if let Ok(rs) = c_str.to_str() {
            debug_assert!(
                matches!(e.code, LarodErrorCode::NONE),
                "larodMapGetStr returned a string AND returned an error!"
            );
            Ok(String::from(rs))
        } else {
            return Err(LarodError {
                msg: String::from("Returned string is not valid UTF-8"),
                code: LarodErrorCode::INVALID_TYPE,
            });
        }
    }
    fn get_int(&self, k: &str) -> Result<i64> {
        let Ok(key_cstr) = CString::new(k) else {
            return Err(LarodError {
                msg: String::from("Could not allocate set_string key CString"),
                code: LarodErrorCode::ALLOC,
            });
        };
        let mut v: i64 = 0;
        let (success, e): (bool, LarodError) =
            unsafe { try_func!(larodMapGetInt, self.raw, key_cstr.as_ptr(), &mut v) };
        if success {
            debug_assert!(
                matches!(e.code, LarodErrorCode::NONE),
                "larodMapGetInt indicated success AND returned an error!"
            );
            Ok(v)
        } else {
            Err(e)
        }
    }
    fn get_int_arr2(&self, k: &str) -> Result<&[i64; 2]> {
        let Ok(key_cstr) = CString::new(k) else {
            return Err(LarodError {
                msg: String::from("Could not allocate set_string key CString"),
                code: LarodErrorCode::ALLOC,
            });
        };
        let (out_arr, e) = unsafe { try_func!(larodMapGetIntArr2, self.raw, key_cstr.as_ptr()) };
        if out_arr.is_null() {
            Err(e)
        } else {
            debug_assert!(
                matches!(e.code, LarodErrorCode::NONE),
                "larodMapGetInt indicated success AND returned an error!"
            );
            unsafe {
                slice::from_raw_parts(out_arr, 2)
                    .try_into()
                    .or(Err(LarodError {
                        msg: String::from("&[i64; 2] data stored in LarodMap is invalid."),
                        code: LarodErrorCode::INVALID_TYPE,
                    }))
            }
        }
    }

    fn get_int_arr4(&self, k: &str) -> Result<&[i64; 4]> {
        let Ok(key_cstr) = CString::new(k) else {
            return Err(LarodError {
                msg: String::from("Could not allocate set_string key CString"),
                code: LarodErrorCode::ALLOC,
            });
        };
        let (out_arr, e) = unsafe { try_func!(larodMapGetIntArr4, self.raw, key_cstr.as_ptr()) };
        if out_arr.is_null() {
            Err(e)
        } else {
            debug_assert!(
                matches!(e.code, LarodErrorCode::NONE),
                "larodMapGetIntArr4 indicated success AND returned an error!"
            );
            unsafe {
                slice::from_raw_parts(out_arr, 4)
                    .try_into()
                    .or(Err(LarodError {
                        msg: String::from("&[i64; 2] data stored in LarodMap is invalid."),
                        code: LarodErrorCode::INVALID_TYPE,
                    }))
            }
        }
    }
}

impl std::ops::Drop for LarodMap {
    fn drop(&mut self) {
        unsafe {
            larodDestroyMap(&mut self.raw);
        }
    }
}

#[derive(Debug)]
pub struct LarodDevice {
    ptr: *const larodDevice,
    name: String,
    id: u32,
}

impl LarodDevice {
    pub fn get_name(&self) -> &str {
        &self.name
    }

    fn larod_get_name(pointer: *const larodDevice) -> Result<String> {
        unsafe {
            let (c_char_ptr, error) = try_func!(larodGetDeviceName, pointer);
            if c_char_ptr.is_null() {
                Err(error)
            } else {
                debug_assert!(
                    matches!(error.code, LarodErrorCode::NONE),
                    "larodGetDeviceName returned an object pointer AND returned an error!"
                );
                let c_name = CStr::from_ptr(c_char_ptr);
                c_name
                    .to_str()
                    .map(|n| String::from(n))
                    .map_err(|e| LarodError {
                        msg: String::from("Returned string is not valid UTF-8"),
                        code: LarodErrorCode::INVALID_TYPE,
                    })
            }
        }
    }

    fn larod_get_instance(pointer: *const larodDevice) -> Result<u32> {
        unsafe {
            let mut instance: u32 = 0;
            let (success, error) = try_func!(larodGetDeviceInstance, pointer, &mut instance);
            if success {
                debug_assert!(
                    matches!(error.code, LarodErrorCode::NONE),
                    "larodGetDeviceInstance returned success AND returned an error!"
                );
                Ok(instance)
            } else {
                Err(error)
            }
        }
    }
}

impl TryFrom<*const larodDevice> for LarodDevice {
    type Error = LarodError;
    fn try_from(value: *const larodDevice) -> Result<Self> {
        let name = LarodDevice::larod_get_name(value)?;
        let id = LarodDevice::larod_get_instance(value)?;
        Ok(Self {
            ptr: value,
            name,
            id,
        })
    }
}

pub struct SessionBuilder {}

impl SessionBuilder {
    pub fn new() -> SessionBuilder {
        SessionBuilder {}
    }
    pub fn build(&self) -> Result<Session> {
        let mut conn: *mut larodConnection = ptr::null_mut();
        let (success, e): (bool, LarodError) = unsafe { try_func!(larodConnect, &mut conn) };
        if success {
            debug_assert!(
                matches!(e.code, LarodErrorCode::NONE),
                "larodConnect indicated success AND returned an error!"
            );
            Ok(Session {
                conn,
                model_map: HashMap::new(),
                devices: Vec::new(),
            })
        } else {
            Err(e)
        }
    }
}

pub struct Session {
    conn: *mut larodConnection,
    model_map: HashMap<String, u64>,
    devices: Vec<larodDevice>,
}

// Using a session builder might not be necessary.
// There's little to configure when starting a session.
impl Session {
    /// Constructs a new `Session`.
    ///
    /// # Panics
    ///
    /// Use `Session::builder()` if you wish to handle the failure as an `Error`
    /// instead of panicking.
    pub fn new() -> Session {
        SessionBuilder::new().build().expect("Session::new()")
    }
    pub fn builder() -> SessionBuilder {
        SessionBuilder::new()
    }
    pub fn disconnect(&mut self) -> Result<()> {
        let (success, e): (bool, LarodError) =
            unsafe { try_func!(larodDisconnect, &mut self.conn) };
        if success {
            debug_assert!(
                matches!(e.code, LarodErrorCode::NONE),
                "larodDisconnect indicated success AND returned an error!"
            );
            Ok(())
        } else {
            Err(e)
        }
    }
    pub fn num_sessions() -> Result<()> {
        Ok(())
    }
    pub fn device() -> Result<()> {
        Ok(())
    }
    pub fn list_chips() -> Result<()> {
        Ok(())
    }
    pub fn list_devices(&mut self) -> Result<Vec<LarodDevice>> {
        let mut num_devices: usize = 0;
        let (d, e) = unsafe {
            let (dev_ptr, e) = try_func!(larodListDevices, self.conn, &mut num_devices);
            if dev_ptr.is_null() {
                return Err(e);
            }
            let raw_devices = unsafe { slice::from_raw_parts(dev_ptr, num_devices) };
            let mut devices: Vec<LarodDevice> = Vec::with_capacity(num_devices);
            for (idx, raw_device) in raw_devices.into_iter().enumerate() {
                let device = LarodDevice::try_from(*raw_device);
                match device {
                    Ok(d) => devices.push(d),
                    Err(conv_error) =>
                    log::error!("Could not identify larodDevice {} of {} returned from larodListDevices.\n{}", idx, num_devices, conv_error.msg),
                }
            }
            (devices, e)
        };
        Ok(d)
    }

    // Overloaded need to check that.
    pub fn load_model(&mut self) -> Result<()> {
        // let model_fd: c_int = 0;
        // let (m, e) = unsafe {
        //     try_func!(larodLoadModel, &mut self.conn, model_fd, )
        // }
        Ok(())
    }
    pub fn get_model() -> Result<()> {
        Ok(())
    }
    pub fn get_models() -> Result<()> {
        Ok(())
    }
    pub fn delete_model() -> Result<()> {
        Ok(())
    }
    pub fn alloc_model_inputs() -> Result<()> {
        Ok(())
    }
    pub fn alloc_model_outputs() -> Result<()> {
        Ok(())
    }
    pub fn destroy_tensors() -> Result<()> {
        Ok(())
    }
    pub fn track_tensor() -> Result<()> {
        Ok(())
    }
    pub fn run_job() -> Result<()> {
        Ok(())
    }
    pub fn run_inference() -> Result<()> {
        Ok(())
    }
    pub fn chip_id() -> Result<()> {
        Ok(())
    }
    pub fn chip_type() -> Result<()> {
        Ok(())
    }
}

impl std::ops::Drop for Session {
    fn drop(&mut self) {
        unsafe {
            try_func!(larodDisconnect, &mut self.conn);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn it_creates_larod_map() {
        assert!(LarodMap::new().is_ok());
    }

    #[test]
    fn it_drops_map() {
        let map = LarodMap::new().unwrap();
        std::mem::drop(map);
    }

    #[test]
    fn larod_map_can_set_str() {
        let mut map = LarodMap::new().unwrap();
        map.set_string("test_key", "test_value").unwrap();
    }

    #[test]
    fn larod_map_can_set_int() {
        let mut map = LarodMap::new().unwrap();
        map.set_int("test_key", 10).unwrap();
    }

    #[test]
    fn larod_map_can_set_2_tuple() {
        let mut map = LarodMap::new().unwrap();
        map.set_int_arr2("test_key", (1, 2)).unwrap();
    }

    #[test]
    fn larod_map_can_set_4_tuple() {
        let mut map = LarodMap::new().unwrap();
        map.set_int_arr4("test_key", (1, 2, 3, 4)).unwrap();
    }
}
