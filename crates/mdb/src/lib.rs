//! A rust wrapper around the [Message Broker API] from [ACAP].
//!
//! [ACAP]: https://axiscommunications.github.io/acap-documentation/
//! [Message Broker API]: https://axiscommunications.github.io/acap-documentation/docs/api/src/api/message-broker/html/index.html
// TODO: Add documentation.
use std::{
    any,
    ffi::CStr,
    fmt::{Debug, Display, Formatter},
    marker::PhantomData,
    slice::from_raw_parts,
};

use libc::{c_char, c_void};
use log::{debug, error};

macro_rules! suppress_unwind {
    ($f:expr) => {
        std::panic::catch_unwind($f).unwrap_or_else(|e| {
            // TODO: Verify that these cannot panic or replace them
            match e.downcast::<String>() {
                Ok(e) => error!("Caught panic in callback (string) {e}"),
                Err(e) => error!("Caught panic in callback (other) {e:?}"),
            };
        });
    };
}

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
    fn new_owned(ptr: *mut mdb_sys::mdb_error_t) -> Self {
        assert!(!ptr.is_null());
        Self(OwnedOrBorrowedError::Owned(ptr))
    }
    fn new_borrowed(ptr: *const mdb_sys::mdb_error_t) -> Self {
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

pub struct Connection {
    ptr: *mut mdb_sys::mdb_connection_t,
    _on_error: Option<Deferred>,
}

impl Connection {
    // TODO: Consider adopting a builder-like pattern.
    /// Passing `None` as the `on_error` callback requires qualifying the generic like so:
    ///
    /// ```
    /// let connection = Connection::try_new::<fn(Error)>(None);
    /// ```
    ///
    /// otherwise the generic is inferred:
    ///
    /// ```
    /// let connection = Connection::try_new(Some(|e: Error|
    ///     panic!("Failed to establish a connection: {e}")
    /// ));
    /// ```
    pub fn try_new<F>(on_error: Option<F>) -> Result<Self, Error>
    where
        F: FnMut(Error) + Send + 'static,
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
                (true, false) => Err(Error::new_owned(error)),
                (true, true) => {
                    panic!("mdb_connection_create returned neither a connection nor an error");
                }
            }
        }
    }

    unsafe extern "C" fn on_error<F>(error: *mut mdb_sys::mdb_error_t, user_data: *mut c_void)
    where
        F: FnMut(Error) + Send + 'static,
    {
        suppress_unwind!(|| {
            // TODO: Remove excessive logging once we are somewhat confident this works
            debug!("Handling error {error:?} with user_data {user_data:?}");
            let error = Error::new_borrowed(error);
            let callback = &mut *(user_data as *mut F);
            callback(error);
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

struct Deferred(Option<Box<dyn FnOnce()>>);
impl Drop for Deferred {
    fn drop(&mut self) {
        assert!(self.0.is_some());
        self.0.take().unwrap()()
    }
}

impl Deferred {
    unsafe fn new<T: 'static>(ptr: *mut T) -> Self {
        Self(Some(Box::new(move || drop(Box::from_raw(ptr)))))
    }
}

pub struct SubscriberConfig {
    ptr: *mut mdb_sys::mdb_subscriber_config_t,
    on_message: Option<Deferred>,
}

impl SubscriberConfig {
    pub fn try_new<F>(topic: &CStr, source: &CStr, on_message: F) -> Result<Self, Error>
    where
        F: for<'a> FnMut(Message<'a>) + Send + 'static,
    {
        debug!("Creating {}...", any::type_name::<Self>());
        unsafe {
            let raw_on_message = Box::into_raw(Box::new(on_message));
            // SAFETY: There a few ways this can be dropped:
            // * This function panics; since the user doesn't get a config back the config either
            //   is leaked or it wasn't created. In either case the pointer will never be
            //   dereferenced.
            // * The struct returned from this function is not used; since the callback is dropped
            //   after the drop implementation for this type this is sound even if drop would
            //   dereference the pointer, which it doesn't.
            // * The struct is passed to `Subscriber::try_new` which makes sure the callback
            //   outlives this `SubscriberConfig`.
            let on_message = Some(Deferred::new(raw_on_message));

            let mut error: *mut mdb_sys::mdb_error_t = std::ptr::null_mut();
            let ptr = mdb_sys::mdb_subscriber_config_create(
                topic.as_ptr(),
                source.as_ptr(),
                Some(Self::on_message::<F>),
                raw_on_message as *mut c_void,
                &mut error,
            );
            match (ptr.is_null(), error.is_null()) {
                (false, false) => {
                    panic!("mdb_subscriber_config_create returned both a connection and an error")
                }
                (false, true) => Ok(Self { ptr, on_message }),
                (true, false) => Err(Error::new_owned(error)),
                (true, true) => panic!(
                    "mdb_subscriber_config_create returned neither a connection nor an error"
                ),
            }
        }
    }

    unsafe extern "C" fn on_message<F>(
        message: *const mdb_sys::mdb_message_t,
        user_data: *mut c_void,
    ) where
        F: for<'a> FnMut(Message<'a>) + Send + 'static,
    {
        suppress_unwind!(|| {
            debug!("Handling message {message:?} with user_data {user_data:?}");
            debug!("Retrieving message...");
            let message = Message::from_raw(message);
            debug!("Retrieving callback...");
            let callback = &mut *(user_data as *mut F);
            debug!("Calling callback...");
            callback(message);
        });
    }
}

impl Drop for SubscriberConfig {
    fn drop(&mut self) {
        // SAFETY: This is always sound because it does not try to dereference the callback.
        unsafe {
            mdb_sys::mdb_subscriber_config_destroy(&mut self.ptr);
        }
    }
}

pub struct Subscriber<'a> {
    ptr: *mut mdb_sys::mdb_subscriber_t,
    _on_done: Deferred,
    // We don't need to keep the entire config alive, only the callback, because
    // `mdb_subscriber_create_async` will copy any information it keeps.
    _on_message: Deferred,
    _marker: PhantomData<&'a Connection>,
}

impl<'a> Subscriber<'a> {
    pub fn try_new<F>(
        connection: &'a Connection,
        mut config: SubscriberConfig,
        on_done: F,
    ) -> Result<Self, Error>
    where
        F: FnMut(Option<Error>) + Send + 'static,
    {
        debug!("Creating {}...", any::type_name::<Self>());
        unsafe {
            let raw_on_done = Box::into_raw(Box::new(on_done));
            let on_done = Deferred::new(raw_on_done);
            let mut error: *mut mdb_sys::mdb_error_t = std::ptr::null_mut();
            let ptr = mdb_sys::mdb_subscriber_create_async(
                connection.ptr,
                config.ptr,
                Some(Self::on_done::<F>),
                raw_on_done as *mut c_void,
                &mut error,
            );
            match (ptr.is_null(), error.is_null()) {
                (false, false) => {
                    panic!("mdb_subscriber_create_async returned both a connection and an error")
                }
                (false, true) => Ok(Self {
                    _marker: PhantomData,
                    ptr,
                    _on_done: on_done,
                    _on_message: config.on_message.take().unwrap(),
                }),
                (true, false) => Err(Error::new_owned(error)),
                (true, true) => {
                    panic!("mdb_subscriber_create_async returned neither a connection nor an error")
                }
            }
        }
    }

    unsafe extern "C" fn on_done<F>(error: *const mdb_sys::mdb_error_t, user_data: *mut c_void)
    where
        F: FnMut(Option<Error>) + Send + 'static,
    {
        suppress_unwind!(|| {
            // TODO: Remove excessive logging once we are somewhat confident this works
            debug!("Handling on_done {error:?} with user_data {user_data:?}");
            let error = match error.is_null() {
                true => None,
                false => Some(Error::new_borrowed(error)),
            };
            let callback = &mut *(user_data as *mut F);
            callback(error);
        });
    }
}

impl Drop for Subscriber<'_> {
    // TODO: Consider allowing the user to control when potentially blocking calls happen.
    // SAFETY: Once destroy has returned, it is guaranteed that neither callback will be running nor
    // ever run again, so it is safe to drop them.
    // Naturally this does not apply to the on error callback, since that is associated with the
    // `Connection` and not the `Subscriber`.
    fn drop(&mut self) {
        unsafe {
            mdb_sys::mdb_subscriber_destroy(&mut self.ptr);
        }
    }
}

unsafe impl Send for Subscriber<'_> {}
// This is Sync as well afaic but so far I have not seen a use case, so it seems safer to defer
// implementation until it is needed or the Send and Sync properties are clearly guaranteed by
// the C API.

pub struct Message<'a> {
    ptr: *const mdb_sys::mdb_message_t,
    _marker: PhantomData<&'a mdb_sys::mdb_message_t>,
}

impl Message<'_> {
    unsafe fn from_raw(ptr: *const mdb_sys::mdb_message_t) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }
    pub fn payload(&self) -> &[u8] {
        unsafe {
            let payload = *mdb_sys::mdb_message_get_payload(self.ptr);
            from_raw_parts(payload.data, payload.size)
        }
    }

    // TODO: Consider other types.
    // This is a monotonic timestamp but I haven't been able to verify that it is compatible with
    // `Instant` nor that it is even possible to create `Instant`s.
    pub fn timestamp(&self) -> &libc::timespec {
        unsafe {
            mdb_sys::mdb_message_get_timestamp(self.ptr)
                .as_ref()
                .expect("the C API guarantees that the timestamp is not null")
        }
    }
}
