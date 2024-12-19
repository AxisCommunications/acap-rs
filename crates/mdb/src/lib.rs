//! A rust wrapper around the [Message Broker API] from [ACAP].
//!
//! [ACAP]: https://axiscommunications.github.io/acap-documentation/
//! [Message Broker API]: https://axiscommunications.github.io/acap-documentation/docs/api/src/api/message-broker/html/index.html

mod channel;
mod channel_info;
mod connection;
mod error;
pub(crate) mod macros;
mod message;
mod subscriber;

use log::debug;
use std::ffi::c_void;
// FIXME: Don't expose borrowed Message
use crate::error::BorrowedError;
use crate::macros::suppress_unwind;
pub use crate::{
    channel::{Channel, ChannelConfig, ChannelInfo},
    connection::Connection,
    error::Error,
    message::{Message, OwnedMessage},
    subscriber::{Subscriber, SubscriberConfig},
};

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

unsafe extern "C" fn on_done_trampoline<F>(
    error: *const mdb_sys::mdb_error_t,
    user_data: *mut c_void,
) where
    F: FnMut(Option<&Error>) + Send + 'static,
{
    suppress_unwind!(|| {
        // TODO: Remove excessive logging once we are somewhat confident this works
        debug!("Handling on_done {error:?} with user_data {user_data:?}");
        let error = match error.is_null() {
            true => None,
            false => Some(BorrowedError::new(error)),
        };
        let callback = &mut *(user_data as *mut F);
        callback(error.as_deref());
    });
}
