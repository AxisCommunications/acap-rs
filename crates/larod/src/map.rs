use std::ffi::{c_char, CStr};

use crate::Error;

/// A key-value parameter map for larod operations.
///
/// Used to pass optional parameters to model loading and tensor allocation.
/// Supports string, integer, and integer array values.
pub struct Map {
    raw: *mut larod_sys::larodMap,
}

impl Map {
    pub fn try_new() -> Result<Self, Error> {
        // SAFETY: larodCreateMap takes no arguments besides the error pointer.
        let (map, maybe_error) = unsafe { try_func!(larod_sys::larodCreateMap) };
        if map.is_null() {
            Err(maybe_error.unwrap_or(Error::NullPointer))
        } else {
            debug_assert!(maybe_error.is_none());
            Ok(Self { raw: map })
        }
    }

    pub fn set_str(&mut self, key: &CStr, value: &CStr) -> Result<(), Error> {
        // SAFETY: self.raw is a valid map, and key and value are NUL-terminated.
        // larod copies both strings.
        let (success, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodMapSetStr,
                self.raw,
                key.as_ptr(),
                value.as_ptr(),
            )
        };
        if success {
            debug_assert!(maybe_error.is_none());
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    /// Returns the string value for `key`.
    ///
    /// A missing key, or a key whose value is not a string, is an error:
    /// `larodMapGetStr` returns NULL only on error (larod.h, "Get string type
    /// value tied to given key"). The returned reference borrows from the map's
    /// internal storage.
    pub fn get_str(&self, key: &CStr) -> Result<&CStr, Error> {
        // larodMapGetStr takes *mut larodMap (not *const) per the C API.
        // SAFETY: self.raw is a valid map and key is NUL-terminated. larodMapGetStr
        // takes *mut larodMap even though it does not modify the map.
        let (ptr, maybe_error): (*const c_char, _) =
            unsafe { try_func!(larod_sys::larodMapGetStr, self.raw, key.as_ptr()) };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::MissingError));
        }
        debug_assert!(maybe_error.is_none());
        // SAFETY: ptr is non-null and points into the map's internal storage.
        // The borrow from &self ensures the map (and thus the string) outlives
        // the returned reference.
        Ok(unsafe { CStr::from_ptr(ptr) })
    }

    pub fn set_int(&mut self, key: &CStr, value: i64) -> Result<(), Error> {
        // SAFETY: self.raw is a valid map and key is NUL-terminated.
        let (success, maybe_error) =
            unsafe { try_func!(larod_sys::larodMapSetInt, self.raw, key.as_ptr(), value) };
        if success {
            debug_assert!(maybe_error.is_none());
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub fn get_int(&self, key: &CStr) -> Result<i64, Error> {
        let mut value: i64 = 0;
        // SAFETY: self.raw is a valid map and key is NUL-terminated.
        let (success, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodMapGetInt,
                self.raw,
                key.as_ptr(),
                &mut value,
            )
        };
        if success {
            debug_assert!(maybe_error.is_none());
            Ok(value)
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub fn set_int_arr2(&mut self, key: &CStr, v0: i64, v1: i64) -> Result<(), Error> {
        // SAFETY: self.raw is a valid map and key is NUL-terminated.
        let (success, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodMapSetIntArr2,
                self.raw,
                key.as_ptr(),
                v0,
                v1,
            )
        };
        if success {
            debug_assert!(maybe_error.is_none());
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    /// Returns a 2-element array, or an error if the key doesn't exist.
    pub fn get_int_arr2(&self, key: &CStr) -> Result<[i64; 2], Error> {
        // SAFETY: self.raw is a valid map and key is NUL-terminated.
        let (ptr, maybe_error) =
            unsafe { try_func!(larod_sys::larodMapGetIntArr2, self.raw, key.as_ptr()) };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        if ptr.is_null() {
            return Err(Error::NullPointer);
        }
        // SAFETY: ptr points to at least 2 i64 values in the map's internal storage.
        Ok(unsafe { [*ptr, *ptr.add(1)] })
    }

    pub fn set_int_arr4(
        &mut self,
        key: &CStr,
        v0: i64,
        v1: i64,
        v2: i64,
        v3: i64,
    ) -> Result<(), Error> {
        // SAFETY: self.raw is a valid map and key is NUL-terminated.
        let (success, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodMapSetIntArr4,
                self.raw,
                key.as_ptr(),
                v0,
                v1,
                v2,
                v3,
            )
        };
        if success {
            debug_assert!(maybe_error.is_none());
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    /// Returns a 4-element array, or an error if the key doesn't exist.
    pub fn get_int_arr4(&self, key: &CStr) -> Result<[i64; 4], Error> {
        // SAFETY: self.raw is a valid map and key is NUL-terminated.
        let (ptr, maybe_error) =
            unsafe { try_func!(larod_sys::larodMapGetIntArr4, self.raw, key.as_ptr()) };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        if ptr.is_null() {
            return Err(Error::NullPointer);
        }
        // SAFETY: ptr points to at least 4 i64 values in the map's internal storage.
        Ok(unsafe { [*ptr, *ptr.add(1), *ptr.add(2), *ptr.add(3)] })
    }

    // *mut because the C API takes *mut even for read-only operations.
    pub(crate) fn as_ptr(&self) -> *mut larod_sys::larodMap {
        self.raw
    }
}

impl std::fmt::Debug for Map {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Map").field("raw", &self.raw).finish()
    }
}

impl Drop for Map {
    fn drop(&mut self) {
        // SAFETY: self.raw came from larodCreateMap and is released exactly once, here.
        // larodDestroyMap nulls the pointer.
        crate::with_larod(|| unsafe { larod_sys::larodDestroyMap(&mut self.raw) })
    }
}
