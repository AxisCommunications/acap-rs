use std::any;

use libc::c_void;
use log::debug;

pub use crate::error::Error;
use crate::{error::BorrowedError, macros::suppress_unwind, Deferred};

pub struct Connection {
    // FIXME: Safety
    pub(crate) ptr: *mut mdb_sys::mdb_connection_t,
    _on_error: Option<Deferred>,
}

impl Connection {
    // TODO: Consider adopting a builder-like pattern.
    /// Passing `None` as the `on_error` callback requires qualifying the generic like so:
    ///
    /// ```
    /// let connection = Connection::try_new::<fn(&Error)>(None);
    /// ```
    ///
    /// otherwise the generic is inferred:
    ///
    /// ```
    /// let connection = Connection::try_new(Some(|e: &Error|
    ///     panic!("Failed to establish a connection: {e}")
    /// ));
    /// ```
    pub fn try_new<F>(on_error: Option<F>) -> Result<Self, Error>
    where
        F: FnMut(&Error) + Send + 'static,
    {
        debug!("Creating {}...", any::type_name::<Self>());
        unsafe {
            let mut error: *mut mdb_sys::mdb_error_t = std::ptr::null_mut();
            let raw_on_error = match on_error {
                None => std::ptr::null_mut(),
                Some(on_error) => Box::into_raw(Box::new(on_error)),
            };
            let on_error = match raw_on_error.is_null() {
                false => Some(Deferred::new(raw_on_error)),
                true => None,
            };
            let ptr = mdb_sys::mdb_connection_create(
                Some(Self::on_error::<F>),
                raw_on_error as *mut c_void,
                &mut error,
            );
            match (ptr.is_null(), error.is_null()) {
                (false, false) => {
                    panic!("mdb_connection_create returned both a connection and an error");
                }
                (false, true) => Ok(Self {
                    ptr,
                    _on_error: on_error,
                }),
                (true, false) => Err(Error::new(error)),
                (true, true) => {
                    panic!("mdb_connection_create returned neither a connection nor an error");
                }
            }
        }
    }

    unsafe extern "C" fn on_error<F>(error: *const mdb_sys::mdb_error_t, user_data: *mut c_void)
    where
        F: FnMut(&Error) + Send + 'static,
    {
        suppress_unwind!(|| {
            // TODO: Remove excessive logging once we are somewhat confident this works
            debug!("Handling error {error:?} with user_data {user_data:?}");
            let error = BorrowedError::new(error);
            let callback = &mut *(user_data as *mut F);
            callback(&error);
        });
    }
}

impl Drop for Connection {
    // TODO: Consider avoiding a blocking call here or providing a method for manually destroying
    //  the connection.
    fn drop(&mut self) {
        // SAFETY: Once the connection is destroyed it, and its worker thread, will not use any of the pointers given
        // to it at construction so accessing `on_error` without synchronization is safe.
        unsafe {
            mdb_sys::mdb_connection_destroy(&mut self.ptr);
        }
    }
}

unsafe impl Send for Connection {}
unsafe impl Sync for Connection {}
