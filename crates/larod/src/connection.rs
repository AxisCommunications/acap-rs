use std::{
    ffi::CStr,
    os::fd::{AsRawFd, BorrowedFd},
};

pub use larod_sys::larodAccess;

use crate::{device::Device, model::Model, tensor::Tensors, Error, FdRequirements, Map};

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
    pub fn try_new() -> Result<Self, Error> {
        let mut raw: *mut larod_sys::larodConnection = std::ptr::null_mut();
        // SAFETY: raw is a valid out-pointer for the new connection.
        let (success, maybe_error) = unsafe { try_func!(larod_sys::larodConnect, &mut raw) };
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
        // SAFETY: self.raw is a valid connection (type invariant).
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
        // SAFETY: self.raw is a valid connection (type invariant).
        let (ptr, maybe_error) =
            unsafe { try_func!(larod_sys::larodListDevices, self.raw, &mut num_devices,) };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());

        // SAFETY: ptr is non-null and points to num_devices device pointers
        // owned by the connection (larod.h, "List available devices").
        let raw_ptrs = unsafe { std::slice::from_raw_parts(ptr, num_devices) };

        let devices = raw_ptrs
            .iter()
            .map(|&raw| Device::from_raw(raw))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(devices)
    }

    /// Get a specific device by name and instance number.
    pub fn device(&self, name: &CStr, instance: u32) -> Result<Device<'_>, Error> {
        // SAFETY: self.raw is a valid connection and name is NUL-terminated.
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
        Device::from_raw(ptr)
    }

    /// Load a model from a file descriptor.
    ///
    /// # Arguments
    ///
    /// * `fd` - File descriptor of the model file (e.g. from `File::as_fd()`).
    ///   larod also accepts a negative fd together with `params`; that is not
    ///   exposed here.
    /// * `device` - The inference device to use. larod.h does not say whether
    ///   it may come from another connection.
    /// * `access` - Access mode (private or public)
    /// * `name` - Optional human-readable name for the model. Without one,
    ///   [`Model::name`] returns the empty string (tested on AXIS P3268-LV, AXIS OS 12.11.77).
    /// * `params` - Optional parameters map
    pub fn load_model(
        &self,
        fd: BorrowedFd<'_>,
        device: &Device<'_>,
        access: larodAccess,
        name: Option<&CStr>,
        params: Option<&Map>,
    ) -> Result<Model<'_>, Error> {
        let name_ptr = name.map_or(std::ptr::null(), |n| n.as_ptr());
        let params_ptr = params.map_or(std::ptr::null(), |p| p.as_ptr());
        // SAFETY: self.raw is a valid connection, fd is open for the call,
        // device belongs to a live connection, and name and params are NULL or
        // valid for the call.
        let (ptr, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodLoadModel,
                self.raw,
                fd.as_raw_fd(),
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
        Model::from_raw(ptr, self)
    }

    /// Retrieve a model by its server-assigned ID.
    ///
    /// This can be used to obtain a handle to a public model loaded by another session.
    /// The returned model is an owned handle (`*mut larodModel`) that will be
    /// destroyed via `larodDestroyModel` on drop.
    pub fn get_model(&self, model_id: u64) -> Result<Model<'_>, Error> {
        // SAFETY: self.raw is a valid connection (type invariant).
        let (ptr, maybe_error) = unsafe { try_func!(larod_sys::larodGetModel, self.raw, model_id) };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());
        Model::from_raw(ptr, self)
    }

    /// List all models visible to this session (own private + all public models).
    ///
    /// The list from `larodGetModels` is released with `larodDestroyModels`
    /// before returning, and each returned [`Model`] is a fresh handle obtained
    /// with [`Connection::get_model`].
    pub fn models(&self) -> Result<Vec<Model<'_>>, Error> {
        let mut num_models: usize = 0;
        // SAFETY: self.raw is a valid connection (type invariant).
        let (ptr, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetModels, self.raw, &mut num_models) };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());

        let models = ModelListGuard {
            raw: ptr,
            len: num_models,
        };
        let mut ids = Vec::with_capacity(num_models);
        for i in 0..num_models {
            // SAFETY: i < num_models, the length of the array larod returned.
            let raw = unsafe { *models.raw.add(i) };
            if raw.is_null() {
                return Err(Error::NullPointer);
            }
            // SAFETY: raw is a non-null model from the list, which is alive until
            // models is dropped below.
            let (id, maybe_error) =
                unsafe { try_func!(larod_sys::larodGetModelId, raw as *const _) };
            if let Some(err) = maybe_error {
                return Err(err);
            }
            ids.push(id);
        }
        drop(models);

        ids.into_iter().map(|id| self.get_model(id)).collect()
    }

    /// Create tensor descriptors for a model's inputs, with backing memory
    /// allocated by the larod daemon.
    pub fn alloc_model_inputs(
        &self,
        model: &Model<'_>,
        fd_requirements: FdRequirements,
        params: Option<&Map>,
    ) -> Result<Tensors<'_>, Error> {
        let mut num_tensors: usize = 0;
        // The C API takes *mut larodMap. larod.h does not say whether it
        // modifies params; this assumes it does not, like larodLoadModel,
        // which takes a const map.
        let params_ptr = params.map_or(std::ptr::null_mut(), |p| p.as_ptr());
        // SAFETY: self.raw and model are valid for the duration of the call,
        // and FdRequirements can only contain documented LAROD_FD_PROP_* bits.
        let (ptr, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodAllocModelInputs,
                self.raw,
                model.as_ptr(),
                fd_requirements.bits(),
                &mut num_tensors,
                params_ptr,
            )
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());
        Tensors::from_raw(ptr, num_tensors, self)
    }

    /// Create tensor descriptors for a model's outputs, with backing memory
    /// allocated by the larod daemon.
    pub fn alloc_model_outputs(
        &self,
        model: &Model<'_>,
        fd_requirements: FdRequirements,
        params: Option<&Map>,
    ) -> Result<Tensors<'_>, Error> {
        let mut num_tensors: usize = 0;
        // The C API takes *mut larodMap. larod.h does not say whether it
        // modifies params; this assumes it does not, like larodLoadModel,
        // which takes a const map.
        let params_ptr = params.map_or(std::ptr::null_mut(), |p| p.as_ptr());
        // SAFETY: self.raw and model are valid for the duration of the call,
        // and FdRequirements can only contain documented LAROD_FD_PROP_* bits.
        let (ptr, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodAllocModelOutputs,
                self.raw,
                model.as_ptr(),
                fd_requirements.bits(),
                &mut num_tensors,
                params_ptr,
            )
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());
        Tensors::from_raw(ptr, num_tensors, self)
    }
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("raw", &self.raw)
            .finish()
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        // larodDisconnect takes *mut *mut and nulls the pointer. Returns bool + error.
        let (success, maybe_error) = crate::with_larod(|| {
            let mut error: *mut larod_sys::larodError = std::ptr::null_mut();
            // SAFETY: self.raw came from larodConnect and is released exactly once,
            // here. larodDisconnect nulls the pointer.
            let success = unsafe { larod_sys::larodDisconnect(&mut self.raw, &mut error) };
            let maybe_error = if error.is_null() {
                None
            } else {
                // SAFETY: error was allocated by larodDisconnect and the
                // process-wide larod lock is held by with_larod.
                Some(unsafe { crate::LarodError::from_raw_locked(error) })
            };
            (success, maybe_error)
        });
        if let Some(err) = maybe_error {
            if !success {
                log::error!("Failed to disconnect from larod: {err}");
            }
        } else if !success {
            log::error!("Failed to disconnect from larod (no error details)");
        }
    }
}

struct ModelListGuard {
    raw: *mut *mut larod_sys::larodModel,
    len: usize,
}

impl Drop for ModelListGuard {
    fn drop(&mut self) {
        // SAFETY: self.raw and self.len are the list and length returned by
        // larodGetModels, released exactly once, here.
        crate::with_larod(|| unsafe { larod_sys::larodDestroyModels(&mut self.raw, self.len) })
    }
}
