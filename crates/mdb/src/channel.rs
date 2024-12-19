use crate::macros::try_func;
use crate::message::OwnedMessage;
use crate::{on_done_trampoline, Connection, Deferred, Error, Message};
use log::debug;
use mdb_sys::mdb_channel_publish_async;
use std::any;
use std::ffi::{c_void, CStr};
use std::marker::PhantomData;
use std::sync::Mutex;

pub struct ChannelConfig {
    ptr: *mut mdb_sys::mdb_channel_config_t,
}

impl ChannelConfig {
    pub fn try_new<F>(topic: &CStr, source: &CStr) -> Result<Self, Error>
    where
        F: for<'a> FnMut(Message<'a>) + Send + 'static,
    {
        debug!("Creating {}...", any::type_name::<Self>());
        unsafe {
            let mut error: *mut mdb_sys::mdb_error_t = std::ptr::null_mut();
            let ptr =
                mdb_sys::mdb_channel_config_create(topic.as_ptr(), source.as_ptr(), &mut error);
            match (ptr.is_null(), error.is_null()) {
                (false, false) => {
                    panic!("mdb_channel_config_create returned both a connection and an error")
                }
                (false, true) => Ok(Self { ptr }),
                (true, false) => Err(Error::new(error)),
                (true, true) => {
                    panic!("mdb_channel_config_create returned neither a connection nor an error")
                }
            }
        }
    }

    pub fn set_info(&mut self, info: &ChannelInfo) -> Result<(), Error> {
        let mut error: *mut mdb_sys::mdb_error_t = std::ptr::null_mut();
        unsafe {
            let success = mdb_sys::mdb_channel_config_set_info(self.ptr, info.ptr, &mut error);
            match (success, error.is_null()) {
                (false, false) => {
                    panic!("mdb_channel_config_set_info returned neither success nor error")
                }
                (false, true) => Err(Error::new(error)),
                (true, false) => Ok(()),
                (true, true) => {
                    panic!("mdb_channel_config_set_info returned both success and error")
                }
            }
        }
    }
}

impl Drop for ChannelConfig {
    fn drop(&mut self) {
        unsafe { mdb_sys::mdb_channel_config_destroy(&mut self.ptr) }
    }
}

pub struct ChannelInfo {
    ptr: *mut mdb_sys::mdb_channel_info_t,
}

impl Drop for ChannelInfo {
    fn drop(&mut self) {
        unsafe { mdb_sys::mdb_channel_info_destroy(&mut self.ptr) }
    }
}

pub struct Channel<'a> {
    ptr: *mut mdb_sys::mdb_channel_t,
    _marker: PhantomData<&'a Connection>,
    callbacks: Mutex<Vec<Deferred>>,
}

impl<'a> Channel<'a> {
    pub fn try_new<F>(
        connection: &'a Connection,
        config: ChannelConfig,
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
            let ptr = mdb_sys::mdb_channel_create_async(
                connection.ptr,
                config.ptr,
                Some(on_done_trampoline::<F>),
                raw_on_done as *mut c_void,
                &mut error,
            );
            match (ptr.is_null(), error.is_null()) {
                (false, false) => {
                    panic!("mdb_channel_create_async returned both a connection and an error")
                }
                (false, true) => Ok(Self {
                    _marker: PhantomData,
                    ptr,
                    callbacks: Mutex::new(vec![on_done]),
                }),
                (true, false) => Err(Error::new(error)),
                (true, true) => {
                    panic!("mdb_channel_create_async returned neither a connection nor an error")
                }
            }
        }
    }

    pub fn publish_async<F>(&mut self, message: OwnedMessage, on_done: F) -> Result<(), Error>
    where
        F: FnMut(Option<&Error>) + Send + 'static,
    {
        unsafe {
            let raw_on_done = Box::into_raw(Box::new(on_done));
            let on_done = Deferred::new(raw_on_done);
            try_func!(
                mdb_channel_publish_async,
                self.ptr,
                message.into_raw(),
                Some(on_done_trampoline::<F>),
                raw_on_done as *mut c_void
            );
            self.callbacks.lock().unwrap().push(on_done);
            Ok(())
        }
    }
}

impl Drop for Channel<'_> {
    fn drop(&mut self) {
        unsafe { mdb_sys::mdb_channel_destroy(&mut self.ptr) }
    }
}
