use std::{
    any,
    ffi::{c_void, CStr},
    marker::PhantomData,
    mem::ManuallyDrop,
    ptr::NonNull,
    sync::Mutex,
};

use libc::c_char;
use log::debug;
use mdb_sys::{
    mdb_channel_config_create, mdb_channel_config_set_info,
    mdb_channel_config_set_on_start_callback, mdb_channel_config_set_on_stop_callback,
    mdb_channel_create_async, mdb_channel_info_copy, mdb_channel_info_create,
    mdb_channel_info_destroy, mdb_channel_info_get_application_data,
    mdb_channel_info_get_application_data_mutable, mdb_channel_publish_async, mdb_dict_get_str,
    mdb_dict_set_str, mdb_dict_t,
};

/// Functionality for creating and publishing messages on a channel.
///
/// To publish messages on a channel, it must first be created.
/// The channel is identified by a source and is grouped under a topic.
/// The unix user using the API must have permission to create channels under a specific topic.
use crate::macros::{try_func, try_func_mandatory, try_func_mandatory_no_args, try_func_optional};
use crate::{
    message::Message, on_done_trampoline, on_start_stop_trampoline, Borrowed, BorrowedMessage,
    BorrowedMut, Connection, Deferred, Error,
};

/// The channel config type is a container for a channel configuration,
/// which is used when creating a channel.
pub struct ChannelConfig {
    ptr: *mut mdb_sys::mdb_channel_config_t,
    on_start: Option<Deferred>,
    on_stop: Option<Deferred>,
}

// TODO: Consider renaming to `ChannelBuilder`
impl ChannelConfig {
    /// Create a channel configuration.
    ///
    /// The channel configuration contains all data structures needed to create a
    /// channel.
    ///
    /// # Parameters
    ///
    /// - `topic`: The topic is used to group similar channels.
    ///   Permissions are set per topic.
    /// - `source`: The source or number of the channel that will be available to subscribers.
    pub fn create<F>(topic: &CStr, source: &CStr) -> Result<Self, Error>
    where
        F: for<'a> FnMut(BorrowedMessage<'a>) + Send + 'static,
    {
        debug!("Creating {}...", any::type_name::<Self>());
        let ptr = unsafe {
            try_func_mandatory!(mdb_channel_config_create, topic.as_ptr(), source.as_ptr())
        };
        Ok(Self {
            ptr,
            on_start: None,
            on_stop: None,
        })
    }

    // TODO: Consider renaming to `info`
    /// Set channel information.
    ///
    /// # Parameters
    ///
    /// - `info`: Channel info that will be available to subscribers.
    ///   This is the information that identifies the channel, its characteristics and capabilities.
    pub fn set_info(&mut self, info: &ChannelInfo) -> Result<(), Error> {
        unsafe { try_func!(mdb_channel_config_set_info, self.ptr, info.ptr) }
        Ok(())
    }

    // TODO: Consider renaming `on_start`.
    /// Register a callback for start requests.
    ///
    /// The callback will be called when there is at least one subscription on the
    ///  channel, which implies that the producer should start publishing messages.
    pub fn set_on_start_callback<F>(&mut self, on_start: F) -> Result<(), Error>
    where
        F: FnMut() + Send + 'static,
    {
        let raw_on_start = Box::into_raw(Box::new(on_start));
        let on_start = unsafe { Deferred::new(raw_on_start) };
        unsafe {
            try_func!(
                mdb_channel_config_set_on_start_callback,
                self.ptr,
                Some(on_start_stop_trampoline::<F>),
                raw_on_start as *mut c_void,
            );
        }
        self.on_start.replace(on_start);
        Ok(())
    }

    // TODO: Consider renaming `on_stop`.
    /// Register a callback for stop requests.
    ///
    /// The callback will be called when there are no more subscriptions on the
    /// channel, which implies that the producer should stop publishing messages.
    pub fn set_on_stop_callback<F>(&mut self, on_stop: F) -> Result<(), Error>
    where
        F: FnMut() + Send + 'static,
    {
        let raw_on_stop = Box::into_raw(Box::new(on_stop));
        let on_stop = unsafe { Deferred::new(raw_on_stop) };
        unsafe {
            try_func!(
                mdb_channel_config_set_on_stop_callback,
                self.ptr,
                Some(on_start_stop_trampoline::<F>),
                raw_on_stop as *mut c_void,
            );
        }
        self.on_stop.replace(on_stop);
        Ok(())
    }
}

impl Drop for ChannelConfig {
    fn drop(&mut self) {
        unsafe { mdb_sys::mdb_channel_config_destroy(&mut self.ptr) }
    }
}

/// The channel config type is a container for a channel configuration,
/// which is used when creating a channel.
pub struct ChannelInfo {
    ptr: *mut mdb_sys::mdb_channel_info_t,
}

impl ChannelInfo {
    // TODO: Consider renaming to `new` or `try_new`.
    /// Create a channel info object.
    pub fn create() -> Result<Self, Error> {
        let ptr = unsafe { try_func_mandatory_no_args!(mdb_channel_info_create) };
        Ok(Self { ptr })
    }

    // TODO: Consider renaming to `try_clone`.
    /// Create a deep copy of a channel info object.
    pub fn copy(&self) -> Result<Self, Error> {
        let ptr = unsafe { try_func_mandatory!(mdb_channel_info_copy, self.ptr) };
        Ok(Self { ptr })
    }

    // TODO: Consider renaming to `application_data`.
    /// Get immutable application_data.
    ///
    /// Application data.
    /// This data is defined by the producer and is specific to the producer application.
    pub fn get_application_data(&self) -> Result<Borrowed<'_, Dict>, Error> {
        let ptr = unsafe { try_func_mandatory!(mdb_channel_info_get_application_data, self.ptr,) };
        let inner = unsafe { ManuallyDrop::new(Dict(NonNull::new_unchecked(ptr as *mut _))) };
        Ok(Borrowed {
            inner,
            _marker: PhantomData,
        })
    }

    // TODO: Consider renaming to `application_data_mut`.
    /// Get mutable application_data.
    ///
    /// Application data.
    /// This data is defined by the producer and is specific to the producer application.
    pub fn get_application_data_mutable(&self) -> Result<BorrowedMut<'_, Dict>, Error> {
        let ptr = unsafe {
            try_func_mandatory!(mdb_channel_info_get_application_data_mutable, self.ptr,)
        };
        let inner = unsafe { ManuallyDrop::new(Dict(NonNull::new_unchecked(ptr as *mut _))) };
        Ok(BorrowedMut {
            inner,
            _marker: PhantomData,
        })
    }
}

impl Drop for ChannelInfo {
    fn drop(&mut self) {
        unsafe { mdb_channel_info_destroy(&mut self.ptr) }
    }
}

pub struct Channel<'a> {
    ptr: *mut mdb_sys::mdb_channel_t,
    _marker: PhantomData<&'a Connection>,
    callbacks: Mutex<Vec<Deferred>>,
}

impl<'a> Channel<'a> {
    /// TODO: Consider replacing or supplementing with a `builder` function.
    /// TODO: Consider renaming to `new` or `try_new`.
    /// Create a channel.
    ///
    /// # Parameters
    ///
    /// - `on_done`: Callback triggered when initial setup of channel is done or on error.
    pub fn create<F>(
        connection: &'a Connection,
        mut config: ChannelConfig,
        on_done: F,
    ) -> Result<Self, Error>
    where
        F: FnMut(Option<&Error>) + Send + 'static,
    {
        debug!("Creating {}...", any::type_name::<Self>());
        let raw_on_done = Box::into_raw(Box::new(on_done));
        let on_done = unsafe { Deferred::new(raw_on_done) };
        let ptr = unsafe {
            try_func_mandatory!(
                mdb_channel_create_async,
                connection.ptr,
                config.ptr,
                Some(on_done_trampoline::<F>),
                raw_on_done as *mut c_void,
            )
        };
        Ok(Self {
            _marker: PhantomData,
            ptr,
            callbacks: Mutex::new(
                [Some(on_done), config.on_start.take(), config.on_stop.take()]
                    .into_iter()
                    .flatten()
                    .collect(),
            ),
        })
    }

    /// Publish a message on channel.
    ///
    /// Messages are received by all subscribers that are subscribed to the channel
    /// when data is published.
    ///
    /// This function is non-blocking.
    /// Care must be taken to limit the publishing rate to a sustainable rate over time.
    ///
    /// # Parameters
    ///
    /// - `on_done`: Triggered when the data has been handed over to the data transporting mechanism
    ///   that will distribute it to active subscribers, or on error.
    pub fn publish_async<F>(&mut self, message: Message, on_done: F) -> Result<(), Error>
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

pub struct Dict(NonNull<mdb_dict_t>);

impl Dict {
    // TODO: Consider returning the old value if it exists.
    /// Set the value of key in the dict.
    ///
    /// If there already is a value for key, it is replaced by the new value.
    pub fn set_str(&mut self, key: &CStr, value: &CStr) -> Result<(), Error> {
        unsafe {
            try_func!(
                mdb_dict_set_str,
                self.0.as_ptr(),
                key.as_ptr(),
                value.as_ptr(),
            );
        }
        Ok(())
    }

    /// Get a string value corresponding to key from the dict.
    pub fn get_str(&self, key: &CStr) -> Result<Option<ForeignString>, Error> {
        let value = unsafe { try_func_optional!(mdb_dict_get_str, self.0.as_ptr(), key.as_ptr(),) };
        Ok(NonNull::new(value as *mut _).map(ForeignString))
    }
}

pub struct ForeignString(NonNull<c_char>);

impl ForeignString {
    pub fn as_str(&self) -> &str {
        unsafe { CStr::from_ptr(self.0.as_ptr()).to_str().unwrap() }
    }
}
