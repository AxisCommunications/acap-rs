//! Provide a async wrapper
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use crate::flex::{Event, Handler, KeyValueSet};
use async_channel::{Receiver, Sender};
use atomic_waker::AtomicWaker;
use futures_lite::Stream;
use pin_project::{pin_project, pinned_drop};

#[pin_project(PinnedDrop)]
pub struct Subscription {
    handler: Arc<Handler>,
    waker: Arc<AtomicWaker>,
    tx: Sender<Event>,
    #[pin]
    rx: Pin<Box<Receiver<Event>>>,
    subscriptions: Vec<crate::flex::Subscription>,
}

#[pinned_drop]
impl PinnedDrop for Subscription {
    fn drop(self: Pin<&mut Self>) {
        let this = self.project();
        for id in this.subscriptions.drain(..) {
            let _ = this.handler.unsubscribe(&id);
        }
    }
}

impl Subscription {
    // TODO(gustafo): replace Arc with H: AsRef<Handler>
    pub fn new(handler: Arc<Handler>) -> Self {
        let (tx, rx) = async_channel::unbounded::<Event>();
        Self {
            handler,
            tx,
            rx: Box::pin(rx),
            subscriptions: vec![],
            waker: Arc::new(AtomicWaker::new()),
        }
    }

    pub fn try_subscribe(
        &mut self,
        subscription_specification: KeyValueSet,
    ) -> Result<(), &'static str> {
        let inner = self.tx.clone();
        let waker = self.waker.clone();
        let Ok(id) = self
            .handler
            .subscribe(subscription_specification, move |_, evt| {
                if let Err(_e) = inner.try_send(evt) {
                    todo!();
                }
                waker.wake();
            })
        else {
            return Err("Unable to subscribe");
        };
        self.subscriptions.push(id);
        Ok(())
    }
}

impl Default for Subscription {
    fn default() -> Self {
        let handler = Arc::new(Handler::new());
        let (tx, rx) = async_channel::unbounded::<Event>();
        Self {
            handler,
            tx,
            rx: Box::pin(rx),
            subscriptions: vec![],
            waker: Arc::new(AtomicWaker::new()),
        }
    }
}

impl Stream for Subscription {
    type Item = Event;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        this.waker.register(cx.waker());
        this.rx.poll_next(cx)
    }
}
