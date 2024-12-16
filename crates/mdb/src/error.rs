use std::{
    ffi::CStr,
    fmt::{Debug, Display, Formatter},
};

use libc::c_char;

unsafe fn pchar_to_string(p_value: *const c_char) -> String {
    assert!(!p_value.is_null());
    let value = String::from(CStr::from_ptr(p_value).to_str().unwrap());
    value
}

enum OwnedOrBorrowedError {
    Owned(*mut mdb_sys::mdb_error_t),
    Borrowed(*const mdb_sys::mdb_error_t),
}
pub struct Error(OwnedOrBorrowedError);

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Code: {}; Message: {:?};", self.code(), self.message())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.code(), self.message())
    }
}

impl Drop for Error {
    fn drop(&mut self) {
        unsafe {
            match &mut self.0 {
                OwnedOrBorrowedError::Owned(ptr) => {
                    mdb_sys::mdb_error_destroy(ptr);
                }
                OwnedOrBorrowedError::Borrowed(_) => {}
            }
        }
    }
}
unsafe impl Send for Error {}
// Note that error is not Sync

impl std::error::Error for Error {}

impl Error {
    pub(crate) fn new_owned(ptr: *mut mdb_sys::mdb_error_t) -> Self {
        assert!(!ptr.is_null());
        Self(OwnedOrBorrowedError::Owned(ptr))
    }
    pub(crate) fn new_borrowed(ptr: *const mdb_sys::mdb_error_t) -> Self {
        assert!(!ptr.is_null());
        Self(OwnedOrBorrowedError::Borrowed(ptr))
    }
    fn as_ref(&self) -> &mdb_sys::mdb_error_t {
        unsafe {
            match self.0 {
                OwnedOrBorrowedError::Owned(ptr) => ptr.as_ref(),
                OwnedOrBorrowedError::Borrowed(ptr) => ptr.as_ref(),
            }
        }
        .unwrap()
    }

    fn code(&self) -> i32 {
        self.as_ref().code
    }

    fn message(&self) -> String {
        unsafe { pchar_to_string(self.as_ref().message) }
    }
}
