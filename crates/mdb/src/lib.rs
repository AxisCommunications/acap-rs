//! A rust wrapper around the [Message Broker API] from [ACAP].
//!
//! [ACAP]: https://axiscommunications.github.io/acap-documentation/
//! [Message Broker API]: https://axiscommunications.github.io/acap-documentation/docs/api/src/api/message-broker/html/index.html

mod connection;
mod error;
pub(crate) mod macros;
mod message;
mod subscriber;

pub use crate::{
    connection::Connection,
    error::Error,
    message::Message,
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
