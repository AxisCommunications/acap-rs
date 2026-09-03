use std::marker::PhantomData;

use crate::{connection::Connection, model::Model, tensor::Tensors, Error, Map};

/// A job request that binds a model to input/output tensors for inference.
///
/// The job request holds references to the connection, model, and tensor arrays.
/// All referenced objects must outlive the job request.
pub struct JobRequest<'a> {
    raw: *mut larod_sys::larodJobRequest,
    conn_raw: *mut larod_sys::larodConnection,
    // 'a is constrained by the constructor to the shortest of conn, model,
    // inputs, and outputs.
    _conn: PhantomData<&'a Connection>,
    _model: PhantomData<&'a Model>,
    _tensors: PhantomData<&'a ()>,
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
    pub fn try_new(
        conn: &'a Connection,
        model: &'a Model,
        inputs: &'a Tensors<'_>,
        outputs: &'a Tensors<'_>,
        params: Option<&Map>,
    ) -> Result<Self, Error> {
        // SAFETY: model, inputs, outputs are valid larod objects. The lifetime 'a
        // ensures they all outlive this JobRequest. Parameters are applied below
        // with larodSetJobRequestParams, which documents that it copies the map.
        let (ptr, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodCreateJobRequest,
                model.as_ptr(),
                inputs.as_ptr(),
                inputs.len(),
                outputs.as_ptr(),
                outputs.len(),
                std::ptr::null_mut(),
            )
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        debug_assert!(maybe_error.is_none());
        let mut request = Self {
            raw: ptr,
            conn_raw: conn.raw,
            _conn: PhantomData,
            _model: PhantomData,
            _tensors: PhantomData,
        };
        if let Some(params) = params {
            request.set_params(params)?;
        }
        Ok(request)
    }

    /// Run inference synchronously.
    ///
    /// After completion, the output tensors' backing memory contains the
    /// inference results.
    pub fn run(&self) -> Result<(), Error> {
        // SAFETY: conn_raw and self.raw are valid pointers. The lifetime 'a
        // ensures the connection and job request are both still alive.
        let (success, maybe_error) =
            unsafe { try_func!(larod_sys::larodRunJob, self.conn_raw, self.raw as *const _,) };
        if success {
            debug_assert!(maybe_error.is_none());
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    /// Set the job priority (0 = lowest, 255 = highest).
    pub fn set_priority(&mut self, priority: u8) -> Result<(), Error> {
        // SAFETY: self.raw is a valid job request pointer.
        let (success, maybe_error) =
            unsafe { try_func!(larod_sys::larodSetJobRequestPriority, self.raw, priority,) };
        if success {
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    /// Set optional parameters for this job request.
    ///
    /// Larod copies the map into the job request, so the map does not need to
    /// outlive this call.
    pub fn set_params(&mut self, params: &Map) -> Result<(), Error> {
        // SAFETY: self.raw and params pointer are valid. Larod copies the map
        // before this function returns.
        // Cast to *const because larodSetJobRequestParams takes *const larodMap.
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

impl Drop for JobRequest<'_> {
    fn drop(&mut self) {
        // larodDestroyJobRequest takes *mut *mut and nulls the pointer.
        crate::with_larod(|| unsafe { larod_sys::larodDestroyJobRequest(&mut self.raw) })
    }
}
