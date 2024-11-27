//! Ergonomic API for sending and receiving events.
//!
//! It is meant to make it easy to use the Event API in a sensible way, encoding knowledge that
//! is not apparent from the C API and its documentation.
//!
//! Note that since this API relies on empirical observations as well as the C API and its
//! documentation; a non-breaking change in the underlying C API could force a breaking change in
//! this API.
use std::{
    mem,
    ops::Add,
    sync::{mpsc::TrySendError, Arc},
    thread,
    thread::JoinHandle,
    time::{Duration, SystemTime},
};

// Drop in mocked implementation of glib
use axevent_sys::mock_glib;
use log::{debug, error, warn};

use crate::flex::{Event, Handler, KeyValueSet};

/// Convert `SystemTime` to `DateTime`
///
/// # Panics
///
/// Panics when `t` is either
/// - less than `i64::MAX` seconds before epoch,
/// - more than `i64::MAX` seconds after epoch, or
/// - represents another time outside the supported range of `DateTime`.
pub fn date_time(t: SystemTime) -> mock_glib::DateTime {
    // TODO: Verify that we are dealing with UTC timestamps
    let secs = match t.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => i64::try_from(d.as_secs()).unwrap(),
        Err(e) => -i64::try_from(e.duration().as_secs()).unwrap(),
    };
    mock_glib::DateTime::from_unix_utc(secs).unwrap()
}

/// Convert `DateTime` to `SystemTime`
///
/// # Panics
///
/// Panics when `t` is before the UNIX epoch.
pub fn system_time(t: mock_glib::DateTime) -> SystemTime {
    // TODO: Verify that we are dealing with UTC timestamps
    SystemTime::UNIX_EPOCH.add(Duration::from_secs(t.to_unix().try_into().unwrap()))
}

pub struct Declaration<'a> {
    pub rx: std::sync::mpsc::Receiver<()>,
    id: crate::flex::Declaration,
    handler: &'a Handler,
}

impl<'a> Drop for Declaration<'a> {
    fn drop(&mut self) {
        if let Err(e) = self.handler.undeclare(&self.id) {
            // TODO: Explore ways to communicate errors within the program
            error!("Could not undeclare because {e:?}")
        }
    }
}

impl<'a> Declaration<'a> {
    pub fn try_new(
        kvs: KeyValueSet,
        stateless: bool,
        handler: &'a Handler,
    ) -> Result<Self, crate::flex::Error> {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let id = handler.declare(
            &kvs,
            stateless,
            Some(move |_| match tx.try_send(()) {
                Ok(()) => debug!("Declaration complete sent"),
                Err(TrySendError::Disconnected(())) => debug!("Declaration complete not sent"),
                // Channel has capacity 1 and at most 1 message is ever sent
                Err(TrySendError::Full(())) => unreachable!(),
            }),
        )?;
        Ok(Self { rx, id, handler })
    }

    pub fn send_event(&self, event: Event) -> Result<(), crate::flex::Error> {
        self.handler.send_event(event, &self.id)
    }
}

pub struct MainLoop {
    main_loop: Arc<mock_glib::MainLoop>,
    join_handle: JoinHandle<()>,
}

impl Default for MainLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl MainLoop {
    pub fn new() -> Self {
        let main_loop = Arc::new(mock_glib::MainLoop::new(None, false));
        let join_handle = thread::spawn({
            let main_loop = Arc::clone(&main_loop);
            move || {
                debug!("Starting main loop");
                main_loop.run();
                debug!("Main loop stopped");
            }
        });
        Self {
            main_loop,
            join_handle,
        }
    }

    pub fn quit_and_join(self) -> thread::Result<()> {
        debug!("Quitting main loop...");
        self.main_loop.quit();
        debug!("Waiting for main loop...");
        self.join_handle.join()
    }
}

pub struct Subscription<'a> {
    pub rx: std::sync::mpsc::Receiver<Event>,
    id: crate::flex::Subscription,
    handler: &'a Handler,
}

impl<'a> Drop for Subscription<'a> {
    fn drop(&mut self) {
        if let Err(e) = self.handler.unsubscribe(&self.id) {
            error!("Could not unsubscribe because {e:?}")
        }
    }
}

impl<'a> Subscription<'a> {
    pub fn try_new(kvs: KeyValueSet, handler: &'a Handler) -> Result<Self, crate::flex::Error> {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let mut droppable_tx = Some(tx);
        let id = handler.subscribe(kvs, move |_, evt| {
            let Some(tx) = &droppable_tx else {
                debug!("Dropping event because sender was previously dropped");
                return;
            };
            if let Err(e) = tx.try_send(evt) {
                // Signal to receiver that the sender experienced a problem.
                drop(mem::take(&mut droppable_tx));
                // Explain to operator what happened.
                match e {
                    TrySendError::Full(_) => warn!("Receiver is not keeping up"),
                    // The `Drop` implementation usually ensures `unsubscribe` is called
                    // before the `rx` is dropped.
                    TrySendError::Disconnected(_) => warn!("Receiver disconnected"),
                }
            }
        })?;
        Ok(Self { rx, id, handler })
    }
}
