// TODO: Add documentation.
use std::{any, ffi::CStr, marker::PhantomData, slice::from_raw_parts};

use libc::c_void;
use log::{debug, error};

pub mod error;

use error::{BorrowedError, OwnedError};

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

pub trait Callback<T>: FnMut(T) + Send + 'static {}
impl<T, F> Callback<T> for F where F: FnMut(T) + Send + 'static {}

type OnMessage = dyn for<'a> Callback<Message<'a>>;
type OnError = dyn for<'a> Callback<BorrowedError<'a>>;
type OnDone = dyn for<'a> Callback<Option<BorrowedError<'a>>>;

pub struct Connection {
    ptr: *mut mdb_sys::mdb_connection_t,
    on_error: *mut OnError,
}

impl Connection {
    // TODO: Consider adopting a builder-like pattern.
    /// Passing `None` as the `@on_error` callback requires qualifying the generic like so:
    ///
    /// ```
    /// let connection = Connection::try_new::<fn(BorrowedError)>(None);
    /// ```
    ///
    /// otherwise the generic is inferred:
    ///
    /// ```
    /// let connection = Connection::try_new(Some(|e: BorrowedError|
    ///     panic!("Failed to establish a connection: {e}")
    /// ));
    /// ```
    pub fn try_new<F>(on_error: Option<F>) -> Result<Self, OwnedError>
    where
        F: for<'a> Callback<BorrowedError<'a>>,
    {
        debug!("Creating {}...", any::type_name::<Self>());
        unsafe {
            let mut error: *mut mdb_sys::mdb_error_t = std::ptr::null_mut();
            let on_error = match on_error {
                None => std::ptr::null_mut(),
                Some(on_error) => Box::into_raw(Box::new(on_error)),
            };
            let ptr = mdb_sys::mdb_connection_create(
                Some(Self::on_error::<F>),
                on_error as *mut c_void,
                &mut error,
            );
            match (ptr.is_null(), error.is_null()) {
                (false, false) => {
                    panic!("mdb_connection_create returned both a connection and an error");
                }
                (false, true) => Ok(Self { ptr, on_error }),
                (true, false) => {
                    if !on_error.is_null() {
                        drop(Box::from_raw(on_error));
                    }
                    Err(OwnedError::new(error))
                }
                (true, true) => {
                    panic!("mdb_connection_create returned neither a connection nor an error");
                }
            }
        }
    }

    unsafe extern "C" fn on_error<F>(error: *mut mdb_sys::mdb_error_t, user_data: *mut c_void)
    where
        F: for<'a> Callback<BorrowedError<'a>>,
    {
        suppress_unwind!(|| {
            // TODO: Remove excessive logging once we are somewhat confident this works
            debug!("Handling error {error:?} with user_data {user_data:?}");
            let error = BorrowedError::new(error);
            let user_data = user_data as *mut F;
            (*user_data)(error);
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
            drop(Box::from_raw(self.on_error));
        }
    }
}

unsafe impl Send for Connection {}
unsafe impl Sync for Connection {}

pub struct SubscriberConfig {
    ptr: *mut mdb_sys::mdb_subscriber_config_t,
    on_message: *mut OnMessage,
}

impl SubscriberConfig {
    pub fn try_new<F>(topic: &CStr, source: &CStr, on_message: F) -> Result<Self, OwnedError>
    where
        F: for<'a> Callback<Message<'a>>,
    {
        debug!("Creating {}...", any::type_name::<Self>());
        unsafe {
            let on_message = Box::into_raw(Box::new(on_message));

            let mut error: *mut mdb_sys::mdb_error_t = std::ptr::null_mut();
            let ptr = mdb_sys::mdb_subscriber_config_create(
                topic.as_ptr(),
                source.as_ptr(),
                Some(Self::on_message::<F>),
                on_message as *mut c_void,
                &mut error,
            );
            match (ptr.is_null(), error.is_null()) {
                (false, false) => {
                    panic!("mdb_subscriber_config_create returned both a connection and an error")
                }
                (false, true) => Ok(Self { ptr, on_message }),
                (true, false) => {
                    drop(Box::from_raw(on_message));
                    Err(OwnedError::new(error))
                }
                (true, true) => {
                    panic!(
                        "mdb_subscriber_config_create returned neither a connection nor an error"
                    )
                }
            }
        }
    }

    unsafe extern "C" fn on_message<F>(
        message: *const mdb_sys::mdb_message_t,
        user_data: *mut c_void,
    ) where
        F: for<'a> Callback<Message<'a>>,
    {
        suppress_unwind!(|| {
            debug!("Handling message {message:?} with user_data {user_data:?}");
            debug!("Retrieving message...");
            let message = Message::from_raw(message);
            debug!("Retrieving callback...");
            let user_data = user_data as *mut F;
            debug!("Calling callback...");
            (*user_data)(message);
        });
    }
}

impl Drop for SubscriberConfig {
    fn drop(&mut self) {
        // SAFETY: `Subscriber` owns the `SubscriberConfig` that mdb_subscriber_create_async is
        // called with, and never touches on_message by itself. The only reference to self.on_message is
        // in mdb_subscriber_t which we destroy here so it is also safe to drop on_message.
        unsafe {
            mdb_sys::mdb_subscriber_config_destroy(&mut self.ptr);
            drop(Box::from_raw(self.on_message));
        }
    }
}

pub struct Subscriber<'a> {
    ptr: *mut mdb_sys::mdb_subscriber_t,
    on_done: *mut OnDone,
    _config: SubscriberConfig,
    _marker: PhantomData<&'a Connection>,
}

impl<'a> Subscriber<'a> {
    pub fn try_new<F>(
        connection: &'a Connection,
        config: SubscriberConfig,
        on_done: F,
    ) -> Result<Self, OwnedError>
    where
        F: for<'c> Callback<Option<BorrowedError<'c>>>,
    {
        debug!("Creating {}...", any::type_name::<Self>());
        unsafe {
            let on_done = Box::into_raw(Box::new(on_done));
            let mut error: *mut mdb_sys::mdb_error_t = std::ptr::null_mut();
            let ptr = mdb_sys::mdb_subscriber_create_async(
                connection.ptr,
                config.ptr,
                Some(Self::on_done::<F>),
                on_done as *mut c_void,
                &mut error,
            );
            match (ptr.is_null(), error.is_null()) {
                (false, false) => {
                    panic!("mdb_subscriber_create_async returned both a connection and an error")
                }
                (false, true) => Ok(Self {
                    ptr,
                    on_done,
                    _config: config,
                    _marker: PhantomData,
                }),
                (true, false) => {
                    drop(Box::from_raw(on_done));
                    Err(OwnedError::new(error))
                }
                (true, true) => {
                    panic!("mdb_subscriber_create_async returned neither a connection nor an error")
                }
            }
        }
    }

    unsafe extern "C" fn on_done<F>(error: *const mdb_sys::mdb_error_t, user_data: *mut c_void)
    where
        F: for<'c> Callback<Option<BorrowedError<'c>>>,
    {
        suppress_unwind!(|| {
            // TODO: Remove excessive logging once we are somewhat confident this works
            debug!("Handling on_done {error:?} with user_data {user_data:?}");
            let error = match error.is_null() {
                true => None,
                false => Some(BorrowedError::new(error)),
            };
            let user_data = user_data as *mut F;
            (*user_data)(error);
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
            drop(Box::from_raw(self.on_done))
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
