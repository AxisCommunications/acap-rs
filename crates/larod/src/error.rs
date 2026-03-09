use std::ffi::CStr;
use std::fmt::{Debug, Display};

pub use larod_sys::larodErrorCode;

/// Calls a larod C function that takes `*mut *mut larodError` as its last argument.
/// Returns `(result, Option<Error>)`.
macro_rules! try_func {
    ($func:path $(,)?) => {{
        let mut error: *mut larod_sys::larodError = ::std::ptr::null_mut();
        let result = $func(&mut error);
        if error.is_null() {
            (result, None)
        } else {
            (result, Some($crate::Error::Larod($crate::LarodError::from_raw(error))))
        }
    }};
    ($func:path, $($arg:expr),+ $(,)?) => {{
        let mut error: *mut larod_sys::larodError = ::std::ptr::null_mut();
        let result = $func($( $arg ),+, &mut error);
        if error.is_null() {
            (result, None)
        } else {
            (result, Some($crate::Error::Larod($crate::LarodError::from_raw(error))))
        }
    }};
}

/// Error type for larod operations.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Larod(#[from] LarodError),
    #[error("larod returned an unexpected null pointer")]
    NullPointer,
    #[error("missing error data from larod library")]
    MissingError,
}

/// Error from the larod library.
///
/// Contains a copy of the error code and message. The original `larodError`
/// is freed immediately after copying via `larodClearError`.
pub struct LarodError {
    code: larodErrorCode,
    message: String,
}

impl LarodError {
    /// Copies data from a raw `larodError` pointer and frees it.
    ///
    /// # Safety
    ///
    /// `raw` must be a valid, non-null `larodError` pointer allocated by the larod library.
    pub(crate) fn from_raw(raw: *mut larod_sys::larodError) -> Self {
        assert!(!raw.is_null());

        // SAFETY: raw is non-null. We dereference to copy code and message pointer,
        // then read the message string, all before calling larodClearError which
        // invalidates the larodError and its contents.
        let larod_err = unsafe { *raw };
        let message = if larod_err.msg.is_null() {
            String::from("Unknown error")
        } else {
            unsafe { CStr::from_ptr(larod_err.msg) }
                .to_str()
                .unwrap_or("Invalid UTF-8 in error message")
                .to_string()
        };
        let code = larod_err.code;

        // Free the C-side error struct.
        let mut raw = raw;
        unsafe { larod_sys::larodClearError(&mut raw) };

        LarodError { code, message }
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(code: larodErrorCode, message: String) -> Self {
        Self { code, message }
    }

    pub fn code_name(&self) -> &'static str {
        match self.code {
            larodErrorCode::LAROD_ERROR_NONE => "LAROD_ERROR_NONE",
            larodErrorCode::LAROD_ERROR_JOB => "LAROD_ERROR_JOB",
            larodErrorCode::LAROD_ERROR_LOAD_MODEL => "LAROD_ERROR_LOAD_MODEL",
            larodErrorCode::LAROD_ERROR_FD => "LAROD_ERROR_FD",
            larodErrorCode::LAROD_ERROR_MODEL_NOT_FOUND => "LAROD_ERROR_MODEL_NOT_FOUND",
            larodErrorCode::LAROD_ERROR_PERMISSION => "LAROD_ERROR_PERMISSION",
            larodErrorCode::LAROD_ERROR_CONNECTION => "LAROD_ERROR_CONNECTION",
            larodErrorCode::LAROD_ERROR_CREATE_SESSION => "LAROD_ERROR_CREATE_SESSION",
            larodErrorCode::LAROD_ERROR_KILL_SESSION => "LAROD_ERROR_KILL_SESSION",
            larodErrorCode::LAROD_ERROR_INVALID_CHIP_ID => "LAROD_ERROR_INVALID_CHIP_ID",
            larodErrorCode::LAROD_ERROR_INVALID_ACCESS => "LAROD_ERROR_INVALID_ACCESS",
            larodErrorCode::LAROD_ERROR_DELETE_MODEL => "LAROD_ERROR_DELETE_MODEL",
            larodErrorCode::LAROD_ERROR_TENSOR_MISMATCH => "LAROD_ERROR_TENSOR_MISMATCH",
            larodErrorCode::LAROD_ERROR_VERSION_MISMATCH => "LAROD_ERROR_VERSION_MISMATCH",
            larodErrorCode::LAROD_ERROR_ALLOC => "LAROD_ERROR_ALLOC",
            larodErrorCode::LAROD_ERROR_POWER_NOT_AVAILABLE => "LAROD_ERROR_POWER_NOT_AVAILABLE",
            _ => "LAROD_ERROR_UNKNOWN",
        }
    }

    pub fn code(&self) -> larodErrorCode {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for LarodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}): {}", self.code_name(), self.code.0, self.message)
    }
}

impl Debug for LarodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LarodError")
            .field("code", &self.code)
            .field("code_name", &self.code_name())
            .field("message", &self.message)
            .finish()
    }
}

impl std::error::Error for LarodError {}
