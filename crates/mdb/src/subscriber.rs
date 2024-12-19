use std::{any, ffi::CStr, marker::PhantomData};

use libc::c_void;
use log::debug;

use crate::{macros::suppress_unwind, on_done_trampoline, Connection, Deferred, Error, Message};

pub struct SubscriberConfig {
    ptr: *mut mdb_sys::mdb_subscriber_config_t,
    // This isn't optional,
    // we just need a way to move the callback so it can be dropped with another object instead.
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
                (true, false) => Err(Error::new(error)),
                (true, true) => panic!(
                    "mdb_subscriber_config_create returned neither a connection nor an error"
                ),
            }
        }
    }

    fn into_callback(mut self) -> Deferred {
        self.on_message.take().unwrap()
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
        config: SubscriberConfig,
        on_done: F,
    ) -> Result<Self, Error>
    where
        F: FnMut(Option<&Error>) + Send + 'static,
    {
        debug!("Creating {}...", any::type_name::<Self>());
        unsafe {
            let raw_on_done = Box::into_raw(Box::new(on_done));
            let on_done = Deferred::new(raw_on_done);
            let mut error: *mut mdb_sys::mdb_error_t = std::ptr::null_mut();
            let ptr = mdb_sys::mdb_subscriber_create_async(
                connection.ptr,
                config.ptr,
                Some(on_done_trampoline::<F>),
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
                    _on_message: config.into_callback(),
                }),
                (true, false) => Err(Error::new(error)),
                (true, true) => {
                    panic!("mdb_subscriber_create_async returned neither a connection nor an error")
                }
            }
        }
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
