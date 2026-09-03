use std::{ffi::CStr, os::raw::c_int};

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
        let (ptr, maybe_error) =
            unsafe { try_func!(larod_sys::larodListDevices, self.raw, &mut num_devices,) };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());

        let raw_ptrs: Vec<*const larod_sys::larodDevice> =
            (0..num_devices).map(|i| unsafe { *ptr.add(i) }).collect();

        let devices = raw_ptrs
            .into_iter()
            .map(Device::from_raw)
            .collect::<Result<Vec<_>, _>>()?;

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
        Device::from_raw(ptr)
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
        Model::from_raw(ptr)
    }

    /// Retrieve a model by its server-assigned ID.
    ///
    /// This can be used to obtain a handle to a public model loaded by another session.
    /// The returned model is an owned handle (`*mut larodModel`) that will be
    /// destroyed via `larodDestroyModel` on drop.
    pub fn get_model(&self, model_id: u64) -> Result<Model, Error> {
        let (ptr, maybe_error) = unsafe { try_func!(larod_sys::larodGetModel, self.raw, model_id) };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());
        Model::from_raw(ptr)
    }

    /// List all models visible to this session (own private + all public models).
    ///
    /// Each model in the returned Vec is an owned handle. The C API returns
    /// `*mut *mut larodModel` (mutable inner pointers, unlike `larodListDevices`
    /// which returns `*const` inner pointers), indicating caller ownership.
    pub fn models(&self) -> Result<Vec<Model>, Error> {
        let mut num_models: usize = 0;
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
            let raw = unsafe { *models.raw.add(i) };
            if raw.is_null() {
                return Err(Error::NullPointer);
            }
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
    ///
    pub fn alloc_model_inputs(
        &self,
        model: &Model,
        fd_requirements: FdRequirements,
        params: Option<&Map>,
    ) -> Result<Tensors<'_>, Error> {
        // SAFETY: FdRequirements can only contain documented LAROD_FD_PROP_* bits.
        unsafe { self.alloc_model_inputs_with_flags(model, fd_requirements.bits(), params) }
    }

    /// Create and allocate model input tensors using raw fd property flags.
    ///
    /// # Safety
    ///
    /// `fd_prop_flags` must be zero or a valid combination of `LAROD_FD_PROP_*`
    /// requirements supported by the model's device. Prefer
    /// [`FdRequirements::AUTO`] or another [`FdRequirements`] value containing
    /// only documented capabilities.
    pub unsafe fn alloc_model_inputs_with_flags(
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
        Tensors::from_raw(ptr, num_tensors, self)
    }

    /// Create tensor descriptors for a model's outputs, with backing memory
    /// allocated by the larod daemon.
    ///
    pub fn alloc_model_outputs(
        &self,
        model: &Model,
        fd_requirements: FdRequirements,
        params: Option<&Map>,
    ) -> Result<Tensors<'_>, Error> {
        // SAFETY: FdRequirements can only contain documented LAROD_FD_PROP_* bits.
        unsafe { self.alloc_model_outputs_with_flags(model, fd_requirements.bits(), params) }
    }

    /// Create and allocate model output tensors using raw fd property flags.
    ///
    /// # Safety
    ///
    /// `fd_prop_flags` must be zero or a valid combination of `LAROD_FD_PROP_*`
    /// requirements supported by the model's device. Prefer
    /// [`FdRequirements::AUTO`] or another [`FdRequirements`] value containing
    /// only documented capabilities.
    pub unsafe fn alloc_model_outputs_with_flags(
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
        crate::with_larod(|| unsafe { larod_sys::larodDestroyModels(&mut self.raw, self.len) })
    }
}
