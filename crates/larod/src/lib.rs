use larod_sys::*;
use std::{ffi::CStr, ptr};

type Result<T> = std::result::Result<T, LarodError>;

// Define our error types. These may be customized for our error handling cases.
// Now we will be able to write our own errors, defer to an underlying error
// implementation, or do something in between.
#[derive(Debug, Clone)]
struct LarodError {
    msg: String,
    code: LarodErrorCode,
}

impl From<*mut larodError> for LarodError {
    fn from(e: *mut larodError) -> Self {
        let le = unsafe { *e };
        let msg: String = unsafe { CStr::from_ptr(le.msg).to_str().unwrap().into() };
        let code: LarodErrorCode = le.code.into();
        Self { msg, code }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone)]
enum LarodErrorCode {
    NONE,
    JOB,
    LOAD_MODEL,
    FD,
    MODEL_NOT_FOUND,
    PERMISSION,
    CONNECTION,
    CREATE_SESSION,
    KILL_SESSION,
    INVALID_CHIP_ID,
    INVALID_ACCESS,
    DELETE_MODEL,
    TENSOR_MISMATCH,
    VERSION_MISMATCH,
    ALLOC,
    POWER_NOT_AVAILABLE,
    MAX_ERRNO,
}

impl From<larodErrorCode> for LarodErrorCode {
    fn from(code: larodErrorCode) -> LarodErrorCode {
        match code {
            larodErrorCode_LAROD_ERROR_NONE => LarodErrorCode::NONE,
            larodErrorCode_LAROD_ERROR_JOB => LarodErrorCode::JOB,
            larodErrorCode_LAROD_ERROR_LOAD_MODEL => LarodErrorCode::LOAD_MODEL,
            larodErrorCode_LAROD_ERROR_FD => LarodErrorCode::FD,
            larodErrorCode_LAROD_ERROR_MODEL_NOT_FOUND => LarodErrorCode::MODEL_NOT_FOUND,
            larodErrorCode_LAROD_ERROR_PERMISSION => LarodErrorCode::PERMISSION,
            larodErrorCode_LAROD_ERROR_CONNECTION => LarodErrorCode::CONNECTION,
            larodErrorCode_LAROD_ERROR_CREATE_SESSION => LarodErrorCode::CREATE_SESSION,
            larodErrorCode_LAROD_ERROR_KILL_SESSION => LarodErrorCode::KILL_SESSION,
            larodErrorCode_LAROD_ERROR_INVALID_CHIP_ID => LarodErrorCode::INVALID_CHIP_ID,
            larodErrorCode_LAROD_ERROR_INVALID_ACCESS => LarodErrorCode::INVALID_ACCESS,
            larodErrorCode_LAROD_ERROR_DELETE_MODEL => LarodErrorCode::DELETE_MODEL,
            larodErrorCode_LAROD_ERROR_TENSOR_MISMATCH => LarodErrorCode::TENSOR_MISMATCH,
            larodErrorCode_LAROD_ERROR_VERSION_MISMATCH => LarodErrorCode::VERSION_MISMATCH,
            larodErrorCode_LAROD_ERROR_ALLOC => LarodErrorCode::ALLOC,
            larodErrorCode_LAROD_ERROR_POWER_NOT_AVAILABLE => LarodErrorCode::POWER_NOT_AVAILABLE,
            larodErrorCode_LAROD_ERROR_MAX_ERRNO => LarodErrorCode::MAX_ERRNO,
            _ => unreachable!(),
        }
    }
}

fn create_map() -> Result<*mut larodMap> {
    let mut error: *mut larodError = ptr::null_mut();
    let map: *mut larodMap = unsafe { larodCreateMap(&mut error) };
    println!("map is_null? {:?}", map.is_null());
    println!("error is_null? {:?}", error.is_null());
    let e: LarodError = error.into();
    if !map.is_null() && matches!(e.code, LarodErrorCode::NONE) {
        Ok(map)
    } else {
        Err(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn it_creates_larod_map() {
        assert!(create_map().is_ok());
    }
}
