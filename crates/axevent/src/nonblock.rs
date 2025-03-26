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
use pin_project::{pin_project, pinned_drop};

/// Represents an event subscription that can be iterated asynchronously
///
/// Users should ensure that they regularly poll this stream since it uses an internal queue
/// to store incoming events. Currently this queue is unbounded which means that creating a
/// subscription and then never iterating over the events will eventually fill up the memory.
#[pin_project(PinnedDrop)]
pub struct Subscription<'a> {
    handler: &'a Handler,
    waker: Arc<AtomicWaker>,
    #[pin]
    rx: Pin<Box<Receiver<Event>>>,
    subscription: crate::flex::Subscription,
}

#[pinned_drop]
impl PinnedDrop for Subscription<'_> {
    fn drop(self: Pin<&mut Self>) {
        let this = self.project();
        let _ = this.handler.unsubscribe(&this.subscription);
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

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        this.waker.register(cx.waker());
        this.rx.poll_next(cx)
    }
}
