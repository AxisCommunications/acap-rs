#![allow(non_upper_case_globals)]
#![allow(clippy::redundant_closure_call)]
/// Flexible API for declaring and sending events.
///
/// It is meant to support migrating users and power users by providing a safe API that
/// * has a similar structure to the C API, and
/// * allows everything that can be done (safely) with the C API.
use std::ffi::CString;
use std::{
    any,
    collections::HashMap,
    ffi::{c_char, c_double, c_int, c_uint, c_void, CStr},
    fmt::Debug,
    process, ptr,
    sync::Mutex,
};

use axevent_sys::{
    ax_event_free, ax_event_get_key_value_set, ax_event_get_time_stamp2, ax_event_handler_declare,
    ax_event_handler_free, ax_event_handler_new, ax_event_handler_send_event,
    ax_event_handler_subscribe, ax_event_handler_undeclare, ax_event_handler_unsubscribe,
    ax_event_key_value_set_add_key_value, ax_event_key_value_set_add_nice_names,
    ax_event_key_value_set_free, ax_event_key_value_set_get_boolean,
    ax_event_key_value_set_get_double, ax_event_key_value_set_get_integer,
    ax_event_key_value_set_get_string, ax_event_key_value_set_get_value_type,
    ax_event_key_value_set_mark_as_data, ax_event_key_value_set_mark_as_source,
    ax_event_key_value_set_mark_as_user_defined, ax_event_key_value_set_new, ax_event_new2,
    AXEvent, AXEventHandler, AXEventKeyValueSet, AXEventValueType,
    AXEventValueType_AX_VALUE_TYPE_BOOL, AXEventValueType_AX_VALUE_TYPE_DOUBLE,
    AXEventValueType_AX_VALUE_TYPE_ELEMENT, AXEventValueType_AX_VALUE_TYPE_INT,
    AXEventValueType_AX_VALUE_TYPE_STRING,
};
pub use glib::Error;
use glib::{
    translate::{from_glib_full, from_glib_none, IntoGlibPtr},
    DateTime,
};
use glib_sys::{gboolean, gpointer, GError};
use log::debug;
macro_rules! abort_unwind {
    ($f:expr) => {
        std::panic::catch_unwind($f).unwrap_or_else(|_| {
            process::abort();
        });
    };
}
unsafe fn try_into_unit(is_ok: gboolean, error: *mut GError) -> Result<()> {
    debug_assert_ne!(is_ok == glib::ffi::GFALSE, error.is_null());
    if error.is_null() {
        Ok(())
    } else {
        Err(from_glib_full(error))
    }
}

macro_rules! try_func {
    ($func:ident, $($arg:expr),+ $(,)?) => {{
        let mut error: *mut GError = ptr::null_mut();
        let success = $func($( $arg ),+, &mut error);
        try_into_unit(success, error)
    }}
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Declaration(u32);

impl Declaration {
    // I think this should work
    unsafe extern "C" fn handle_callback(declaration: c_uint, user_data: *mut c_void) {
        abort_unwind!(|| {
            let callback = user_data as *mut Box<DeclarationCompleteCallback>;
            let callback = Box::from_raw(callback);
            callback(Self(declaration));
        });
    }
}

// It is not stated explicitly, but it makes no sense that this callback would be called more than
// once.
type DeclarationCompleteCallback = dyn FnOnce(Declaration) + Send + 'static;

pub struct Event {
    raw: *mut AXEvent,
    // TODO: Considering using separate owned and borrowed key value set types.
    // This is a hack to make it possible to hand out references.
    key_value_set: KeyValueSet,
}

impl Event {
    fn from_raw(raw: *mut AXEvent) -> Self {
        unsafe {
            // Converting to `*mut` is safe as long as we ensure that none of the mutable methods on
            // `KeyValueSet` are called, which we do by never handing out a mutable reference to the
            // `KeyValueSet`.
            let key_value_set = KeyValueSet::from_raw(ax_event_get_key_value_set(raw) as *mut _);
            Self { raw, key_value_set }
        }
    }

    pub fn new2(key_value_set: KeyValueSet, time_stamp: Option<DateTime>) -> Self {
        unsafe {
            let raw = ax_event_new2(key_value_set.raw, time_stamp.into_glib_ptr());
            Self { raw, key_value_set }
        }
    }

    pub fn key_value_set(&self) -> &KeyValueSet {
        &self.key_value_set
    }

    pub fn time_stamp2(&self) -> DateTime {
        unsafe { from_glib_none(ax_event_get_time_stamp2(self.raw)) }
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        debug!("Dropping {}", any::type_name::<Self>());
        self.key_value_set.raw = ptr::null_mut();
        unsafe {
            ax_event_free(self.raw);
        }
    }
}

unsafe impl Send for Event {}

pub struct Handler {
    raw: *mut AXEventHandler,
    // TODO: Investigate storing the `Box`es directly
    subscription_callbacks: Mutex<HashMap<Subscription, *mut Box<SubscriptionCallback>>>,
}

impl Default for Handler {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        debug!("Dropping {}", any::type_name::<Self>());
        // SAFETY: Empirically `ax_event_handler_free` will not return before all callbacks have
        // returned, so it is safe to free the stored callbacks.
        unsafe {
            ax_event_handler_free(self.raw);
            for (_, b) in self.subscription_callbacks.lock().unwrap().drain() {
                drop(Box::from_raw(b));
            }
        }
    }
}

unsafe impl Send for Handler {}
// The docs state that the function are "thread safe".
unsafe impl Sync for Handler {}

impl Handler {
    pub fn new() -> Self {
        unsafe {
            Self {
                raw: ax_event_handler_new(),
                subscription_callbacks: Mutex::new(HashMap::new()),
            }
        }
    }

    pub fn declare(
        &self,
        key_value_set: &KeyValueSet,
        stateless: bool,
        callback: Option<Box<DeclarationCompleteCallback>>,
    ) -> Result<Declaration> {
        // TODO: Don't leak callback when it is never called.
        let callback = callback.map(|c| Box::into_raw(Box::new(c)));
        unsafe {
            let mut declaration = c_uint::default();
            try_func!(
                ax_event_handler_declare,
                self.raw,
                key_value_set.raw,
                stateless as c_int,
                &mut declaration,
                if callback.is_none() {
                    None
                } else {
                    Some(Declaration::handle_callback)
                },
                match callback {
                    None => ptr::null_mut(),
                    Some(callback) => callback as *mut c_void,
                },
            )?;

            let handle = Declaration(declaration);
            Ok(handle)
        }
    }

    pub fn undeclare(&self, declaration: &Declaration) -> Result<()> {
        unsafe {
            try_func!(ax_event_handler_undeclare, self.raw, declaration.0)?;
            Ok(())
        }
    }

    pub fn send_event(&self, event: Event, declaration: &Declaration) -> Result<()> {
        unsafe {
            try_func!(
                ax_event_handler_send_event,
                self.raw,
                declaration.0,
                event.raw,
            )
        }
    }

    pub fn subscribe(
        &self,
        key_value_set: KeyValueSet,
        callback: Box<SubscriptionCallback>,
    ) -> Result<Subscription> {
        let callback = Box::into_raw(Box::new(callback));
        unsafe {
            let mut subscription = c_uint::default();
            try_func!(
                ax_event_handler_subscribe,
                self.raw,
                key_value_set.raw,
                &mut subscription,
                Some(Subscription::handle_callback),
                callback as *mut c_void,
            )?;

            let handle = Subscription(subscription);

            self.subscription_callbacks
                .lock()
                .unwrap()
                .insert(handle, callback);

            Ok(handle)
        }
    }

    pub fn unsubscribe(&self, subscription: &Subscription) -> Result<()> {
        unsafe {
            let result = try_func!(ax_event_handler_unsubscribe, self.raw, subscription.0);
            let b = self
                .subscription_callbacks
                .lock()
                .unwrap()
                .remove(subscription)
                .unwrap();
            drop(Box::from_raw(b));
            result
        }
    }
}

pub struct KeyValueSet {
    raw: *mut AXEventKeyValueSet,
}

impl Default for KeyValueSet {
    fn default() -> Self {
        KeyValueSet::new()
    }
}

impl Drop for KeyValueSet {
    fn drop(&mut self) {
        debug!("Dropping {}", any::type_name::<Self>());
        unsafe {
            // `Event` sets this to null when it is borrowed and should not be freed.
            if self.raw.is_null() {
                return;
            }
            ax_event_key_value_set_free(self.raw);
        }
    }
}

impl KeyValueSet {
    fn from_raw(raw: *mut AXEventKeyValueSet) -> Self {
        Self { raw }
    }

    pub fn new() -> Self {
        unsafe {
            Self {
                raw: ax_event_key_value_set_new(),
            }
        }
    }

    pub fn add_key_value<T: TypedValue>(
        &mut self,
        key: &CStr,
        namespace: Option<&CStr>,
        value: Option<T>,
    ) -> Result<&mut Self> {
        unsafe {
            let value: Option<Value> = value.map(|v| v.into());
            try_func!(
                ax_event_key_value_set_add_key_value,
                self.raw,
                key.as_ptr(),
                match namespace {
                    Some(v) => v.as_ptr(),
                    None => ptr::null(),
                },
                match &value {
                    None => ptr::null(),
                    Some(v) => match v {
                        Value::Int(v) => v as *const _ as *const c_void,
                        Value::Bool(v) => v as *const _ as *const c_void,
                        Value::Double(v) => v as *const _ as *const c_void,
                        Value::String(v) => v.as_ptr() as *const c_void,
                    },
                },
                match T::value_type() {
                    ValueType::Int => AXEventValueType_AX_VALUE_TYPE_INT,
                    ValueType::Bool => AXEventValueType_AX_VALUE_TYPE_BOOL,
                    ValueType::Double => AXEventValueType_AX_VALUE_TYPE_DOUBLE,
                    ValueType::String => AXEventValueType_AX_VALUE_TYPE_STRING,
                    ValueType::Element => AXEventValueType_AX_VALUE_TYPE_ELEMENT,
                },
            )?;
            Ok(self)
        }
    }

    pub fn mark_as_source(&mut self, key: &CStr, namespace: Option<&CStr>) -> Result<&mut Self> {
        unsafe {
            try_func!(
                ax_event_key_value_set_mark_as_source,
                self.raw,
                key.as_ptr(),
                match namespace {
                    Some(v) => v.as_ptr(),
                    None => ptr::null(),
                },
            )?;
            Ok(self)
        }
    }

    pub fn mark_as_data(&mut self, key: &CStr, namespace: Option<&CStr>) -> Result<&mut Self> {
        unsafe {
            try_func!(
                ax_event_key_value_set_mark_as_data,
                self.raw,
                key.as_ptr(),
                match namespace {
                    Some(v) => v.as_ptr(),
                    None => ptr::null(),
                },
            )?;
            Ok(self)
        }
    }
    pub fn mark_as_user_defined(
        &mut self,
        key: &CStr,
        namespace: Option<&CStr>,
        user_tag: &CStr,
    ) -> Result<&mut Self> {
        unsafe {
            try_func!(
                ax_event_key_value_set_mark_as_user_defined,
                self.raw,
                key.as_ptr(),
                match namespace {
                    Some(v) => v.as_ptr(),
                    None => ptr::null(),
                },
                user_tag.as_ptr(),
            )?;
            Ok(self)
        }
    }

    pub fn add_nice_names(
        &mut self,
        key: &CStr,
        namespace: Option<&CStr>,
        key_nice_name: Option<&CStr>,
        value_nice_name: Option<&CStr>,
    ) -> Result<&mut Self> {
        unsafe {
            try_func!(
                ax_event_key_value_set_add_nice_names,
                self.raw,
                key.as_ptr(),
                match namespace {
                    Some(v) => v.as_ptr(),
                    None => ptr::null(),
                },
                match key_nice_name {
                    Some(v) => v.as_ptr(),
                    None => ptr::null(),
                },
                match value_nice_name {
                    Some(v) => v.as_ptr(),
                    None => ptr::null(),
                },
            )?;
            Ok(self)
        }
    }

    pub fn get_value_type(&self, key: &CStr, namespace: Option<&CStr>) -> Result<ValueType> {
        unsafe {
            let mut value_type = AXEventValueType::default();
            try_func!(
                ax_event_key_value_set_get_value_type,
                self.raw,
                key.as_ptr(),
                match namespace {
                    Some(v) => v.as_ptr(),
                    None => ptr::null(),
                },
                &mut value_type,
            )?;
            Ok(match value_type {
                AXEventValueType_AX_VALUE_TYPE_INT => ValueType::Int,
                AXEventValueType_AX_VALUE_TYPE_BOOL => ValueType::Bool,
                AXEventValueType_AX_VALUE_TYPE_DOUBLE => ValueType::Double,
                AXEventValueType_AX_VALUE_TYPE_STRING => ValueType::String,
                AXEventValueType_AX_VALUE_TYPE_ELEMENT => ValueType::Element,
                _ => unreachable!(),
            })
        }
    }

    pub fn get_integer(&self, key: &CStr, namespace: Option<&CStr>) -> Result<i32> {
        unsafe {
            let mut value = c_int::default();
            try_func!(
                ax_event_key_value_set_get_integer,
                self.raw,
                key.as_ptr(),
                match namespace {
                    Some(v) => v.as_ptr(),
                    None => ptr::null(),
                },
                &mut value,
            )?;
            Ok(value)
        }
    }

    pub fn get_boolean(&self, key: &CStr, namespace: Option<&CStr>) -> Result<bool> {
        unsafe {
            let mut value = gboolean::default();
            try_func!(
                ax_event_key_value_set_get_boolean,
                self.raw,
                key.as_ptr(),
                match namespace {
                    Some(v) => v.as_ptr(),
                    None => ptr::null(),
                },
                &mut value,
            )?;
            Ok(match value {
                0 => false,
                1 => true,
                _ => panic!("Expected gboolean to be either 0 or 1 but got {value}"),
            })
        }
    }

    pub fn get_double(&self, key: &CStr, namespace: Option<&CStr>) -> Result<f64> {
        unsafe {
            let mut value = c_double::default();
            try_func!(
                ax_event_key_value_set_get_double,
                self.raw,
                key.as_ptr(),
                match namespace {
                    Some(v) => v.as_ptr(),
                    None => ptr::null(),
                },
                &mut value,
            )?;
            Ok(value)
        }
    }

    pub fn get_string(&self, key: &CStr, namespace: Option<&CStr>) -> Result<CString> {
        unsafe {
            let mut value: *mut c_char = ptr::null_mut();
            try_func!(
                ax_event_key_value_set_get_string,
                self.raw,
                key.as_ptr(),
                match namespace {
                    Some(v) => v.as_ptr(),
                    None => ptr::null(),
                },
                &mut value,
            )?;
            Ok(CString::from(CStr::from_ptr(value)))
        }
    }

    pub fn remove_key(&mut self, key: &CStr, namespace: Option<&CStr>) -> Result<&mut Self> {
        unsafe {
            try_func!(
                ax_event_key_value_set_mark_as_source,
                self.raw,
                key.as_ptr(),
                match namespace {
                    Some(v) => v.as_ptr(),
                    None => ptr::null(),
                },
            )?;
            Ok(self)
        }
    }
}

unsafe impl Send for KeyValueSet {}

type Result<T> = core::result::Result<T, Error>;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Subscription(u32);

impl Subscription {
    unsafe extern "C" fn handle_callback(
        subscription: c_uint,
        event: *mut AXEvent,
        user_data: gpointer,
    ) {
        abort_unwind!(|| {
            let callback = user_data as *mut Box<SubscriptionCallback>;
            let event = Event::from_raw(event);
            (*callback)(Subscription(subscription), event);
        });
    }
}

type SubscriptionCallback = dyn FnMut(Subscription, Event) + Send;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i32),
    Bool(bool),
    Double(f64),
    String(CString),
}

impl Eq for Value {}

macro_rules! impl_from_value {
    ($v:ident, $t:ty) => {
        impl From<Value> for Option<$t> {
            fn from(v: Value) -> Self {
                match v {
                    Value::$v(v) => Some(v),
                    _ => None,
                }
            }
        }
    };
    ($v:ident, $t:ty, $c:expr) => {
        impl From<Value> for Option<$t> {
            fn from(v: Value) -> Self {
                match v {
                    Value::$v(v) => Some($c(v)),
                    _ => None,
                }
            }
        }
    };
}
impl_from_value!(Bool, bool);
impl_from_value!(Double, f64);
impl_from_value!(Int, i32);
impl_from_value!(String, CString);

mod private {
    pub trait Sealed {}

    impl Sealed for usize {}
}

pub trait TypedValue: private::Sealed + Into<Value> {
    fn value_type() -> ValueType;
}

macro_rules! impl_typed_value {
    ($t:ty, $v:ident, $c:expr) => {
        impl private::Sealed for $t {}
        impl TypedValue for $t {
            fn value_type() -> ValueType {
                ValueType::$v
            }
        }
        impl From<$t> for Value {
            fn from(v: $t) -> Self {
                Self::$v($c(v))
            }
        }
    };
}
impl_typed_value!(bool, Bool, |v| v);
impl_typed_value!(f64, Double, |v| v);
impl_typed_value!(i32, Int, |v| v);
impl_typed_value!(&bool, Bool, |v: &bool| *v);
impl_typed_value!(&f64, Double, |v: &f64| *v);
impl_typed_value!(&i32, Int, |v: &i32| *v);
impl_typed_value!(&CStr, String, |v: &CStr| v.into());

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueType {
    Int,
    Bool,
    Double,
    String,
    Element,
}
