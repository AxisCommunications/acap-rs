//! Async wrapper around axevent
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use crate::flex::{Event, Handler, KeyValueSet};
use async_channel::Receiver;
use atomic_waker::AtomicWaker;
use futures_lite::Stream;
use log::warn;

/// Represents an event subscription that can be iterated asynchronously
///
/// Users should ensure that they regularly poll this stream since it uses an internal queue
/// to store incoming events. Currently this queue is unbounded which means that creating a
/// subscription and then never iterating over the events will eventually fill up the memory.
pub struct Subscription<'a> {
    handler: &'a Handler,
    waker: Arc<AtomicWaker>,
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
        let (tx, rx) = async_channel::unbounded::<Event>();
        let waker = Arc::new(AtomicWaker::new());
        let axevent_tx = tx.clone();
        let axevent_waker = waker.clone();
        let subscription = handler.subscribe(subscription_specification, move |_, evt| {
            if let Err(e) = axevent_tx.try_send(evt) {
                warn!("Unable to queue event due to {e}");
                return;
            }
            axevent_waker.wake();
        })?;
        Ok(Self {
            handler,
            rx: Box::pin(rx),
            subscription,
            waker,
        })
    }
}

impl Stream for Subscription<'_> {
    type Item = Event;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.waker.register(cx.waker());
        self.rx.as_mut().poll_next(cx)
    }
}
