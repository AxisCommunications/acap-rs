use std::marker::PhantomData;

use crate::connection::Connection;
use crate::model::Model;
use crate::tensor::Tensors;
use crate::{Error, Map};

/// A job request that binds a model to input/output tensors for inference.
///
/// The job request holds references to the connection, model, and tensor arrays.
/// All referenced objects must outlive the job request.
pub struct JobRequest<'a> {
    raw: *mut larod_sys::larodJobRequest,
    conn: &'a Connection,
    // Ensure the model and tensors outlive this job request.
    // The C library stores raw pointers to these internally.
    _model: PhantomData<&'a Model>,
    _tensors: PhantomData<&'a Tensors<'a>>,
}

impl<'a> JobRequest<'a> {
    /// Create a new job request.
    ///
    /// # Arguments
    ///
    /// * `conn` - Connection for running the job
    /// * `model` - The model to run inference with
    /// * `inputs` - Input tensor array
    /// * `outputs` - Output tensor array
    /// * `params` - Optional parameters map
    pub fn new(
        conn: &'a Connection,
        model: &'a Model,
        inputs: &'a Tensors<'_>,
        outputs: &'a Tensors<'_>,
        params: Option<&mut Map>,
    ) -> Result<Self, Error> {
        let params_ptr = params.map_or(std::ptr::null_mut(), |p| p.as_ptr());
        let (ptr, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodCreateJobRequest,
                model.as_ptr(),
                inputs.as_ptr(),
                inputs.len(),
                outputs.as_ptr(),
                outputs.len(),
                params_ptr,
            )
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());
        Ok(Self {
            raw: ptr,
            conn,
            _model: PhantomData,
            _tensors: PhantomData,
        })
    }

    /// Run inference synchronously.
    ///
    /// After completion, the output tensors' backing memory contains the
    /// inference results.
    pub fn run(&self) -> Result<(), Error> {
        let (success, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodRunJob,
                self.conn.raw,
                self.raw as *const _,
            )
        };
        if success {
            debug_assert!(maybe_error.is_none());
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    /// Set the job priority (0 = lowest, 255 = highest).
    pub fn set_priority(&mut self, priority: u8) -> Result<(), Error> {
        let (success, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodSetJobRequestPriority,
                self.raw,
                priority,
            )
        };
        if success {
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    /// Set optional parameters for this job request.
    pub fn set_params(&mut self, params: &Map) -> Result<(), Error> {
        let (success, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodSetJobRequestParams,
                self.raw,
                params.as_ptr() as *const _,
            )
        };
        if success {
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }
}

impl std::fmt::Debug for JobRequest<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobRequest")
            .field("raw", &self.raw)
            .finish()
    }
}

// SAFETY: We hold exclusive ownership of the raw pointer and the larod
// job request does not require access from a specific thread.
unsafe impl Send for JobRequest<'_> {}

impl Drop for JobRequest<'_> {
    fn drop(&mut self) {
        // larodDestroyJobRequest takes *mut *mut and nulls the pointer.
        unsafe { larod_sys::larodDestroyJobRequest(&mut self.raw) }
    }
}
