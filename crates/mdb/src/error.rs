use std::ffi::{c_char, CStr};
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;

unsafe fn pchar_to_string(p_value: *const c_char) -> String {
    assert!(!p_value.is_null());
    let value = String::from(CStr::from_ptr(p_value).to_str().unwrap());
    value
}

pub struct OwnedError {
    ptr: *mut mdb_sys::mdb_error_t,
}

pub struct BorrowedError<'a> {
    ptr: *const mdb_sys::mdb_error_t,
    _marker: PhantomData<&'a mdb_sys::mdb_error_t>,
}

impl Debug for OwnedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Code: {}; Message: {:?};", self.code(), self.message())
    }
}

impl Display for OwnedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.code(), self.message())
    }
}

impl Debug for BorrowedError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Code: {}; Message: {:?};", self.code(), self.message())
    }
}

impl Display for BorrowedError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.code(), self.message())
    }
}

impl Drop for OwnedError {
    fn drop(&mut self) {
        unsafe {
            mdb_sys::mdb_error_destroy(&mut self.ptr);
        }
    }
}
unsafe impl Send for OwnedError {}
// Note that OwnedError is not Sync

impl std::error::Error for OwnedError {}

impl OwnedError {
    pub(crate) fn new(ptr: *mut mdb_sys::mdb_error_t) -> Self {
        OwnedError { ptr }
    }

    fn as_ref(&self) -> &mdb_sys::mdb_error_t {
        unsafe { self.ptr.as_ref().unwrap() }
    }

    fn code(&self) -> i32 {
        self.as_ref().code
    }
    fn message(&self) -> String {
        unsafe { pchar_to_string(self.as_ref().message) }
    }
}

impl BorrowedError<'_> {
    pub(crate) fn new(ptr: *const mdb_sys::mdb_error_t) -> Self {
        BorrowedError {
            ptr,
            _marker: PhantomData,
        }
    }

    fn as_ref(&self) -> &mdb_sys::mdb_error_t {
        unsafe { self.ptr.as_ref().unwrap() }
    }

    fn code(&self) -> i32 {
        self.as_ref().code
    }
    fn message(&self) -> String {
        unsafe { pchar_to_string(self.as_ref().message) }
    }
}
