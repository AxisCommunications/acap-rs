use axstorage_sys::{
    ax_storage_error_quark, ax_storage_get_path, ax_storage_get_status, ax_storage_get_storage_id,
    ax_storage_get_type, ax_storage_list, ax_storage_release_async, ax_storage_setup_async,
    ax_storage_subscribe, ax_storage_unsubscribe, gchar, guint, AXStorage,
    AXStorageErrorCode_AX_STORAGE_ERROR_GENERIC,
    AXStorageErrorCode_AX_STORAGE_ERROR_INCOMPATIBLE_VALUE,
    AXStorageErrorCode_AX_STORAGE_ERROR_INVALID_ARGUMENT,
    AXStorageErrorCode_AX_STORAGE_ERROR_SETUP, AXStorageErrorCode_AX_STORAGE_ERROR_SUBSCRIPTION,
    AXStorageErrorCode_AX_STORAGE_ERROR_UNSUBSCRIBE, AXStorageStatusEventId,
    AXStorageStatusEventId_AX_STORAGE_AVAILABLE_EVENT,
    AXStorageStatusEventId_AX_STORAGE_EXITING_EVENT, AXStorageStatusEventId_AX_STORAGE_FULL_EVENT,
    AXStorageStatusEventId_AX_STORAGE_WRITABLE_EVENT, AXStorageType, AXStorageType_EXTERNAL_TYPE,
    AXStorageType_LOCAL_TYPE, AXStorageType_UNKNOWN_TYPE,
};
use glib::{
    error::ErrorDomain,
    ffi::GError,
    translate::{from_glib, FromGlibPtrFull},
    GStringPtr, List, Quark,
};
use glib_sys::{g_free, g_strdup, gpointer, GTRUE};
use std::ffi::OsStr;
use std::path::Path;
use std::{
    ffi::{c_char, c_void, CStr},
    mem,
    mem::ManuallyDrop,
    ptr,
    ptr::NonNull,
};
// The documentation states that we are responsible for freeing the callbacks, but it does state
// when it is safe to do so making it impossible to create a Rust abstraction that both:
// - does not leak memory and
// - is safe.
// TODO: Don't leak callbacks

// All C functions that take an `AXStorage` take it as a mutable pointer, even if it is used only to
// retrieve the path. This seems needlessly restrictive.
// TODO: Explore replacing `&mut Storage` with `&Storage`

// All C functions that take a `storage_id` take it as a mutable pointer, even if it used only to
// retrieve a status. This seems needlessly restrictive and forces the subscribe callback in the
// example app to use an exclusive reference in order to set up the storage.
// TODO: Explore replacing `&mut GStringPtr` with `&GStringPtr`

macro_rules! try_func {
    ($func:ident $(,$arg:expr)* $(,)?) => {{
        let mut error: *mut GError = ptr::null_mut();
        let retval = $func($($arg,)* &mut error);
        if !error.is_null() {
            return Err(glib::Error::from_glib_full(error));
        }
        retval
    }};
}

#[derive(Debug)]
#[repr(transparent)]
pub struct CStringPtr(NonNull<c_char>);

impl CStringPtr {
    /// Create an owned string from a foreign allocation
    ///
    /// # Safety
    ///
    /// In addition to the safety preconditions for [`CStr::from_ptr`] the memory must have been
    /// allocated in a manner compatible with [`glib_sys::g_free`] and there must be no other
    /// users of this memory.
    unsafe fn from_ptr(ptr: *mut c_char) -> Self {
        debug_assert!(!ptr.is_null());
        Self(NonNull::new_unchecked(ptr))
    }

    /// Create a [`CStr`] slice from the underlying pointer.
    pub fn to_c_str(&self) -> &CStr {
        // SAFETY: The preconditions for instantiating this type include all preconditions
        // for `CStr::from_ptr`.
        unsafe { CStr::from_ptr(self.0.as_ptr() as *const c_char) }
    }

    /// Create an [`OsStr`] slice from the underlying pointer.
    #[cfg(unix)]
    pub fn to_os_str(&self) -> &OsStr {
        use std::os::unix::ffi::OsStrExt;
        OsStr::from_bytes(self.to_c_str().to_bytes())
    }

    /// Create a [`Path`] slice from the underlying pointer.
    #[cfg(unix)]
    pub fn to_path(&self) -> &Path {
        self.to_os_str().as_ref()
    }
}

impl Clone for CStringPtr {
    fn clone(&self) -> Self {
        // SAFETY: The constructor guarantees that the pointer is not dangling and that the string
        // is null terminated.
        let ptr = unsafe { g_strdup(self.0.as_ptr()) };
        Self(NonNull::new(ptr).unwrap())
    }
}

impl Drop for CStringPtr {
    fn drop(&mut self) {
        // SAFETY: The preconditions for instantiating this type include:
        // - having full ownership of the memory.
        // - having allocated the memory in a manner that is compatible with `g_free`.
        unsafe {
            g_free(self.0.as_ptr() as *mut c_void);
        }
    }
}

/// A storage that is, or was, set up.
#[derive(Debug)]
pub struct Storage {
    raw: *mut AXStorage,
}

// TODO: SAFETY
unsafe impl Send for Storage {}

/// The ephemeral properties of a storage that can be observed.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum StatusEventId {
    /// Event for when storage becomes available and unavailable.
    Available,
    /// Event for when storage is about to exit, that's when client must stop using storage.
    Exiting,
    /// Event for when storage becomes full and when there is free space again.
    /// When the storage becomes full the client must not write more data to it;
    /// it's only ok to remove data.
    Full,
    /// Event for when storage becomes writable or readonly.
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

/// The possible locations of a storage.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Type {
    Local,
    External,
}

impl Type {
    fn from_raw(value: AXStorageType) -> Self {
        match value {
            v if v == AXStorageType_LOCAL_TYPE => Self::Local,
            v if v == AXStorageType_EXTERNAL_TYPE => Self::External,
            _ => unreachable!(),
        }
    }
}

/// The errors reported by the library.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Error {
    /// The error does not fit into any category.
    Generic,
    /// An invalid argument was supplied.
    InvalidArgument,
    /// The type of the supplied value does not match the type expected.
    IncompatibleValue,
    /// Something went wrong while subscribing for events.
    Subscription,
    /// Something went wrong while unsubscribing.
    Unsubscribe,
    /// Something went wrong while storage is set up for use.
    Setup,
    #[doc(hidden)]
    __Unknown(u32),
}

impl ErrorDomain for Error {
    fn domain() -> Quark {
        // TODO: SAFETY
        unsafe { from_glib(ax_storage_error_quark()) }
    }

    #[allow(non_upper_case_globals)]
    fn code(self) -> i32 {
        let code = match self {
            Self::Generic => AXStorageErrorCode_AX_STORAGE_ERROR_GENERIC,
            Self::InvalidArgument => AXStorageErrorCode_AX_STORAGE_ERROR_INVALID_ARGUMENT,
            Self::IncompatibleValue => AXStorageErrorCode_AX_STORAGE_ERROR_INCOMPATIBLE_VALUE,
            Self::Subscription => AXStorageErrorCode_AX_STORAGE_ERROR_SUBSCRIPTION,
            Self::Unsubscribe => AXStorageErrorCode_AX_STORAGE_ERROR_UNSUBSCRIBE,
            Self::Setup => AXStorageErrorCode_AX_STORAGE_ERROR_SETUP,
            Self::__Unknown(c) => c,
        };
        code as i32
    }

    #[allow(non_upper_case_globals)]
    fn from(code: i32) -> Option<Self>
    where
        Self: Sized,
    {
        let code = code as u32;
        Some(match code {
            AXStorageErrorCode_AX_STORAGE_ERROR_GENERIC => Self::Generic,
            AXStorageErrorCode_AX_STORAGE_ERROR_INVALID_ARGUMENT => Self::InvalidArgument,
            AXStorageErrorCode_AX_STORAGE_ERROR_INCOMPATIBLE_VALUE => Self::IncompatibleValue,
            AXStorageErrorCode_AX_STORAGE_ERROR_SUBSCRIPTION => Self::Subscription,
            AXStorageErrorCode_AX_STORAGE_ERROR_UNSUBSCRIBE => Self::Unsubscribe,
            AXStorageErrorCode_AX_STORAGE_ERROR_SETUP => Self::Setup,
            _ => Self::__Unknown(code),
        })
    }
}

/// Returns the IDs of all connected storages.
pub fn list() -> Result<List<GStringPtr>, glib::Error> {
    // TODO: SAFETY
    unsafe {
        let list = try_func!(ax_storage_list);
        Ok(List::from_glib_full(list))
    }
}

/// Subscribe to events.
///
/// # Parameters:
///
/// - `storage_id`: ID of Storage to subscribe to events for.
/// - `callback`: Closure called when an event changes state.
///   May be called even when no known event has changed state.
pub fn subscribe<F>(storage_id: &mut GStringPtr, callback: F) -> Result<guint, glib::Error>
where
    F: FnMut(&mut GStringPtr, Option<glib::Error>) + Send + 'static,
{
    // TODO: SAFETY
    unsafe {
        let callback = Box::into_raw(Box::new(callback));
        let id = try_func!(
            ax_storage_subscribe,
            to_mut_ptr(storage_id),
            Some(subscribe_callback_trampoline::<F>),
            callback as gpointer
        );
        debug_assert_ne!(id, 0);

        Ok(id)
    }
}

// TODO: SAFETY
unsafe extern "C" fn subscribe_callback_trampoline<F>(
    storage_id: *mut gchar,
    user_data: gpointer,
    error: *mut GError,
) where
    F: FnMut(&mut GStringPtr, Option<glib::Error>) + Send + 'static,
{
    let mut storage_id: ManuallyDrop<GStringPtr> = mem::transmute(storage_id);
    let error = if error.is_null() {
        None
    } else {
        Some(glib::Error::from_glib_full(error))
    };
    let callback = &mut *(user_data as *mut F);
    callback(&mut storage_id, error);
}

/// Stop subscribing to events.
///
/// # Parameters:
/// - `id`: A subscription ID as returned by [`subscribe`].
pub fn unsubscribe(id: guint) -> Result<(), glib::Error> {
    // TODO: SAFETY
    unsafe {
        let success = try_func!(ax_storage_unsubscribe, id);
        debug_assert_eq!(success, GTRUE);
        Ok(())
    }
}

/// Setup storage for use.
///
/// This method must be called before the storage is to be used in any way.
///
/// Note that [`release_async`] must be called on the returned [`Storage`].
///
/// # Parameters:
/// - `storage_id`: ID of storage to set up.
/// - `callback`: Closure called when the setup is done.
pub fn setup_async<F>(storage_id: &mut GStringPtr, callback: Option<F>) -> Result<(), glib::Error>
where
    F: FnMut(Result<Storage, glib::Error>) + Send + 'static,
{
    // TODO: SAFETY
    unsafe {
        let callback = callback.map(|c| Box::into_raw(Box::new(c)));
        let success = try_func!(
            ax_storage_setup_async,
            to_mut_ptr(storage_id),
            if callback.is_none() {
                None
            } else {
                Some(setup_async_callback_trampoline::<F>)
            },
            match callback {
                None => ptr::null_mut(),
                Some(callback) => callback as gpointer,
            }
        );
        debug_assert_eq!(success, GTRUE);
        Ok(())
    }
}

// TODO: SAFETY
unsafe extern "C" fn setup_async_callback_trampoline<F>(
    storage: *mut AXStorage,
    user_data: gpointer,
    error: *mut GError,
) where
    F: FnMut(Result<Storage, glib::Error>) + Send + 'static,
{
    let result = if error.is_null() {
        debug_assert!(!storage.is_null());
        Ok(Storage { raw: storage })
    } else {
        debug_assert!(storage.is_null());
        Err(glib::Error::from_glib_full(error))
    };
    let callback = &mut *(user_data as *mut F);
    callback(result);
}

/// Release storage.
///
/// This method should be called when done using the storage.
///
/// Note that the actual result of the release will be available in the callback.
///
/// # Parameters:
/// - `storage`: [`Storage`] to release.
/// - `callback`: Called when the release is done.
pub fn release_async<F>(storage: &mut Storage, callback: Option<F>) -> Result<(), glib::Error>
where
    F: FnMut(Option<glib::Error>) + Send + 'static,
{
    // TODO: SAFETY
    unsafe {
        let callback = callback.map(|c| Box::into_raw(Box::new(c)));
        let success = try_func!(
            ax_storage_release_async,
            storage.raw,
            if callback.is_none() {
                None
            } else {
                Some(release_async_trampoline::<F>)
            },
            match callback {
                None => ptr::null_mut(),
                Some(callback) => callback as gpointer,
            }
        );
        debug_assert_eq!(success, GTRUE);
        Ok(())
    }
}

// TODO: SAFETY
unsafe extern "C" fn release_async_trampoline<F>(user_data: gpointer, error: *mut GError)
where
    F: FnMut(Option<glib::Error>) + Send + 'static,
{
    let error = if error.is_null() {
        None
    } else {
        Some(glib::Error::from_glib_full(error))
    };
    let callback = &mut *(user_data as *mut F);
    callback(error);
}

/// Returns the location on the storage where the client should save its files.
pub fn get_path(storage: &mut Storage) -> Result<CStringPtr, glib::Error> {
    // TODO: SAFETY
    let path = unsafe { try_func!(ax_storage_get_path, storage.raw) };
    // SAFETY: This is safe because:
    // - The foreign function sets the error if the path is null in which case we return early
    //   above.
    // - The foreign function creates the value with `g_strdup` so it will be nul terminated
    //   and reads to up to and including the nul terminator are valid.
    // - This function owns the memory and does not mutate it.
    // - Paths will never be longer than `isize::MAX` in practice.
    unsafe { Ok(CStringPtr::from_ptr(path)) }
}

/// Returns the status of the provided event.
pub fn get_status(storage_id: &mut GStringPtr, event: StatusEventId) -> Result<bool, glib::Error> {
    // TODO: SAFETY
    unsafe {
        let status = try_func!(
            ax_storage_get_status,
            to_mut_ptr(storage_id),
            event.into_raw()
        );
        Ok(status == GTRUE)
    }
}

/// Returns the storage ID.
pub fn get_storage_id(storage: &mut Storage) -> Result<GStringPtr, glib::Error> {
    // TODO: SAFETY
    unsafe {
        let mut storage_id = try_func!(ax_storage_get_storage_id, storage.raw);
        Ok(ptr::read(
            &mut storage_id as *mut *mut gchar as *mut GStringPtr,
        ))
    }
}

/// Returns the storage type.
pub fn get_type(storage: &mut Storage) -> Result<Type, glib::Error> {
    // TODO: SAFETY
    unsafe {
        let storage_type = try_func!(ax_storage_get_type, storage.raw);
        debug_assert_ne!(storage_type, AXStorageType_UNKNOWN_TYPE);
        Ok(Type::from_raw(storage_type))
    }
}

// TODO: Verify safety of passing the result of this to C
fn to_mut_ptr(s: &mut GStringPtr) -> *mut gchar {
    s.as_ptr() as *mut gchar
}
