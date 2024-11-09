//! A safe warpper around the larod-sys bindings to the larod C library.
//!
//! # Gotchas
//! Many of the C functions return either a bool or a pointer to some object.
//! Additionally, one of the out arguments is a pointer to a larodError
//! object. If the normal return type is true, or not NULL in the case of a
//! pointer, the pointer to the larodError struct is expected to be NULL. This
//! represents two potentially conflicting indicators of whether the function
//! succeeded.
//!
//! Crucially, objects pointed to by returned pointers *AND* a non-NULL pointer
//! to a larodError struct need to be dealocated. That is handled appropriately
//! by constructing the LarodError struct if the larodError pointer is non-NULL
//! and the impl Drop for LarodError will dealocate the object appropriately.
//!
//! Example
//! ```rust
//! use larod::Session;
//! let session = Session::new();
//! let devices = session.devices();
//! ```
//!
//! # TODOs:
//! - [ ] [larodDisconnect](https://axiscommunications.github.io/acap-documentation/docs/api/src/api/larod/html/larod_8h.html#ab8f97b4b4d15798384ca25f32ca77bba)
//!     indicates it may fail to "kill a session." What are the implications if it fails to kill a session? Can we clear the sessions?

use core::slice;
use larod_sys::*;
use std::{
    collections::HashMap,
    ffi::{c_char, CStr, CString},
    marker::PhantomData,
    ptr::{self},
};

type Result<T> = std::result::Result<T, Error>;

macro_rules! try_func {
    ($func:ident $(,)?) => {{
        let mut error: *mut larodError = ptr::null_mut();
        let success = $func(&mut error);
        if error.is_null() {
            (success, None)
        } else {
            (success, Some(Error::LarodError(LarodError{inner: error})))
        }
    }};
    ($func:ident, $($arg:expr),+ $(,)?) => {{
        let mut error: *mut larodError = ptr::null_mut();
        let success = $func($( $arg ),+, &mut error);
        if error.is_null() {
            (success, None)
        } else {
            (success, Some(Error::LarodError(LarodError{inner: error})))
        }

    }}
}

// Most larod functions require a `NULL`` pointer to a larodError AND may
// produce either a `NULL`` output pointer or `false` if an error occurs. This
// results in two potential indicators of whether the function succeeded. If we
// get a`true` output, we expect the larodError to be a pointer to `NULL` still.
// In the possibly rare event that a function succeeds but the larodError
// pointer is not `NULL`, we need to deallocate that memory by calling
// `larodClearError`. The `try_func` macro always checks to see if the
// larodError pointer is `NULL` and return a `LarodError` if not. Doing so will
// call `larodClearError` when it is ultimately dropped.
#[derive(Debug)]
pub struct LarodError {
    inner: *mut larodError,
}

impl LarodError {
    pub fn msg(&self) -> Result<String> {
        if self.inner.is_null() {
            Err(Error::NullLarodPointer)
        } else {
            let msg_slice = unsafe { CStr::from_ptr((*self.inner).msg).to_str() };
            match msg_slice {
                Ok(m) => Ok(m.into()),
                Err(e) => {
                    log::error!("larodError.msg contained invalid UTF-8: {:?}", e);
                    Err(Error::InvalidLarodMessage)
                }
            }
        }
    }

    pub fn code(&self) -> larodErrorCode {
        unsafe { (*self.inner).code }
    }
}

impl Drop for LarodError {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { larodClearError(&mut self.inner) }
        }
    }
}

#[derive(Debug)]
pub enum Error {
    LarodError(LarodError),
    NullLarodPointer,
    InvalidLarodMessage,
    PointerToInvalidData,
    CStringAllocation,
    MissingLarodError,
}

// impl LarodError {
//     /// Convert from liblarod larodError to LarodError
//     /// If larodError is not NULL, it must be dealocated by calling larodClearError
//     fn from(e: *mut larodError) -> Self {
//         if e.is_null() {
//             Self::default()
//         } else {
//             let le = unsafe { *e };
//             let msg: String = unsafe {
//                 CStr::from_ptr(le.msg)
//                     .to_str()
//                     .unwrap_or("Error message invalid")
//                     .into()
//             };
//             let code: LarodErrorCode = le.code.into();
//             // unsafe {
//             //     larodClearError(&mut e);
//             // }
//             Self { msg, code }
//         }
//     }
// }

/// A type representing a larodMap.
pub struct LarodMap {
    raw: *mut larodMap,
}

impl LarodMap {
    /// Create a new larodMap object
    pub fn new() -> Result<Self> {
        let (map, maybe_error): (*mut larodMap, Option<Error>) =
            unsafe { try_func!(larodCreateMap) };
        if !map.is_null() {
            debug_assert!(
                maybe_error.is_none(),
                "larodCreateMap allocated a map AND returned an error!"
            );
            Ok(Self { raw: map })
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    /// Add a string to a larodMap object.
    /// Example
    /// ```rust
    /// use larod::LarodMap;
    ///
    /// let map = LarodMap::new().expect("Error creating map");
    /// map.set_string("key", "value").expect("Error setting string value for larodMap");
    /// ```
    pub fn set_string(&mut self, k: &str, v: &str) -> Result<()> {
        let Ok(key_cstr) = CString::new(k.as_bytes()) else {
            return Err(Error::CStringAllocation);
        };
        let Ok(value_cstr) = CString::new(v.as_bytes()) else {
            return Err(Error::CStringAllocation);
        };
        let (success, maybe_error): (bool, Option<Error>) = unsafe {
            try_func!(
                larodMapSetStr,
                self.raw,
                key_cstr.as_ptr(),
                value_cstr.as_ptr(),
            )
        };
        if success {
            debug_assert!(
                maybe_error.is_none(),
                "larodMapSetStr indicated success AND returned an error!"
            );
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    /// Add an integer to a larodMap object.
    /// Example
    /// ```rust
    /// use larod::LarodMap;
    ///
    /// let map = LarodMap::new().expect("Error creating map");
    /// map.set_int("key", 45).expect("Error setting integer value for larodMap");
    /// ```
    pub fn set_int(&mut self, k: &str, v: i64) -> Result<()> {
        let Ok(key_cstr) = CString::new(k.as_bytes()) else {
            return Err(Error::CStringAllocation);
        };
        let (success, maybe_error): (bool, Option<Error>) =
            unsafe { try_func!(larodMapSetInt, self.raw, key_cstr.as_ptr(), v) };
        if success {
            debug_assert!(
                maybe_error.is_none(),
                "larodMapSetInt indicated success AND returned an error!"
            );
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    /// Add an integer array of two items to a larodMap object.
    /// Example
    /// ```rust
    /// use larod::LarodMap;
    ///
    /// let map = LarodMap::new().expect("Error creating map");
    /// map.set_int_arr2("key", (45, 64)).expect("Error setting integer array for larodMap");
    /// ```
    pub fn set_int_arr2(&mut self, k: &str, v: (i64, i64)) -> Result<()> {
        let Ok(key_cstr) = CString::new(k.as_bytes()) else {
            return Err(Error::CStringAllocation);
        };
        let (success, maybe_error): (bool, Option<Error>) =
            unsafe { try_func!(larodMapSetIntArr2, self.raw, key_cstr.as_ptr(), v.0, v.1) };

        if success {
            debug_assert!(
                maybe_error.is_none(),
                "larodMapSetIntArr2 indicated success AND returned an error!"
            );
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    /// Add an integer array of 4 items to a larodMap object.
    /// Example
    /// ```rust
    /// use larod::LarodMap;
    ///
    /// let map = LarodMap::new().expect("Error creating map");
    /// map.set_int_arr4("key", (45, 64, 36, 23)).expect("Error setting integer array for larodMap");
    /// ```
    pub fn set_int_arr4(&mut self, k: &str, v: (i64, i64, i64, i64)) -> Result<()> {
        let Ok(key_cstr) = CString::new(k.as_bytes()) else {
            return Err(Error::CStringAllocation);
        };
        let (success, maybe_error): (bool, Option<Error>) = unsafe {
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
                maybe_error.is_none(),
                "larodMapSetIntArr4 indicated success AND returned an error!"
            );
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    /// Get a string from a larodMap object.
    /// Example
    /// ```rust
    /// use larod::LarodMap;
    ///
    /// let map = LarodMap::new().expect("Error creating map");
    /// map.set_string("key", "value").expect("Error setting string value for larodMap");
    /// let returned_string = map.get_string("key").expect("Unable to return value for \"key\"");
    /// ```
    pub fn get_string(&self, k: &str) -> Result<String> {
        let Ok(key_cstr) = CString::new(k) else {
            return Err(Error::CStringAllocation);
        };
        let (c_str_ptr, maybe_error): (*const c_char, Option<Error>) =
            unsafe { try_func!(larodMapGetStr, self.raw, key_cstr.as_ptr()) };
        let c_str = unsafe { CStr::from_ptr(c_str_ptr) };
        if let Ok(rs) = c_str.to_str() {
            debug_assert!(
                maybe_error.is_none(),
                "larodMapGetStr returned a string AND returned an error!"
            );
            Ok(String::from(rs))
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    /// Get an integer array of 4 items from a larodMap object.
    /// Example
    /// ```rust
    /// use larod::LarodMap;
    ///
    /// let map = LarodMap::new().expect("Error creating map");
    /// map.set_int("key", 45).expect("Error setting integer array for larodMap");
    /// let value = map.get_int("key").expect("Unable to get array values for \"key\"");
    /// ```
    pub fn get_int(&self, k: &str) -> Result<i64> {
        let Ok(key_cstr) = CString::new(k) else {
            return Err(Error::CStringAllocation);
        };
        let mut v: i64 = 0;
        let (success, maybe_error): (bool, Option<Error>) =
            unsafe { try_func!(larodMapGetInt, self.raw, key_cstr.as_ptr(), &mut v) };
        if success {
            debug_assert!(
                maybe_error.is_none(),
                "larodMapGetInt indicated success AND returned an error!"
            );
            Ok(v)
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    /// Get an integer array of 4 items from a larodMap object.
    /// Example
    /// ```rust
    /// use larod::LarodMap;
    ///
    /// let map = LarodMap::new().expect("Error creating map");
    /// map.set_int_arr2("key", (45, 64)).expect("Error setting integer array for larodMap");
    /// let returned_array = map.get_int_arr2("key").expect("Unable to get array values for \"key\"");
    /// ```
    pub fn get_int_arr2(&self, k: &str) -> Result<&[i64; 2]> {
        let Ok(key_cstr) = CString::new(k) else {
            return Err(Error::CStringAllocation);
        };
        let (out_arr, maybe_error) =
            unsafe { try_func!(larodMapGetIntArr2, self.raw, key_cstr.as_ptr()) };
        if !out_arr.is_null() {
            debug_assert!(
                maybe_error.is_none(),
                "larodMapGetInt indicated success AND returned an error!"
            );
            unsafe {
                slice::from_raw_parts(out_arr, 2)
                    .try_into()
                    .or(Err(Error::PointerToInvalidData))
            }
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    /// Get an integer array of 4 items from a larodMap object.
    /// Example
    /// ```rust
    /// use larod::LarodMap;
    ///
    /// let map = LarodMap::new().expect("Error creating map");
    /// map.set_int_arr4("key", (45, 64, 36, 23)).expect("Error setting integer array for larodMap");
    /// let returned_array = map.get_int_arr4("key").expect("Unable to get array values for \"key\"");
    /// ```
    pub fn get_int_arr4(&self, k: &str) -> Result<&[i64; 4]> {
        let Ok(key_cstr) = CString::new(k) else {
            return Err(Error::CStringAllocation);
        };
        let (out_arr, maybe_error) =
            unsafe { try_func!(larodMapGetIntArr4, self.raw, key_cstr.as_ptr()) };
        if !out_arr.is_null() {
            debug_assert!(
                maybe_error.is_none(),
                "larodMapGetIntArr4 indicated success AND returned an error!"
            );
            unsafe {
                slice::from_raw_parts(out_arr, 4)
                    .try_into()
                    .or(Err(Error::PointerToInvalidData))
            }
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
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

/// A type representing a larodDevice.
/// The lifetime of LarodDevice is explicitly tied to the lifetime of a
/// [Session]. So using a LarodDevice after the Session it was acquired from
/// will cause compilation to fail.
/// ```compile_fail
/// use larod::Session;
/// let sess = Session::new();
/// let first_device = sess
///     .devices()
///     .expect("unable to get devices")
///     .pop()
///     .expect("empty devices list!");
/// drop(sess);
/// println!("{:?}", first_device.name());
/// ```
#[derive(Debug)]
pub struct LarodDevice<'a> {
    // The caller does not get ownership of the returned pointer and must not
    // attempt to free it. The lifetime of the memory pointed to expires when
    // conn closes.
    ptr: *const larodDevice,
    phantom: PhantomData<&'a Session<'a>>,
}

impl<'a> LarodDevice<'a> {
    /// Get the name of a larodDevice.
    pub fn name(&self) -> Result<String> {
        unsafe {
            let (c_char_ptr, maybe_error) = try_func!(larodGetDeviceName, self.ptr);
            if !c_char_ptr.is_null() {
                debug_assert!(
                    maybe_error.is_none(),
                    "larodGetDeviceName returned an object pointer AND returned an error!"
                );
                let c_name = CStr::from_ptr(c_char_ptr);
                c_name
                    .to_str()
                    .map(String::from)
                    .map_err(|_e| Error::InvalidLarodMessage)
            } else {
                Err(maybe_error.unwrap_or(Error::MissingLarodError))
            }
        }
    }

    /// Get the instance of a larodDevice.
    /// From the larod documentation
    /// > *In case there are multiple identical devices that are available in the service, they are distinguished by an instance number, with the first instance starting from zero.*
    pub fn instance(&self) -> Result<u32> {
        unsafe {
            let mut instance: u32 = 0;
            let (success, maybe_error) = try_func!(larodGetDeviceInstance, self.ptr, &mut instance);
            if success {
                debug_assert!(
                    maybe_error.is_none(),
                    "larodGetDeviceInstance returned success AND returned an error!"
                );
                Ok(instance)
            } else {
                Err(maybe_error.unwrap_or(Error::MissingLarodError))
            }
        }
    }
}

pub struct SessionBuilder {}

impl SessionBuilder {
    pub fn new() -> SessionBuilder {
        SessionBuilder {}
    }
    pub fn build(&self) -> Result<Session<'static>> {
        let mut conn: *mut larodConnection = ptr::null_mut();
        let (success, maybe_error): (bool, Option<Error>) =
            unsafe { try_func!(larodConnect, &mut conn) };
        if success {
            debug_assert!(
                maybe_error.is_none(),
                "larodConnect indicated success AND returned an error!"
            );
            Ok(Session {
                conn,
                model_map: HashMap::new(),
                phantom: PhantomData,
            })
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }
}

impl Default for SessionBuilder {
    fn default() -> Self {
        SessionBuilder::new()
    }
}

pub struct Session<'a> {
    conn: *mut larodConnection,
    model_map: HashMap<String, u64>,
    phantom: PhantomData<&'a larodConnection>,
}

// Using a session builder might not be necessary.
// There's little to configure when starting a session.
impl<'a> Session<'a> {
    /// Constructs a new `Session`.
    ///
    /// # Panics
    ///
    /// Use `Session::builder()` if you wish to handle the failure as an `Error`
    /// instead of panicking.
    pub fn new() -> Session<'a> {
        SessionBuilder::new().build().expect("Session::new()")
    }
    pub fn builder() -> SessionBuilder {
        SessionBuilder::new()
    }
    pub fn disconnect(&mut self) -> Result<()> {
        let (success, maybe_error): (bool, Option<Error>) =
            unsafe { try_func!(larodDisconnect, &mut self.conn) };
        if success {
            debug_assert!(
                maybe_error.is_none(),
                "larodDisconnect indicated success AND returned an error!"
            );
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }
    pub fn num_sessions() -> Result<()> {
        Ok(())
    }

    /// Returns a reference to an available device
    pub fn device(&self, name: &str, instance: u32) -> Result<LarodDevice> {
        let Ok(name_cstr) = CString::new(name) else {
            return Err(Error::CStringAllocation);
        };
        let (device_ptr, maybe_error) =
            unsafe { try_func!(larodGetDevice, self.conn, name_cstr.as_ptr(), instance) };
        if !device_ptr.is_null() {
            debug_assert!(
                maybe_error.is_none(),
                "larodGetDevice indicated success AND returned an error!"
            );
            Ok(LarodDevice {
                ptr: device_ptr,
                phantom: PhantomData,
            })
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    pub fn list_chips() -> Result<()> {
        Ok(())
    }

    /// Get a reference to a HashMap of name LarodDevice pairs.
    pub fn devices(&self) -> Result<Vec<LarodDevice>> {
        let mut num_devices: usize = 0;
        let (dev_ptr, maybe_error) =
            unsafe { try_func!(larodListDevices, self.conn, &mut num_devices) };
        if dev_ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::MissingLarodError));
        }
        let raw_devices =
            unsafe { slice::from_raw_parts::<'a, *const larodDevice>(dev_ptr, num_devices) };

        let devices: Vec<LarodDevice> = raw_devices
            .iter()
            .map(|ptr| LarodDevice {
                ptr: *ptr,
                phantom: PhantomData,
            })
            .collect();

        Ok(devices)
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

impl<'a> Default for Session<'a> {
    fn default() -> Self {
        SessionBuilder::default()
            .build()
            .expect("Session::default()")
    }
}

impl<'a> std::ops::Drop for Session<'a> {
    fn drop(&mut self) {
        unsafe {
            try_func!(larodDisconnect, &mut self.conn);
        }
    }
}

#[cfg(all(test, target_arch = "aarch64", feature = "device-tests"))]
mod tests {
    use super::*;

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
    fn larod_map_can_get_str() {
        let mut map = LarodMap::new().unwrap();
        map.set_string("test_key", "this_value").unwrap();
        let s = map.get_string("test_key").unwrap();
        assert_eq!(s, String::from("this_value"));
    }

    #[test]
    fn larod_map_can_set_int() {
        let mut map = LarodMap::new().unwrap();
        map.set_int("test_key", 10).unwrap();
    }

    #[test]
    fn larod_map_can_get_int() {
        let mut map = LarodMap::new().unwrap();
        map.set_int("test_key", 9).unwrap();
        let i = map.get_int("test_key").unwrap();
        assert_eq!(i, 9);
    }

    #[test]
    fn larod_map_can_set_2_tuple() {
        let mut map = LarodMap::new().unwrap();
        map.set_int_arr2("test_key", (1, 2)).unwrap();
    }
    #[test]
    fn larod_map_can_get_2_tuple() {
        let mut map = LarodMap::new().unwrap();
        map.set_int_arr2("test_key", (5, 6)).unwrap();
        let arr = map.get_int_arr2("test_key").unwrap();
        assert_eq!(arr[0], 5);
        assert_eq!(arr[1], 6);
    }

    #[test]
    fn larod_map_can_set_4_tuple() {
        let mut map = LarodMap::new().unwrap();
        map.set_int_arr4("test_key", (1, 2, 3, 4)).unwrap();
    }

    #[test]
    fn larod_map_can_get_4_tuple() {
        let mut map = LarodMap::new().unwrap();
        map.set_int_arr4("test_key", (1, 2, 3, 4)).unwrap();
        let arr = map.get_int_arr4("test_key").unwrap();
        assert_eq!(arr[0], 1);
        assert_eq!(arr[1], 2);
        assert_eq!(arr[2], 3);
        assert_eq!(arr[3], 4);
    }

    #[test]
    fn it_establishes_session() {
        let sess = Session::new();
    }

    #[test]
    fn it_lists_devices() {
        let sess = Session::new();
        let devices = sess.get_devices().unwrap();
        for device in devices {
            println!(
                "device: {}, id: {}, addr: {:?}",
                device.get_name().unwrap(),
                device.get_instance().unwrap(),
                unsafe { std::ptr::addr_of!(*device.ptr) },
            );
        }
    }
}
