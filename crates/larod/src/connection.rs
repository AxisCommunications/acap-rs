use std::ffi::CStr;
use std::marker::PhantomData;
use std::os::raw::c_int;
use std::sync::mpsc;

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
        if !success || raw.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
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

        // Copy device pointers into a Vec, then free the outer C-heap array.
        let devices: Vec<Device<'_>> = (0..num_devices)
            .map(|i| {
                let dev_ptr = unsafe { *ptr.add(i) };
                Device::from_raw(dev_ptr)
            })
            .collect();

        // SAFETY: The outer array was allocated by larod (C heap). The individual
        // device pointers are borrowed from the connection and must NOT be freed.
        unsafe { libc::free(ptr as *mut libc::c_void) };

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

        // Copy model pointers into individual Model wrappers, then free the outer array.
        let models: Vec<Model> = (0..num_models)
            .map(|i| {
                let model_ptr = unsafe { *ptr.add(i) };
                Model::from_raw(model_ptr)
            })
            .collect();

        // SAFETY: The outer array was allocated by larod (C heap). Individual
        // model pointers are now owned by their Model wrappers.
        unsafe { libc::free(ptr as *mut libc::c_void) };

        Ok(models)
    }

    /// Load a model asynchronously.
    ///
    /// Returns a [`ModelFuture`] that can be waited on to get the loaded model.
    /// The connection must remain alive until the future is resolved.
    ///
    /// # Arguments
    ///
    /// Same as [`load_model`](Connection::load_model).
    pub fn load_model_async(
        &self,
        fd: c_int,
        device: &Device<'_>,
        access: larodAccess,
        name: Option<&CStr>,
        params: Option<&Map>,
    ) -> Result<ModelFuture<'_>, Error> {
        let name_ptr = name.map_or(std::ptr::null(), |n| n.as_ptr());
        let params_ptr = params.map_or(std::ptr::null(), |p| p.as_ptr());

        let (tx, rx) = mpsc::sync_channel(1);
        let user_data = Box::into_raw(Box::new(tx)) as *mut std::os::raw::c_void;

        let (success, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodLoadModelAsync,
                self.raw,
                fd,
                device.as_ptr(),
                access,
                name_ptr,
                params_ptr,
                Some(load_model_callback as unsafe extern "C" fn(_, _, _)),
                user_data,
            )
        };
        if !success {
            // Reclaim the sender to avoid leaking.
            // SAFETY: When larodLoadModelAsync returns false, the C API guarantees
            // the callback will NOT be invoked, so user_data has not been consumed.
            unsafe {
                drop(Box::from_raw(
                    user_data as *mut mpsc::SyncSender<Result<Model, Error>>,
                ));
            }
            return Err(maybe_error.unwrap_or(Error::MissingError));
        }
        debug_assert!(maybe_error.is_none());
        Ok(ModelFuture {
            rx,
            _marker: PhantomData,
        })
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

/// Handle for an asynchronous model load operation.
///
/// Call [`wait`](ModelFuture::wait) to block until the model is loaded.
/// The connection must remain alive until this future is resolved.
pub struct ModelFuture<'conn> {
    rx: mpsc::Receiver<Result<Model, Error>>,
    _marker: PhantomData<&'conn Connection>,
}

impl ModelFuture<'_> {
    /// Block until the asynchronous model load completes.
    ///
    /// Returns `Error::CallbackNeverInvoked` if the larod daemon drops the
    /// async request without invoking the callback (e.g. daemon crash).
    pub fn wait(self) -> Result<Model, Error> {
        self.rx.recv().map_err(|_| Error::CallbackNeverInvoked)?
    }
}

// SAFETY: ModelFuture only contains an mpsc::Receiver (which is Send when T: Send)
// and a PhantomData lifetime marker. It does not access the Connection at runtime.
unsafe impl Send for ModelFuture<'_> {}

/// C callback for `larodLoadModelAsync`.
///
/// # Safety
///
/// `user_data` must be a pointer created by `Box::into_raw(Box::new(SyncSender<...>))`.
unsafe extern "C" fn load_model_callback(
    model: *mut larod_sys::larodModel,
    user_data: *mut std::os::raw::c_void,
    error: *mut larod_sys::larodError,
) {
    // SAFETY: user_data was created from Box::into_raw in load_model_async.
    let tx = unsafe {
        Box::from_raw(user_data as *mut mpsc::SyncSender<Result<Model, Error>>)
    };
    let result = if !error.is_null() {
        // The error is owned by the larod daemon; copy without freeing.
        Err(Error::Larod(crate::LarodError::from_raw_borrowed(error)))
    } else if model.is_null() {
        // Defensive: larod returned success with no model and no error.
        Err(Error::NullPointer)
    } else {
        Ok(Model::from_raw(model))
    };
    let _ = tx.send(result);
}

impl Drop for Connection {
    fn drop(&mut self) {
        // larodDisconnect takes *mut *mut and nulls the pointer. Returns bool + error.
        let mut error: *mut larod_sys::larodError = std::ptr::null_mut();
        let success =
            unsafe { larod_sys::larodDisconnect(&mut self.raw, &mut error) };
        if !success {
            if !error.is_null() {
                let err = crate::LarodError::from_raw(error);
                log::error!("Failed to disconnect from larod: {err}");
            } else {
                log::error!("Failed to disconnect from larod (no error details)");
            }
        }
    }
}
