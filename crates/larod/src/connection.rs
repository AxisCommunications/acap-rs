use std::ffi::CStr;
use std::os::raw::c_int;

use crate::device::Device;
use crate::model::Model;
use crate::tensor::Tensors;
use crate::{Error, Map};

pub use larod_sys::larodAccess;

/// A connection to the larod inference daemon.
///
/// This is the root object for all larod operations. Devices, models,
/// tensors, and job requests are all created through or associated with
/// a connection.
///
/// The connection is closed when dropped.
pub struct Connection {
    pub(crate) raw: *mut larod_sys::larodConnection,
}

impl Connection {
    /// Connect to the larod daemon.
    pub fn new() -> Result<Self, Error> {
        let mut raw: *mut larod_sys::larodConnection = std::ptr::null_mut();
        let (success, maybe_error) =
            unsafe { try_func!(larod_sys::larodConnect, &mut raw) };
        if !success {
            return Err(maybe_error.unwrap_or(Error::MissingError));
        }
        if raw.is_null() {
            return Err(Error::NullPointer);
        }
        debug_assert!(maybe_error.is_none());
        Ok(Self { raw })
    }

    /// Returns the number of active sessions on the larod daemon.
    pub fn num_sessions(&self) -> Result<u64, Error> {
        let mut num: u64 = 0;
        let (success, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetNumSessions, self.raw, &mut num) };
        if success {
            debug_assert!(maybe_error.is_none());
            Ok(num)
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    /// List all available inference devices.
    ///
    /// The returned devices borrow from this connection and become invalid
    /// when the connection is dropped.
    pub fn devices(&self) -> Result<Vec<Device<'_>>, Error> {
        let mut num_devices: usize = 0;
        let (ptr, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodListDevices,
                self.raw,
                &mut num_devices,
            )
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());

        // Free the outer array before wrapping (panic safety).
        let raw_ptrs: Vec<*const larod_sys::larodDevice> =
            (0..num_devices).map(|i| unsafe { *ptr.add(i) }).collect();

        // SAFETY: The outer array was allocated by larod (C heap). The individual
        // device pointers are borrowed from the connection and must NOT be freed.
        unsafe { libc::free(ptr as *mut libc::c_void) };

        let devices = raw_ptrs.into_iter().map(Device::from_raw).collect();

        Ok(devices)
    }

    /// Get a specific device by name and instance number.
    pub fn device(&self, name: &CStr, instance: u32) -> Result<Device<'_>, Error> {
        let (ptr, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodGetDevice,
                self.raw as *const _,
                name.as_ptr(),
                instance,
            )
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());
        Ok(Device::from_raw(ptr))
    }

    /// Load a model from a file descriptor.
    ///
    /// # Arguments
    ///
    /// * `fd` - File descriptor of the model file (e.g. from `File::as_raw_fd()`)
    /// * `device` - The inference device to use
    /// * `access` - Access mode (private or public)
    /// * `name` - Optional human-readable name for the model
    /// * `params` - Optional parameters map
    pub fn load_model(
        &self,
        fd: c_int,
        device: &Device<'_>,
        access: larodAccess,
        name: Option<&CStr>,
        params: Option<&Map>,
    ) -> Result<Model, Error> {
        let name_ptr = name.map_or(std::ptr::null(), |n| n.as_ptr());
        let params_ptr = params.map_or(std::ptr::null(), |p| p.as_ptr());
        let (ptr, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodLoadModel,
                self.raw,
                fd,
                device.as_ptr(),
                access,
                name_ptr,
                params_ptr,
            )
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());
        Ok(Model::from_raw(ptr))
    }

    /// Retrieve a model by its server-assigned ID.
    ///
    /// This can be used to obtain a handle to a public model loaded by another session.
    /// The returned model is an owned handle (`*mut larodModel`) that will be
    /// destroyed via `larodDestroyModel` on drop.
    pub fn get_model(&self, model_id: u64) -> Result<Model, Error> {
        let (ptr, maybe_error) = unsafe {
            try_func!(larod_sys::larodGetModel, self.raw, model_id)
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());
        Ok(Model::from_raw(ptr))
    }

    /// List all models visible to this session (own private + all public models).
    ///
    /// Each model in the returned Vec is an owned handle. The C API returns
    /// `*mut *mut larodModel` (mutable inner pointers, unlike `larodListDevices`
    /// which returns `*const` inner pointers), indicating caller ownership.
    pub fn models(&self) -> Result<Vec<Model>, Error> {
        let mut num_models: usize = 0;
        let (ptr, maybe_error) = unsafe {
            try_func!(larod_sys::larodGetModels, self.raw, &mut num_models)
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());

        // Free the outer array before wrapping (panic safety). We use libc::free
        // rather than larodDestroyModels because the inner model handles transfer
        // to Model wrappers (which call larodDestroyModel on drop).
        let raw_ptrs: Vec<*mut larod_sys::larodModel> =
            (0..num_models).map(|i| unsafe { *ptr.add(i) }).collect();

        unsafe { libc::free(ptr as *mut libc::c_void) };

        let models = raw_ptrs.into_iter().map(Model::from_raw).collect();

        Ok(models)
    }

    /// Create tensor descriptors for a model's inputs, with backing memory
    /// allocated by the larod daemon.
    pub fn alloc_model_inputs(
        &self,
        model: &Model,
        fd_prop_flags: u32,
        params: Option<&Map>,
    ) -> Result<Tensors<'_>, Error> {
        let mut num_tensors: usize = 0;
        // C API takes *mut larodMap even for read-only access.
        let params_ptr = params.map_or(std::ptr::null_mut(), |p| p.as_ptr());
        let (ptr, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodAllocModelInputs,
                self.raw,
                model.as_ptr(),
                fd_prop_flags,
                &mut num_tensors,
                params_ptr,
            )
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());
        // SAFETY: ptr is a valid tensor array returned by larod with num_tensors elements.
        Ok(unsafe { Tensors::from_raw(ptr, num_tensors, self) })
    }

    /// Create tensor descriptors for a model's outputs, with backing memory
    /// allocated by the larod daemon.
    pub fn alloc_model_outputs(
        &self,
        model: &Model,
        fd_prop_flags: u32,
        params: Option<&Map>,
    ) -> Result<Tensors<'_>, Error> {
        let mut num_tensors: usize = 0;
        let params_ptr = params.map_or(std::ptr::null_mut(), |p| p.as_ptr());
        let (ptr, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodAllocModelOutputs,
                self.raw,
                model.as_ptr(),
                fd_prop_flags,
                &mut num_tensors,
                params_ptr,
            )
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());
        // SAFETY: ptr is a valid tensor array returned by larod with num_tensors elements.
        Ok(unsafe { Tensors::from_raw(ptr, num_tensors, self) })
    }
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("raw", &self.raw)
            .finish()
    }
}

// SAFETY: We hold exclusive ownership of the raw pointer and the larod
// daemon connection does not require access from a specific thread.
unsafe impl Send for Connection {}

// Connection is intentionally !Sync — the larod daemon connection is not
// safe for concurrent access from multiple threads.

impl Drop for Connection {
    fn drop(&mut self) {
        // larodDisconnect takes *mut *mut and nulls the pointer. Returns bool + error.
        let mut error: *mut larod_sys::larodError = std::ptr::null_mut();
        let success =
            unsafe { larod_sys::larodDisconnect(&mut self.raw, &mut error) };
        // Always free the error if set, even on success, to avoid leaking.
        if !error.is_null() {
            let err = crate::LarodError::from_raw(error);
            if !success {
                log::error!("Failed to disconnect from larod: {err}");
            }
        } else if !success {
            log::error!("Failed to disconnect from larod (no error details)");
        }
    }
}
