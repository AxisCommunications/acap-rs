//! Async wrapper around axevent
use std::{
    pin::Pin,
    process,
    task::{Context, Poll},
};

use crate::flex::{Event, Handler, KeyValueSet};
use async_channel::Receiver;
use futures_lite::Stream;
use log::{error, warn};

/// Represents an event subscription that can be iterated asynchronously
///
/// # Panics
///
/// The callback sent to the event system will panic if the internal queue fills up. To prevent
/// this happening, users must ensure that they regularly poll the stream to pull values out of
/// the queue.
pub struct Subscription<'a> {
    handler: &'a Handler,
    rx: Pin<Box<Receiver<Event>>>,
    subscription: crate::flex::Subscription,
}

impl Drop for Subscription<'_> {
    fn drop(&mut self) {
        let _ = self.handler.unsubscribe(&self.subscription);
    }
}

impl<'a> Subscription<'a> {
    pub fn try_new(
        handler: &'a Handler,
        subscription_specification: KeyValueSet,
    ) -> Result<Self, crate::flex::Error> {
        let (tx, rx) = async_channel::bounded::<Event>(16);
        let subscription = handler.subscribe(subscription_specification, move |_, evt| {
            if let Err(e) = tx.try_send(evt) {
                match e {
                    async_channel::TrySendError::Full(_) => {
                        error!("Event queue is full. This is most likely due to this stream not being polled.
                                Users of this API should ensure that they regularly try to pull
                                values out of the stream to prevent the queue from filling up.");
                        process::abort();
                    },
                    async_channel::TrySendError::Closed(_) => {
                        warn!("Event queue was closed unexpectedly, no more events will be delivered.");
                    },
                };
            }
        })?;
        Ok(Self {
            handler,
            rx: Box::pin(rx),
            subscription,
        })
    }
}

impl Stream for Subscription<'_> {
    type Item = Event;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.as_mut().poll_next(cx)
    }
}
