use std::{
    fmt::Formatter,
    mem,
    sync::{
        atomic::{AtomicBool, Ordering},
        Condvar, Mutex,
    },
    thread::sleep,
    time::Duration,
};

use crate::mock_glib_sys::GDateTime;
use crate::{AXEvent, Message, DECLARATIONS, PENDING, SUBSCRIPTIONS};

pub struct MainLoop {
    stop: AtomicBool,
    running: Mutex<bool>,
    cond: Condvar,
}
impl MainLoop {
    pub fn new(context: Option<()>, is_running: bool) -> Self {
        Self {
            stop: AtomicBool::new(false),
            running: Mutex::new(true),
            cond: Condvar::new(),
        }
    }

    fn broadcast_declaration(handle: u32) {
        dbg!("broadcast_declaration", &handle);
        if let Some(declaration) = DECLARATIONS.lock().unwrap().get(&handle) {
            if let Some(callback) = declaration.callback {
                unsafe {
                    callback(handle, declaration.user_data);
                }
            }
        }
    }

    fn broadcast_event(event: String) {
        dbg!("broadcast_event", &event);
        for (handle, subscription) in SUBSCRIPTIONS.lock().unwrap().iter() {
            if let Some(callback) = subscription.callback {
                unsafe {
                    callback(
                        *handle,
                        Box::into_raw(Box::new(AXEvent::from_str(&event).unwrap())),
                        subscription.user_data,
                    );
                }
            }
        }
    }

    pub fn run(&self) {
        // for _ in 0..2 {
        //     for subscriber in SUBSCRIPTIONS.lock().unwrap().values() {
        //         if let Some(callback) = subscriber.callback {
        //             unsafe {
        //                 let mut key_value_set = crate::_AXEventKeyValueSet { _unused: 0 };
        //                 callback(
        //                     0,
        //                     Box::into_raw(Box::new(AXEvent(Mutex::new(_AXEvent {
        //                         key_value_set,
        //                     })))),
        //                     subscriber.user_data,
        //                 );
        //             }
        //         }
        //     }
        // }
        while !self.stop.load(Ordering::Relaxed) {
            println!("Running main loop");
            let messages: Vec<Message> = mem::take(PENDING.lock().unwrap().as_mut());
            for message in messages {
                match message {
                    Message::Declaration(handle) => Self::broadcast_declaration(handle),
                    Message::Event(event) => Self::broadcast_event(event),
                }
            }
            sleep(Duration::from_secs(1));
        }
        println!("Stopping main loop");
        *self.running.lock().unwrap() = true;
        self.cond.notify_all();
    }
    pub fn quit(&self) {
        println!("Quitting main loop");
        self.stop.store(true, Ordering::Relaxed);
    }
    pub fn join(&self) {
        println!("Joining main loop");
        drop(
            self.cond
                .wait_while(self.running.lock().unwrap(), |running| *running)
                .unwrap(),
        );
        println!("Joined main loop");
    }
}

#[derive(Debug)]
pub enum BoolError {}
impl std::fmt::Display for BoolError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for BoolError {}

#[derive(Debug)]
pub struct DateTime {
    raw: *mut GDateTime,
}
impl DateTime {
    pub fn from_unix_utc(t: i64) -> Result<Self, BoolError> {
        Ok(Self {
            raw: Box::into_raw(Box::new(GDateTime::new(t))),
        })
    }

    pub fn to_unix(&self) -> i64 {
        unsafe { (*self.raw).seconds_since_epoch() }
    }

    pub fn addr_of_mut(&mut self) -> *mut GDateTime {
        self as *mut _ as *mut GDateTime
    }
}

impl Drop for DateTime {
    fn drop(&mut self) {
        unsafe { drop(Box::from_raw(self.raw)) }
    }
}

pub mod translate {
    use super::Error;
    use crate::mock_glib::DateTime;
    use crate::mock_glib_sys::{g_date_time_ref, GDateTime};
    use glib_sys::GError;
    use std::ptr;

    pub trait IntoGlibPtr<P> {
        // Required method
        unsafe fn into_glib_ptr(self) -> P;
    }
    impl IntoGlibPtr<*mut GDateTime> for Option<DateTime> {
        unsafe fn into_glib_ptr(self) -> *mut GDateTime {
            match self {
                None => ptr::null_mut(),
                Some(DateTime { raw }) => raw,
            }
        }
    }

    pub fn from_glib_full(raw: *mut GError) -> Error {
        todo!()
    }
    // pub fn from_glib_full(raw: *mut GDateTime) -> DateTime {
    //     unsafe { DateTime { raw } }
    // }

    pub fn from_glib_none(raw: *mut GDateTime) -> DateTime {
        unsafe {
            DateTime {
                raw: g_date_time_ref(raw),
            }
        }
    }
}

#[derive(Debug)]
pub struct Error();

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for Error {}
