use std::ffi::CStr;

use crate::connection::Connection;
use crate::tensor::Tensors;
use crate::Error;

pub use larod_sys::larodAccess;

/// A loaded ML model.
///
/// Models are loaded via [`Connection::load_model`] and destroyed when dropped.
/// The model handle is local to this process - dropping it does not remove the
/// model from the larod daemon (use [`Model::delete`] for that).
pub struct Model {
    pub(crate) raw: *mut larod_sys::larodModel,
}

impl Model {
    pub(crate) fn from_raw(raw: *mut larod_sys::larodModel) -> Self {
        Self { raw }
    }

    /// Returns the server-assigned model ID.
    pub fn id(&self) -> Result<u64, Error> {
        let (id, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetModelId, self.raw as *const _) };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        Ok(id)
    }

    /// Returns the model name, or `None` if no name was set.
    pub fn name(&self) -> Result<&CStr, Error> {
        let (ptr, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetModelName, self.raw as *const _) };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());
        // SAFETY: ptr is non-null, points into model's internal storage.
        Ok(unsafe { CStr::from_ptr(ptr) })
    }

    pub fn access(&self) -> Result<larodAccess, Error> {
        let (access, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetModelAccess, self.raw as *const _) };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        Ok(access)
    }

    /// Returns the model size in bytes.
    pub fn size(&self) -> Result<usize, Error> {
        let (size, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetModelSize, self.raw as *const _) };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        Ok(size)
    }

    pub fn num_inputs(&self) -> Result<usize, Error> {
        let (n, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetModelNumInputs, self.raw as *const _) };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        Ok(n)
    }

    pub fn num_outputs(&self) -> Result<usize, Error> {
        let (n, maybe_error) = unsafe {
            try_func!(larod_sys::larodGetModelNumOutputs, self.raw as *const _)
        };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        Ok(n)
    }

    /// Returns the byte sizes of all input tensors.
    ///
    /// The returned Vec is copied from a C-heap-allocated array which is freed
    /// after copying.
    pub fn input_byte_sizes(&self) -> Result<Vec<usize>, Error> {
        let mut num: usize = 0;
        let (ptr, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodGetModelInputByteSizes,
                self.raw as *const _,
                &mut num,
            )
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());
        // SAFETY: ptr points to num usize values allocated by larod (C heap).
        let sizes = unsafe { std::slice::from_raw_parts(ptr, num) }.to_vec();
        unsafe { libc::free(ptr as *mut libc::c_void) };
        Ok(sizes)
    }

    /// Returns the byte sizes of all output tensors.
    pub fn output_byte_sizes(&self) -> Result<Vec<usize>, Error> {
        let mut num: usize = 0;
        let (ptr, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodGetModelOutputByteSizes,
                self.raw as *const _,
                &mut num,
            )
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());
        let sizes = unsafe { std::slice::from_raw_parts(ptr, num) }.to_vec();
        unsafe { libc::free(ptr as *mut libc::c_void) };
        Ok(sizes)
    }

    /// Create tensor descriptors pre-configured with the model's input metadata.
    /// No backing memory is allocated - use `Connection::alloc_model_inputs` for that.
    pub fn create_inputs(&self) -> Result<OwnedTensorPtrs, Error> {
        let mut num: usize = 0;
        let (ptr, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodCreateModelInputs,
                self.raw as *const _,
                &mut num,
            )
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());
        Ok(OwnedTensorPtrs { raw: ptr, len: num })
    }

    /// Create tensor descriptors pre-configured with the model's output metadata.
    pub fn create_outputs(&self) -> Result<OwnedTensorPtrs, Error> {
        let mut num: usize = 0;
        let (ptr, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodCreateModelOutputs,
                self.raw as *const _,
                &mut num,
            )
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());
        Ok(OwnedTensorPtrs { raw: ptr, len: num })
    }

    /// Delete this model from the larod daemon.
    ///
    /// This removes the model from the server. For private models, only the
    /// loading session can delete. For public models, any session can delete.
    pub fn delete(self, conn: &Connection) -> Result<(), Error> {
        let raw = self.raw;
        // Prevent Drop from calling larodDestroyModel - we handle cleanup here.
        // On success, larodDeleteModel already removes the model.
        // On failure, we still need to destroy the local handle.
        std::mem::forget(self);

        let (success, maybe_error) = unsafe {
            try_func!(larod_sys::larodDeleteModel, conn.raw, raw)
        };
        if success {
            debug_assert!(maybe_error.is_none());
            // Model deleted server-side. Destroy the local handle.
            let mut raw = raw;
            unsafe { larod_sys::larodDestroyModel(&mut raw) };
            Ok(())
        } else {
            // Delete failed - still destroy the local handle to avoid leaking.
            let mut raw = raw;
            unsafe { larod_sys::larodDestroyModel(&mut raw) };
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub(crate) fn as_ptr(&self) -> *const larod_sys::larodModel {
        self.raw as *const _
    }
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model").field("raw", &self.raw).finish()
    }
}

// SAFETY: We hold exclusive ownership of the raw pointer and the larod
// model handle does not require access from a specific thread.
unsafe impl Send for Model {}

impl Drop for Model {
    fn drop(&mut self) {
        // larodDestroyModel takes *mut *mut larodModel and nulls the pointer.
        unsafe { larod_sys::larodDestroyModel(&mut self.raw) }
    }
}

/// Intermediate tensor pointer array returned by `Model::create_inputs/outputs`.
///
/// These tensor descriptors have no backing memory yet. Call
/// [`into_tensors`](Self::into_tensors) to wrap them for proper cleanup,
/// or use `Connection::alloc_model_inputs/outputs` instead.
///
/// Dropping without calling `into_tensors()` leaks the tensor descriptors
/// (the C API requires a connection for cleanup, which this type doesn't hold).
#[must_use = "call .into_tensors() to avoid leaking tensor descriptors"]
pub struct OwnedTensorPtrs {
    pub(crate) raw: *mut *mut larod_sys::larodTensor,
    pub(crate) len: usize,
}

impl OwnedTensorPtrs {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Convert into a `Tensors` wrapper that will use `larodDestroyTensors`
    /// for cleanup (requires a connection).
    pub fn into_tensors(self, conn: &Connection) -> Tensors<'_> {
        // SAFETY: self.raw is a valid tensor array with self.len elements,
        // originally returned by larodCreateModelInputs/Outputs.
        let tensors = unsafe { Tensors::from_raw(self.raw, self.len, conn) };
        // Prevent the OwnedTensorPtrs destructor from running since
        // ownership has been transferred to Tensors.
        std::mem::forget(self);
        tensors
    }
}
