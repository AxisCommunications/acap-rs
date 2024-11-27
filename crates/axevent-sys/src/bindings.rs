#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]
#![allow(unused_variables)]

type gchar = ::std::os::raw::c_char;
type gdouble = ::std::os::raw::c_double;

type gint = ::std::os::raw::c_int;
type guint = ::std::os::raw::c_uint;
use std::{
    collections::HashMap,
    ffi::c_void,
    ptr::addr_of,
    sync::{
        atomic::{AtomicU32, Ordering},
        LazyLock, Mutex,
    },
};

use crate::{mock_glib::DateTime, mock_glib_sys::GDateTime};
use glib_sys::{gboolean, gconstpointer, gpointer, GError};

pub const AXEventValueType_AX_VALUE_TYPE_INT: AXEventValueType = 0;
pub const AXEventValueType_AX_VALUE_TYPE_BOOL: AXEventValueType = 1;
pub const AXEventValueType_AX_VALUE_TYPE_DOUBLE: AXEventValueType = 2;
pub const AXEventValueType_AX_VALUE_TYPE_STRING: AXEventValueType = 3;
pub const AXEventValueType_AX_VALUE_TYPE_ELEMENT: AXEventValueType = 4;
pub type AXEventValueType = ::std::os::raw::c_uint;

pub const AXEventErrorCode_AX_EVENT_ERROR_GENERIC: AXEventErrorCode = 0;
pub const AXEventErrorCode_AX_EVENT_ERROR_INVALID_ARGUMENT: AXEventErrorCode = 1;
pub const AXEventErrorCode_AX_EVENT_ERROR_INCOMPATIBLE_VALUE: AXEventErrorCode = 2;
pub const AXEventErrorCode_AX_EVENT_ERROR_DECLARATION: AXEventErrorCode = 3;
pub const AXEventErrorCode_AX_EVENT_ERROR_UNDECLARE: AXEventErrorCode = 4;
pub const AXEventErrorCode_AX_EVENT_ERROR_SEND: AXEventErrorCode = 5;
pub const AXEventErrorCode_AX_EVENT_ERROR_SUBSCRIPTION: AXEventErrorCode = 6;
pub const AXEventErrorCode_AX_EVENT_ERROR_UNSUBSCRIBE: AXEventErrorCode = 7;
pub const AXEventErrorCode_AX_EVENT_ERROR_KEY_NOT_FOUND: AXEventErrorCode = 8;
pub const AXEventErrorCode_AX_EVENT_ERROR_END: AXEventErrorCode = 9;
pub type AXEventErrorCode = ::std::os::raw::c_uint;

pub static DECLARATIONS: LazyLock<Mutex<HashMap<u32, Declaration>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
pub static SUBSCRIPTIONS: LazyLock<Mutex<HashMap<u32, Subscription>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static NEXT_HANDLE: AtomicU32 = AtomicU32::new(0);

pub static PENDING: Mutex<Vec<Message>> = Mutex::new(Vec::new());

// TODO: Consider boxing the events
pub enum Message {
    Declaration(u32),
    Event(String),
}

#[derive(Debug)]
pub(crate) struct Declaration {
    pub callback: AXDeclarationCompleteCallback,
    pub user_data: *mut c_void,
}

unsafe impl Send for Declaration {}

#[derive(Debug)]
pub(crate) struct Subscription {
    pub callback: AXSubscriptionCallback,
    pub user_data: *mut c_void,
}

unsafe impl Send for Subscription {}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _AXEventKeyValueSet {
    _unused: usize,
}
pub type AXEventKeyValueSet = _AXEventKeyValueSet;

pub unsafe extern "C" fn ax_event_key_value_set_new() -> *mut AXEventKeyValueSet {
    Box::into_raw(Box::new(_AXEventKeyValueSet { _unused: 0 }))
}

pub unsafe extern "C" fn ax_event_key_value_set_free(key_value_set: *mut AXEventKeyValueSet) {
    drop(Box::from_raw(key_value_set));
}

pub unsafe extern "C" fn ax_event_key_value_set_add_key_value(
    key_value_set: *mut AXEventKeyValueSet,
    key: *const gchar,
    name_space: *const gchar,
    value: gconstpointer,
    value_type: AXEventValueType,
    error: *mut *mut GError,
) -> gboolean {
    1
}

pub unsafe extern "C" fn ax_event_key_value_set_mark_as_source(
    key_value_set: *mut AXEventKeyValueSet,
    key: *const gchar,
    name_space: *const gchar,
    error: *mut *mut GError,
) -> gboolean {
    1
}

pub unsafe extern "C" fn ax_event_key_value_set_mark_as_data(
    key_value_set: *mut AXEventKeyValueSet,
    key: *const gchar,
    name_space: *const gchar,
    error: *mut *mut GError,
) -> gboolean {
    1
}

pub unsafe extern "C" fn ax_event_key_value_set_mark_as_user_defined(
    key_value_set: *mut AXEventKeyValueSet,
    key: *const gchar,
    name_space: *const gchar,
    user_tag: *const gchar,
    error: *mut *mut GError,
) -> gboolean {
    1
}

pub unsafe extern "C" fn ax_event_key_value_set_add_nice_names(
    key_value_set: *mut AXEventKeyValueSet,
    key: *const gchar,
    name_space: *const gchar,
    key_nice_name: *const gchar,
    value_nice_name: *const gchar,
    error: *mut *mut GError,
) -> gboolean {
    1
}

pub unsafe extern "C" fn ax_event_key_value_set_get_value_type(
    key_value_set: *const AXEventKeyValueSet,
    key: *const gchar,
    name_space: *const gchar,
    value_type: *mut AXEventValueType,
    error: *mut *mut GError,
) -> gboolean {
    1
}

pub unsafe extern "C" fn ax_event_key_value_set_get_integer(
    key_value_set: *const AXEventKeyValueSet,
    key: *const gchar,
    name_space: *const gchar,
    value: *mut gint,
    error: *mut *mut GError,
) -> gboolean {
    1
}

pub unsafe extern "C" fn ax_event_key_value_set_get_boolean(
    key_value_set: *const AXEventKeyValueSet,
    key: *const gchar,
    name_space: *const gchar,
    value: *mut gboolean,
    error: *mut *mut GError,
) -> gboolean {
    1
}

pub unsafe extern "C" fn ax_event_key_value_set_get_double(
    key_value_set: *const AXEventKeyValueSet,
    key: *const gchar,
    name_space: *const gchar,
    value: *mut gdouble,
    error: *mut *mut GError,
) -> gboolean {
    1
}

pub unsafe extern "C" fn ax_event_key_value_set_get_string(
    key_value_set: *const AXEventKeyValueSet,
    key: *const gchar,
    name_space: *const gchar,
    value: *mut *mut gchar,
    error: *mut *mut GError,
) -> gboolean {
    1
}

#[repr(C)]
#[derive(Debug)]
pub struct _AXEvent {
    key_value_set: _AXEventKeyValueSet,
    t: GDateTime,
}
#[derive(Debug)]
pub struct AXEvent(_AXEvent);

pub unsafe extern "C" fn ax_event_new2(
    key_value_set: *mut AXEventKeyValueSet,
    time_stamp: *mut GDateTime,
) -> *mut AXEvent {
    Box::into_raw(Box::new(AXEvent(_AXEvent {
        key_value_set: *Box::from_raw(key_value_set),
        t: GDateTime::new(0),
    })))
}

impl AXEvent {
    pub fn to_string(&self) -> String {
        String::new()
    }

    pub fn from_str(s: &str) -> Result<Self, ()> {
        Ok(Self(_AXEvent {
            key_value_set: _AXEventKeyValueSet { _unused: 0 },
            t: GDateTime::new(0),
        }))
    }
}

pub unsafe extern "C" fn ax_event_free(event: *mut AXEvent) {
    drop(Box::from_raw(event));
}

pub unsafe extern "C" fn ax_event_get_key_value_set(
    event: *mut AXEvent,
) -> *const AXEventKeyValueSet {
    addr_of!((*event).0.key_value_set)
}

pub unsafe extern "C" fn ax_event_get_time_stamp2(event: *mut AXEvent) -> *mut GDateTime {
    &mut (*event).0.t as *mut GDateTime
}

pub struct _AXEventHandler {
    _unused: [u8; 0],
}

pub struct AXEventHandler(Mutex<_AXEventHandler>);

pub type AXSubscriptionCallback = ::std::option::Option<
    unsafe extern "C" fn(subscription: guint, event: *mut AXEvent, user_data: gpointer),
>;
pub type AXDeclarationCompleteCallback =
    ::std::option::Option<unsafe extern "C" fn(declaration: guint, user_data: gpointer)>;

pub unsafe extern "C" fn ax_event_handler_new() -> *mut AXEventHandler {
    Box::into_raw(Box::new(AXEventHandler(Mutex::new(_AXEventHandler {
        _unused: [0; 0],
    }))))
}

pub unsafe extern "C" fn ax_event_handler_free(event_handler: *mut AXEventHandler) {
    drop(Box::from_raw(event_handler))
}

pub unsafe extern "C" fn ax_event_handler_declare(
    event_handler: *mut AXEventHandler,
    key_value_set: *mut AXEventKeyValueSet,
    stateless: gboolean,
    declaration: *mut guint,
    callback: AXDeclarationCompleteCallback,
    user_data: gpointer,
    error: *mut *mut GError,
) -> gboolean {
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    assert!(DECLARATIONS
        .lock()
        .unwrap()
        .insert(
            handle,
            Declaration {
                callback,
                user_data
            }
        )
        .is_none());
    PENDING.lock().unwrap().push(Message::Declaration(handle));
    *declaration = handle;
    1
}

pub unsafe extern "C" fn ax_event_handler_undeclare(
    event_handler: *mut AXEventHandler,
    declaration: guint,
    error: *mut *mut GError,
) -> gboolean {
    // I don't know if the library requires unsubscribe to happen at most once.
    // If the Rust API makes this possible, we should probably remove the assert and mock the
    // actual behavior.
    assert!(DECLARATIONS.lock().unwrap().remove(&declaration).is_some());
    1
}

pub unsafe extern "C" fn ax_event_handler_send_event(
    event_handler: *mut AXEventHandler,
    declaration: guint,
    event: *mut AXEvent,
    error: *mut *mut GError,
) -> gboolean {
    dbg!("ax_event_handler_send_event");
    PENDING
        .lock()
        .unwrap()
        .push(Message::Event((*event).to_string()));
    1
}

pub unsafe extern "C" fn ax_event_handler_subscribe(
    event_handler: *mut AXEventHandler,
    key_value_set: *mut AXEventKeyValueSet,
    subscription: *mut guint,
    callback: AXSubscriptionCallback,
    user_data: gpointer,
    error: *mut *mut GError,
) -> gboolean {
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    assert!(SUBSCRIPTIONS
        .lock()
        .unwrap()
        .insert(
            handle,
            Subscription {
                callback,
                user_data
            }
        )
        .is_none());
    *subscription = handle;
    1
}

pub unsafe extern "C" fn ax_event_handler_unsubscribe(
    event_handler: *mut AXEventHandler,
    subscription: guint,
    error: *mut *mut GError,
) -> gboolean {
    // I don't know if the library requires unsubscribe to happen at most once.
    // If the Rust API makes this possible, we should probably remove the assert and mock the
    // actual behavior.
    assert!(SUBSCRIPTIONS
        .lock()
        .unwrap()
        .remove(&subscription)
        .is_some());
    1
}
