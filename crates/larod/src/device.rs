use std::ffi::CStr;
use std::marker::PhantomData;

use crate::Error;

/// A hardware inference device (e.g. NPU, GPU, CPU).
///
/// Devices are borrowed from a [`Connection`](crate::Connection) and become
/// invalid when the connection is dropped. They cannot be individually freed.
pub struct Device<'conn> {
    raw: *const larod_sys::larodDevice,
    // Use *const to suppress auto-Sync (raw pointers are !Sync).
    _conn: PhantomData<&'conn *const ()>,
}

impl<'conn> Device<'conn> {
    pub(crate) fn from_raw(raw: *const larod_sys::larodDevice) -> Self {
        Self {
            raw,
            _conn: PhantomData,
        }
    }

    /// Returns the device name (e.g. "cpu-tflite", "artpec8-dlpu-tflite").
    pub fn name(&self) -> Result<&CStr, Error> {
        let (ptr, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetDeviceName, self.raw) };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());
        // SAFETY: ptr is non-null and points into the device's internal storage.
        // Lifetime is tied to the connection via 'conn.
        Ok(unsafe { CStr::from_ptr(ptr) })
    }

    /// Returns the device instance number.
    pub fn instance(&self) -> Result<u32, Error> {
        let mut instance: u32 = 0;
        let (success, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodGetDeviceInstance,
                self.raw,
                &mut instance,
            )
        };
        if success {
            debug_assert!(maybe_error.is_none());
            Ok(instance)
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub(crate) fn as_ptr(&self) -> *const larod_sys::larodDevice {
        self.raw
    }
}

impl std::fmt::Debug for Device<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Device").field("raw", &self.raw).finish()
    }
}

// SAFETY: Device is a borrowed read-only pointer into connection-internal storage.
// Moving the Device to another thread is safe as long as the connection outlives it
// (enforced by the 'conn lifetime).
unsafe impl Send for Device<'_> {}
