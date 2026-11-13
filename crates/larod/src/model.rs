use std::{ffi::CStr, marker::PhantomData};

pub use larod_sys::larodAccess;

use crate::{connection::Connection, tensor::Tensors, Error};

/// `#define LAROD_INVALID_MODEL_ID UINT64_MAX` in larod.h; bindgen does not emit it.
const LAROD_INVALID_MODEL_ID: u64 = u64::MAX;

/// A loaded ML model.
///
/// Models are loaded via [`Connection::load_model`] and destroyed when dropped.
/// The model handle is local to this process - dropping it does not remove the
/// model from the larod daemon (use [`Model::delete`] for that).
///
/// A `Model` borrows the [`Connection`] it was obtained through. The model's
/// device expires when that connection closes (larod.h, "Get device for a
/// model"). After the connection closed, `larodDeleteModel` through another
/// connection fails with `LAROD_ERROR_INVALID_CHIP_ID` (tested on AXIS P3268-LV, AXIS OS 12.11.77). Use
/// [`Connection::get_model`] to get a handle bound to another connection.
pub struct Model<'conn> {
    raw: *mut larod_sys::larodModel,
    _conn: PhantomData<&'conn Connection>,
}

impl<'conn> Model<'conn> {
    pub(crate) fn from_raw(
        raw: *mut larod_sys::larodModel,
        _conn: &'conn Connection,
    ) -> Result<Self, Error> {
        if raw.is_null() {
            return Err(Error::NullPointer);
        }
        Ok(Self {
            raw,
            _conn: PhantomData,
        })
    }

    /// Returns the server-assigned model ID.
    pub fn id(&self) -> Result<u64, Error> {
        // SAFETY: self.raw is a valid model handle (type invariant).
        let (id, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetModelId, self.raw as *const _) };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        if id == LAROD_INVALID_MODEL_ID {
            return Err(Error::MissingError);
        }
        Ok(id)
    }

    /// Returns the model name.
    ///
    /// NULL from `larodGetModelName` signals an error (larod.h, "Get model
    /// name"). A model loaded without a name has the empty name (tested on AXIS P3268-LV, AXIS OS 12.11.77).
    pub fn name(&self) -> Result<&CStr, Error> {
        // SAFETY: self.raw is a valid model handle (type invariant).
        let (ptr, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetModelName, self.raw as *const _) };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::MissingError));
        }
        debug_assert!(maybe_error.is_none());
        // SAFETY: ptr is non-null and "valid as long as @p model handle is
        // valid" (larod.h, "Get model name"), which outlives the borrow of &self.
        Ok(unsafe { CStr::from_ptr(ptr) })
    }

    pub fn access(&self) -> Result<larodAccess, Error> {
        // SAFETY: self.raw is a valid model handle (type invariant).
        let (access, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetModelAccess, self.raw as *const _) };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        if access == larodAccess::LAROD_ACCESS_INVALID {
            return Err(Error::MissingError);
        }
        Ok(access)
    }

    /// Returns the model size in bytes.
    pub fn size(&self) -> Result<usize, Error> {
        // SAFETY: self.raw is a valid model handle (type invariant).
        let (size, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetModelSize, self.raw as *const _) };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        Ok(size)
    }

    pub fn num_inputs(&self) -> Result<usize, Error> {
        // SAFETY: self.raw is a valid model handle (type invariant).
        let (n, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetModelNumInputs, self.raw as *const _) };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        Ok(n)
    }

    pub fn num_outputs(&self) -> Result<usize, Error> {
        // SAFETY: self.raw is a valid model handle (type invariant).
        let (n, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetModelNumOutputs, self.raw as *const _) };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        Ok(n)
    }

    /// Create tensor descriptors pre-configured with the model's input metadata.
    /// No backing memory is allocated - use `Connection::alloc_model_inputs` for that.
    pub fn create_inputs<'c>(&self, conn: &'c Connection) -> Result<Tensors<'c>, Error> {
        let mut num: usize = 0;
        // SAFETY: self.raw is a valid model handle (type invariant).
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
        Tensors::from_raw(ptr, num, conn)
    }

    /// Create tensor descriptors pre-configured with the model's output metadata.
    pub fn create_outputs<'c>(&self, conn: &'c Connection) -> Result<Tensors<'c>, Error> {
        let mut num: usize = 0;
        // SAFETY: self.raw is a valid model handle (type invariant).
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
        Tensors::from_raw(ptr, num, conn)
    }

    /// Delete this model from the larod daemon.
    ///
    /// This removes the model from the server. For private models, only the
    /// loading session can delete. For public models, any session can delete.
    /// The local handle is released either way when `self` is dropped.
    pub fn delete(self, conn: &Connection) -> Result<(), Error> {
        // SAFETY: conn.raw is a valid connection and self.raw a valid model handle.
        // larodDeleteModel does not release the handle, so Drop still runs
        // larodDestroyModel (larod.h, "Delete a loaded model").
        let (success, maybe_error) =
            unsafe { try_func!(larod_sys::larodDeleteModel, conn.raw, self.raw) };
        if success {
            debug_assert!(maybe_error.is_none());
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub(crate) fn as_ptr(&self) -> *const larod_sys::larodModel {
        self.raw as *const _
    }
}

impl std::fmt::Debug for Model<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model").field("raw", &self.raw).finish()
    }
}

impl Drop for Model<'_> {
    fn drop(&mut self) {
        // SAFETY: self.raw came from larod and is released exactly once, here.
        // larodDestroyModel nulls the pointer.
        crate::with_larod(|| unsafe { larod_sys::larodDestroyModel(&mut self.raw) })
    }
}
