// TODO: Add documentation.
use std::{
    any,
    ffi::CStr,
    fmt::{Debug, Display, Formatter},
    mem,
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

type OnMessage = dyn FnMut(&Message) + Send + 'static;
type OnError = dyn FnMut(&Error) + Send + 'static;
type OnDone = dyn FnMut(Option<&Error>) + Send + 'static;

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
    on_error: *mut Box<OnError>,
}

impl Connection {
    // TODO: Consider adopting a builder-like pattern.
    pub fn try_new(on_error: Option<Box<OnError>>) -> Result<Self, Error> {
        debug!("Creating {}...", any::type_name::<Self>());
        unsafe {
            let mut error: *mut mdb_sys::mdb_error_t = std::ptr::null_mut();
            let on_error = match on_error {
                None => std::ptr::null_mut(),
                Some(on_error) => Box::into_raw(Box::new(on_error)),
            };
            let ptr = mdb_sys::mdb_connection_create(
                Some(Self::on_error),
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
                    Err(Error::new_owned(error))
                }
                (true, true) => {
                    panic!("mdb_connection_create returned neither a connection nor an error");
                }
            }
        }
    }

    unsafe extern "C" fn on_error(error: *mut mdb_sys::mdb_error_t, user_data: *mut c_void) {
        suppress_unwind!(|| {
            // TODO: Remove excessive logging once we are somewhat confident this works
            debug!("Handling error {error:?} with user_data {user_data:?}");
            let error = Error::new_borrowed(error);
            let user_data = user_data as *mut Box<OnError>;
            (*user_data)(&error);
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
    on_message: *mut Box<OnMessage>,
}

impl SubscriberConfig {
    pub fn try_new(topic: &CStr, source: &CStr, on_message: Box<OnMessage>) -> Result<Self, Error> {
        debug!("Creating {}...", any::type_name::<Self>());
        unsafe {
            let on_message = Box::into_raw(Box::new(on_message));

            let mut error: *mut mdb_sys::mdb_error_t = std::ptr::null_mut();
            let ptr = mdb_sys::mdb_subscriber_config_create(
                topic.as_ptr(),
                source.as_ptr(),
                Some(Self::on_message),
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
                    Err(Error::new_owned(error))
                }
                (true, true) => {
                    panic!(
                        "mdb_subscriber_config_create returned neither a connection nor an error"
                    )
                }
            }
        }
    }

    unsafe extern "C" fn on_message(
        message: *const mdb_sys::mdb_message_t,
        user_data: *mut c_void,
    ) {
        suppress_unwind!(|| {
            debug!("Handling message {message:?} with user_data {user_data:?}");
            debug!("Retrieving message...");
            let message = Message::from_raw(message);
            debug!("Retrieving callback...");
            let user_data = user_data as *mut Box<OnMessage>;
            debug!("Calling callback...");
            (*user_data)(&message);
        });
    }
}

impl Drop for SubscriberConfig {
    fn drop(&mut self) {
        // SAFETY: `Subscriber::try_new` sets `self.on_message = null_mut()` before passing it on
        // and no other code reads it, so it is safe to drop.
        unsafe {
            mdb_sys::mdb_subscriber_config_destroy(&mut self.ptr);
            if !self.on_message.is_null() {
                drop(Box::from_raw(self.on_message));
            }
        }
    }
}

pub struct Subscriber<'a> {
    // Ensure the raw connection is not destroyed before the subscriber
    _connection: &'a Connection,
    ptr: *mut mdb_sys::mdb_subscriber_t,
    on_done: *mut Box<OnDone>,
    // We don't need to keep the entire config alive, only the callback, because
    // `mdb_subscriber_create_async` will copy any information it keeps.
    on_message: *mut Box<OnMessage>,
}

impl<'a> Subscriber<'a> {
    pub fn try_new(
        connection: &'a Connection,
        mut config: SubscriberConfig,
        on_done: Box<OnDone>,
    ) -> Result<Self, Error> {
        debug!("Creating {}...", any::type_name::<Self>());
        unsafe {
            let on_done = Box::into_raw(Box::new(on_done));
            let mut error: *mut mdb_sys::mdb_error_t = std::ptr::null_mut();
            let ptr = mdb_sys::mdb_subscriber_create_async(
                connection.ptr,
                config.ptr,
                Some(Self::on_done),
                on_done as *mut c_void,
                &mut error,
            );
            match (ptr.is_null(), error.is_null()) {
                (false, false) => {
                    panic!("mdb_subscriber_create_async returned both a connection and an error")
                }
                (false, true) => {
                    let on_message = config.on_message;
                    config.on_message = std::ptr::null_mut();
                    Ok(Self {
                        _connection: connection,
                        ptr,
                        on_done,
                        on_message,
                    })
                }
                (true, false) => {
                    drop(Box::from_raw(on_done));
                    Err(Error::new_owned(error))
                }
                (true, true) => {
                    panic!("mdb_subscriber_create_async returned neither a connection nor an error")
                }
            }
        }
    }

    unsafe extern "C" fn on_done(error: *const mdb_sys::mdb_error_t, user_data: *mut c_void) {
        suppress_unwind!(|| {
            // TODO: Remove excessive logging once we are somewhat confident this works
            debug!("Handling on_done {error:?} with user_data {user_data:?}");
            let error = match error.is_null() {
                true => None,
                false => Some(Error::new_borrowed(error)),
            };
            let user_data = user_data as *mut Box<OnDone>;
            (*user_data)(error.as_ref());
        });
    }
}

impl<'a> Drop for Subscriber<'a> {
    // TODO: Consider allowing the user to control when potentially blocking calls happen.
    // SAFETY: Once destroy has returned, it is guaranteed that neither callback will be running nor
    // ever run again, so it is safe to drop them.
    // Naturally this does not apply to the on error callback, since that is associated with the
    // `Connection` and not the `Subscriber`.
    fn drop(&mut self) {
        unsafe {
            mdb_sys::mdb_subscriber_destroy(&mut self.ptr);
            drop(Box::from_raw(self.on_done));
            drop(Box::from_raw(self.on_message));
        }
    }
}

unsafe impl<'a> Send for Subscriber<'a> {}
// This is Sync as well afaic but so far I have not seen a use case, so it seems safer to defer
// implementation until it is needed or the Send and Sync properties are clearly guaranteed by
// the C API.

pub struct Message {
    ptr: *const mdb_sys::mdb_message_t,
}

impl Message {
    unsafe fn from_raw(ptr: *const mdb_sys::mdb_message_t) -> Self {
        // TODO: Can we encode that this is never owned?
        Self { ptr }
    }
    pub fn payload(&self) -> &[u8] {
        unsafe {
            let payload = *mdb_sys::mdb_message_get_payload(self.ptr);
            from_raw_parts(payload.data, payload.size)
        }
    }

    // TODO: Figure out a better type to return
    pub fn timestamp_bytes(&self) -> &[u8] {
        unsafe {
            from_raw_parts(
                mdb_sys::mdb_message_get_timestamp(self.ptr) as *const u8,
                mem::size_of::<mdb_sys::timespec>(),
            )
        }
    }
}
