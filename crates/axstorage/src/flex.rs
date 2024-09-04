use std::{
    ffi::{c_char, CStr, CString},
    fmt::{Display, Formatter, Pointer},
    ptr,
};

use axstorage_sys::{
    ax_storage_get_path, ax_storage_get_status, ax_storage_get_storage_id, ax_storage_get_type,
    ax_storage_list, ax_storage_release_async, ax_storage_setup_async, ax_storage_subscribe,
    ax_storage_unsubscribe, gchar, guint, AXStorage, AXStorageStatusEventId,
    AXStorageStatusEventId_AX_STORAGE_AVAILABLE_EVENT,
    AXStorageStatusEventId_AX_STORAGE_EXITING_EVENT, AXStorageStatusEventId_AX_STORAGE_FULL_EVENT,
    AXStorageStatusEventId_AX_STORAGE_WRITABLE_EVENT, AXStorageType, AXStorageType_EXTERNAL_TYPE,
    AXStorageType_LOCAL_TYPE, AXStorageType_UNKNOWN_TYPE,
};
use glib::{
    ffi::GError,
    translate::{from_glib_full, FromGlibPtrFull, GlibPtrDefault, TransparentPtrType},
    Error, List,
};
use glib_sys::{g_free, g_str_equal, g_strdup, gpointer, GTRUE};

macro_rules! try_func {
    ($func:ident, $($arg:expr),+ $(,)?) => {{
        let mut error: *mut GError = ptr::null_mut();
        let retval = $func($( $arg ),+, &mut error);
        if !error.is_null() {
            return Err(from_glib_full(error));
        }
        retval
    }};
}

#[non_exhaustive]
pub enum StatusEventId {
    Available,
    Exiting,
    Full,
    Writable,
}

impl StatusEventId {
    fn into_raw(self) -> AXStorageStatusEventId {
        match self {
            Self::Available => AXStorageStatusEventId_AX_STORAGE_AVAILABLE_EVENT,
            Self::Exiting => AXStorageStatusEventId_AX_STORAGE_EXITING_EVENT,
            Self::Full => AXStorageStatusEventId_AX_STORAGE_FULL_EVENT,
            Self::Writable => AXStorageStatusEventId_AX_STORAGE_WRITABLE_EVENT,
        }
    }
}

#[derive(Debug)]
pub struct Storage {
    raw: *mut AXStorage,
}

unsafe impl Send for Storage {}

impl Storage {
    pub fn get_path(&mut self) -> Result<CString, Error> {
        unsafe {
            let path = try_func!(ax_storage_get_path, self.raw);
            Ok(CString::from_raw(path))
        }
    }
    pub fn get_storage_id(&self) -> Result<StorageId, Error> {
        unsafe {
            let storage_id = try_func!(ax_storage_get_storage_id, self.raw);
            Ok(StorageId(storage_id as gpointer))
        }
    }
    pub fn get_type(&self) -> Result<StorageType, Error> {
        unsafe {
            let storage_type = try_func!(ax_storage_get_type, self.raw);
            Ok(StorageType::from_raw(storage_type))
        }
    }

    pub fn release_async<F>(&mut self, callback: F) -> Result<(), Error>
    where
        F: FnMut(Option<Error>) + Send,
    {
        unsafe {
            let callback = Box::into_raw(Box::new(callback)) as gpointer;
            let success = try_func!(
                ax_storage_release_async,
                self.raw,
                Some(Self::release_async_trampoline::<F>),
                callback
            );
            debug_assert_eq!(success, GTRUE);
            Ok(())
        }
    }
    unsafe extern "C" fn release_async_trampoline<F>(user_data: gpointer, error: *mut GError)
    where
        F: FnMut(Option<Error>) + Send,
    {
        let error = if error.is_null() {
            None
        } else {
            Some(Error::from_glib_full(error))
        };
        let callback = &mut *(user_data as *mut F);
        callback(error);
    }
}

#[derive(Debug, Eq)]
pub struct StorageId(gpointer);

impl Clone for StorageId {
    fn clone(&self) -> Self {
        unsafe { Self(g_strdup(self.0 as *const gchar) as gpointer) }
    }
}

impl GlibPtrDefault for StorageId {
    type GlibType = *mut gchar;
}

impl PartialEq<Self> for StorageId {
    fn eq(&self, other: &Self) -> bool {
        unsafe { g_str_equal(self.0, other.0) == GTRUE }
    }
}

unsafe impl TransparentPtrType for StorageId {}

impl Display for StorageId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let s = CStr::from_ptr(self.0 as *const c_char);
            s.fmt(f)
        }
    }
}
impl Drop for StorageId {
    fn drop(&mut self) {
        unsafe {
            println!("Dropping StorageId@{:?}", self.0);
            g_free(self.0);
        }
    }
}

unsafe impl Send for StorageId {}

impl StorageId {
    pub fn get_status(&mut self, event: StatusEventId) -> Result<bool, Error> {
        unsafe {
            let mut error: *mut GError = ptr::null_mut();
            let status = ax_storage_get_status(self.0 as *mut gchar, event.into_raw(), &mut error);
            if !error.is_null() {
                return Err(Error::from_glib_full(error));
            }
            Ok(status == GTRUE)
        }
    }

    pub fn setup_async<F: FnMut(Result<Storage, Error>)>(
        &mut self,
        callback: F,
    ) -> Result<(), Error> {
        unsafe {
            let callback = Box::into_raw(Box::new(callback)) as gpointer;
            let success = try_func!(
                ax_storage_setup_async,
                self.0 as *mut gchar,
                Some(Self::setup_async_callback_trampoline::<F>),
                callback
            );
            debug_assert_eq!(success, GTRUE);
            Ok(())
        }
    }

    unsafe extern "C" fn setup_async_callback_trampoline<F>(
        storage: *mut AXStorage,
        user_data: gpointer,
        error: *mut GError,
    ) where
        F: FnMut(Result<Storage, Error>),
    {
        let result = if error.is_null() {
            debug_assert!(!storage.is_null());
            Ok(Storage { raw: storage })
        } else {
            debug_assert!(storage.is_null());
            Err(Error::from_glib_full(error))
        };
        let callback = &mut *(user_data as *mut F);
        callback(result);
    }

    pub fn subscribe<F>(&mut self, callback: F) -> Result<SubscriptionId, Error>
    where
        F: FnMut(StorageId, Option<Error>) + Send,
    {
        unsafe {
            let callback = Box::into_raw(Box::new(callback)) as gpointer;
            // Note that callback will be called anytime the status changes
            let subscription_id = try_func!(
                ax_storage_subscribe,
                self.0 as *mut gchar,
                Some(Self::subscribe_callback_trampoline::<F>),
                callback
            );
            debug_assert_ne!(subscription_id, 0);
            Ok(SubscriptionId(subscription_id))
            // TODO: Drop callback.
        }
    }

    unsafe extern "C" fn subscribe_callback_trampoline<F>(
        storage_id: *mut gchar,
        user_data: gpointer,
        error: *mut GError,
    ) where
        F: FnMut(StorageId, Option<Error>) + Send,
    {
        let storage_id = StorageId(storage_id as gpointer);
        let error = if error.is_null() {
            None
        } else {
            Some(Error::from_glib_full(error))
        };
        let callback = &mut *(user_data as *mut F);
        callback(storage_id, error);
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub enum StorageType {
    Local,
    External,
    Unknown,
}

impl StorageType {
    fn from_raw(value: AXStorageType) -> Self {
        match value {
            v if v == AXStorageType_LOCAL_TYPE => Self::Local,
            v if v == AXStorageType_EXTERNAL_TYPE => Self::External,
            v if v == AXStorageType_UNKNOWN_TYPE => Self::Unknown,
            _ => unreachable!(),
        }
    }
}

pub fn list() -> Result<List<StorageId>, Error> {
    unsafe {
        let mut error: *mut GError = ptr::null_mut();
        let list = ax_storage_list(&mut error);
        if !error.is_null() {
            debug_assert!(list.is_null());
            return Err(Error::from_glib_full(error));
        }
        Ok(List::from_glib_full(list))
    }
}

#[derive(Debug)]
pub struct SubscriptionId(guint);

impl SubscriptionId {
    // TODO: Consider consuming self and returning it the error, if any.
    pub fn unsubscribe(&self) -> Result<(), Error> {
        unsafe {
            let success = try_func!(ax_storage_unsubscribe, self.0);
            debug_assert_eq!(success, GTRUE);
            Ok(())
        }
    }
}
