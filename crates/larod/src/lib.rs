use core::slice;
use larod_sys::*;
use std::{
    ffi::{CStr, CString},
    ptr::{self, slice_from_raw_parts},
};

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
    INVALID_TYPE,
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

pub struct LarodMap {
    raw: *mut larodMap,
}

impl LarodMap {
    fn new() -> Result<Self> {
        let mut error: *mut larodError = ptr::null_mut();
        let map: *mut larodMap = unsafe { larodCreateMap(&mut error) };
        println!("map is_null? {:?}", map.is_null());
        println!("error is_null? {:?}", error.is_null());
        let e: LarodError = error.into();
        if !map.is_null() && matches!(e.code, LarodErrorCode::NONE) {
            Ok(Self { raw: map })
        } else {
            Err(e)
        }
    }

    fn set_string(&mut self, k: &str, v: &str) -> Result<()> {
        let Ok(key_cstr) = CString::new(k.as_bytes()) else {
            return Err(LarodError {
                msg: String::from("Could not allocate set_string key CString"),
                code: LarodErrorCode::ALLOC,
            });
        };
        let Ok(value_cstr) = CString::new(v.as_bytes()) else {
            return Err(LarodError {
                msg: String::from("Could not allocate set_string value CString"),
                code: LarodErrorCode::ALLOC,
            });
        };
        let mut error: *mut larodError = ptr::null_mut();
        let success =
            unsafe { larodMapSetStr(self.raw, key_cstr.as_ptr(), value_cstr.as_ptr(), &mut error) };
        if success {
            Ok(())
        } else {
            Err(error.into())
        }
    }
    fn set_int(&mut self, k: &str, v: i64) -> Result<()> {
        let Ok(key_cstr) = CString::new(k.as_bytes()) else {
            return Err(LarodError {
                msg: String::from("Could not allocate set_string key CString"),
                code: LarodErrorCode::ALLOC,
            });
        };
        let mut error: *mut larodError = ptr::null_mut();
        let success = unsafe { larodMapSetInt(self.raw, key_cstr.as_ptr(), v, &mut error) };
        if success {
            Ok(())
        } else {
            Err(error.into())
        }
    }
    fn set_int_arr2(&mut self, k: &str, v: (i64, i64)) -> Result<()> {
        let Ok(key_cstr) = CString::new(k.as_bytes()) else {
            return Err(LarodError {
                msg: String::from("Could not allocate set_string key CString"),
                code: LarodErrorCode::ALLOC,
            });
        };
        let mut error: *mut larodError = ptr::null_mut();
        let success =
            unsafe { larodMapSetIntArr2(self.raw, key_cstr.as_ptr(), v.0, v.1, &mut error) };
        if success {
            Ok(())
        } else {
            Err(error.into())
        }
    }
    fn set_int_arr4(&mut self, k: &str, v: (i64, i64, i64, i64)) -> Result<()> {
        let Ok(key_cstr) = CString::new(k.as_bytes()) else {
            return Err(LarodError {
                msg: String::from("Could not allocate set_string key CString"),
                code: LarodErrorCode::ALLOC,
            });
        };
        let mut error: *mut larodError = ptr::null_mut();
        let success = unsafe {
            larodMapSetIntArr4(self.raw, key_cstr.as_ptr(), v.0, v.1, v.2, v.3, &mut error)
        };
        if success {
            Ok(())
        } else {
            Err(error.into())
        }
    }

    fn get_string(&self, k: &str) -> Result<String> {
        let Ok(key_cstr) = CString::new(k) else {
            return Err(LarodError {
                msg: String::from("Could not allocate set_string key CString"),
                code: LarodErrorCode::ALLOC,
            });
        };
        let mut error: *mut larodError = ptr::null_mut();
        let s = unsafe { CStr::from_ptr(larodMapGetStr(self.raw, key_cstr.as_ptr(), &mut error)) };
        let Ok(rs) = s.to_str() else {
            return Err(LarodError {
                msg: String::from("Returned string is not valid UTF-8"),
                code: LarodErrorCode::INVALID_TYPE,
            });
        };
        Ok(String::from(rs))
    }
    fn get_int(&self, k: &str) -> Result<i64> {
        let Ok(key_cstr) = CString::new(k) else {
            return Err(LarodError {
                msg: String::from("Could not allocate set_string key CString"),
                code: LarodErrorCode::ALLOC,
            });
        };
        let mut error: *mut larodError = ptr::null_mut();
        let mut v: i64 = 0;
        let success = unsafe { larodMapGetInt(self.raw, key_cstr.as_ptr(), &mut v, &mut error) };
        if success {
            Ok(v)
        } else {
            Err(error.into())
        }
    }
    fn get_int_arr2(&self, k: &str) -> Result<&[i64; 2]> {
        let Ok(key_cstr) = CString::new(k) else {
            return Err(LarodError {
                msg: String::from("Could not allocate set_string key CString"),
                code: LarodErrorCode::ALLOC,
            });
        };
        let mut error: *mut larodError = ptr::null_mut();
        let maybe_int_arr = unsafe {
            let ip = larodMapGetIntArr2(self.raw, key_cstr.as_ptr(), &mut error);
            if ip.is_null() {
                return Err(LarodError {
                    msg: String::from("Could not get integer array from LarodMap"),
                    code: LarodErrorCode::INVALID_TYPE,
                });
            } else {
                slice::from_raw_parts(ip, 2).try_into()
            }
        };
        let Ok(int_arr) = maybe_int_arr else {
            return Err(LarodError {
                msg: String::from("&[i64; 2] data stored in LarodMap is invalid."),
                code: LarodErrorCode::INVALID_TYPE,
            });
        };
        Ok(int_arr)
    }

    fn get_int_arr4(&self, k: &str) -> Result<&[i64; 4]> {
        let Ok(key_cstr) = CString::new(k) else {
            return Err(LarodError {
                msg: String::from("Could not allocate set_string key CString"),
                code: LarodErrorCode::ALLOC,
            });
        };
        let mut error: *mut larodError = ptr::null_mut();
        let maybe_int_arr = unsafe {
            let ip = larodMapGetIntArr4(self.raw, key_cstr.as_ptr(), &mut error);
            if ip.is_null() {
                return Err(LarodError {
                    msg: String::from("Could not get integer array from LarodMap"),
                    code: LarodErrorCode::INVALID_TYPE,
                });
            } else {
                slice::from_raw_parts(ip, 4).try_into()
            }
        };
        let Ok(int_arr) = maybe_int_arr else {
            return Err(LarodError {
                msg: String::from("&[i64; 2] data stored in LarodMap is invalid."),
                code: LarodErrorCode::INVALID_TYPE,
            });
        };
        Ok(int_arr)
    }
}

impl std::ops::Drop for LarodMap {
    fn drop(&mut self) {
        unsafe {
            larodDestroyMap(&mut self.raw);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn it_creates_larod_map() {
        assert!(LarodMap::new().is_ok());
    }

    #[test]
    fn it_drops_map() {
        let map = LarodMap::new().unwrap();
        std::mem::drop(map);
    }

    #[test]
    fn larod_map_can_set_str() {
        let mut map = LarodMap::new().unwrap();
        map.set_string("test_key", "test_value").unwrap();
    }

    #[test]
    fn larod_map_can_set_int() {
        let mut map = LarodMap::new().unwrap();
        map.set_int("test_key", 10).unwrap();
    }

    #[test]
    fn larod_map_can_set_2_tuple() {
        let mut map = LarodMap::new().unwrap();
        map.set_int_arr2("test_key", (1, 2)).unwrap();
    }

    #[test]
    fn larod_map_can_set_4_tuple() {
        let mut map = LarodMap::new().unwrap();
        map.set_int_arr4("test_key", (1, 2, 3, 4)).unwrap();
    }
}
