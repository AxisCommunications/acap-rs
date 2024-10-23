use std::ffi::{c_char, CStr};
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;

unsafe fn pchar_to_string(p_value: *const c_char) -> String {
    assert!(!p_value.is_null());
    let value = String::from(CStr::from_ptr(p_value).to_str().unwrap());
    value
}

pub struct Error {
    ptr: *mut mdb_sys::mdb_error_t,
}

pub(crate) struct BorrowedError<'a> {
    ptr: *const mdb_sys::mdb_error_t,
    _marker: PhantomData<&'a mdb_sys::mdb_error_t>,
}

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
            mdb_sys::mdb_error_destroy(&mut self.ptr);
        }
    }
}
unsafe impl Send for Error {}
// Note that Error is not Sync

impl std::error::Error for Error {}

impl Error {
    pub(crate) fn new(ptr: *mut mdb_sys::mdb_error_t) -> Self {
        Error { ptr }
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

impl std::ops::Deref for BorrowedError<'_> {
    type Target = Error;
    // SAFETY: self.ptr is effectively a reference to Error provided by mdb-sys for the duration
    // of a callback. BorrowedError is tied to the lifetime of the contained pointer and since
    // Error does not implement Copy there is no way to leak the contained pointer.
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.ptr as *const Error) }
    }
}

impl BorrowedError<'_> {
    pub(crate) fn new(ptr: *const mdb_sys::mdb_error_t) -> Self {
        BorrowedError {
            ptr,
            _marker: PhantomData,
        }
    }
}
