//! Provide a async wrapper
use std::{
    pin::{self, Pin},
    sync::Arc,
    task::{self, Context, Poll},
};

use crate::flex::{Event, Handler, KeyValueSet};
use async_channel::{Receiver, Sender};
use atomic_waker::AtomicWaker;
use futures_lite::{FutureExt, Stream};

pub struct Subscription {
    handler: Arc<Handler>,
    waker: Arc<AtomicWaker>,
    tx: Sender<Event>,
    rx: Receiver<Event>,
    subscriptions: Vec<crate::flex::Subscription>,
}

impl Subscription {
    pub fn new(handler: Arc<Handler>) -> Self {
        let (tx, rx) = async_channel::unbounded::<Event>();
        Self {
            handler,
            tx,
            rx,
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
                if let Err(e) = inner.try_send(evt) {
                    // TBD
                    return;
                }
                println!("Successfully queued event. In queue: {}", inner.len());
                waker.wake();
            })
        else {
            return Err("Unable to subscribe");
        };
        self.subscriptions.push(id);
        Ok(())
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        for id in self.subscriptions.drain(..) {
            let _ = self.handler.unsubscribe(&id);
        }
    }
}

impl Stream for Subscription {
    type Item = Event;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let boxed = &mut self.rx.recv().boxed_local();
        self.waker.register(cx.waker());
        println!("About to poll");
        match boxed.poll(cx) {
            Poll::Ready(r) => {
                println!("Got one event");
                Poll::Ready(r.ok())
            }
            Poll::Pending => {
                println!("No events available, waiting");
                Poll::Pending
            }
        }
    }
}
