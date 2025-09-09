#![allow(non_upper_case_globals)]
#![allow(clippy::redundant_closure_call)]
//! Flexible API for declaring and sending events.
//!
//! It is meant to support migrating users and power users by providing a safe API that
//! * has a similar structure to the C API, and
//! * allows everything that can be done (safely) with the C API.
//!
//! Please see the ACAP documentation for [`axevent.h`](https://axiscommunications.github.io/acap-documentation/docs/api/src/api/axevent/html/axevent_8h.html).
use std::{
    any,
    collections::HashMap,
    ffi::{c_char, c_double, c_int, c_uint, c_void, CStr, CString},
    fmt::Debug,
    mem::ManuallyDrop,
    process, ptr,
    ptr::NonNull,
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
    ax_event_key_value_set_mark_as_user_defined, ax_event_key_value_set_new,
    ax_event_key_value_set_remove_key, ax_event_new2, AXEvent, AXEventHandler, AXEventKeyValueSet,
    AXEventValueType, AXEventValueType_AX_VALUE_TYPE_BOOL, AXEventValueType_AX_VALUE_TYPE_DOUBLE,
    AXEventValueType_AX_VALUE_TYPE_ELEMENT, AXEventValueType_AX_VALUE_TYPE_INT,
    AXEventValueType_AX_VALUE_TYPE_STRING,
};
pub use glib::Error;
use glib::{
    translate::{from_glib_full, from_glib_none, IntoGlibPtr},
    DateTime,
};
use glib_sys::{g_free, gboolean, gpointer, GError};
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

    pub fn as_c_str(&self) -> &CStr {
        // SAFETY: The preconditions for instantiating this type include all preconditions
        // for `CStr::from_ptr`.
        unsafe { CStr::from_ptr(self.0.as_ptr() as *const c_char) }
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

struct Deferred(Option<Box<dyn FnOnce()>>);

impl Drop for Deferred {
    fn drop(&mut self) {
        assert!(self.0.is_some());
        self.0.take().unwrap()()
    }
}

impl Deferred {
    unsafe fn new<T: 'static>(ptr: *mut T) -> Self {
        Self(Some(Box::new(move || drop(Box::from_raw(ptr)))))
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Declaration(u32);

impl Declaration {
    // I think this should work
    unsafe extern "C" fn handle_callback<F>(declaration: c_uint, user_data: *mut c_void)
    where
        F: FnMut(Declaration),
    {
        abort_unwind!(|| {
            let callback = &mut *(user_data as *mut F);
            callback(Self(declaration));
        });
    }
}

/// Please see the ACAP documentation for [`ax_event.sh`](https://axiscommunications.github.io/acap-documentation/docs/api/src/api/axevent/html/ax__event_8h.html).
pub struct Event {
    raw: *mut AXEvent,
    // TODO: Considering using separate owned and borrowed key value set types.
    // This is a hack to make it possible to hand out references.
    key_value_set: ManuallyDrop<KeyValueSet>,
}

impl Event {
    // Even though this function is private having it as safe makes it difficult to keep track of
    // when safety preconditions must be considered, and when they need not.
    // TODO: Mark as unsafe
    fn from_raw(raw: *mut AXEvent) -> Self {
        let key_value_set = unsafe { ax_event_get_key_value_set(raw) };
        debug_assert!(!key_value_set.is_null());
        // SAFETY:
        // - Converting `*const` to `*mut` is safe because it does come from neither a Rust
        //   reference nor a `restricted` C pointer, and we only ever access the resulting
        //   `KeyValueSet` through a non-mutable reference.
        // - `ax_event_get_key_value_set` never returns null (reasonable assumption).
        // TODO: Update C API documentation to guarantee this invariant.
        let key_value_set = unsafe {
            KeyValueSet {
                raw: NonNull::new_unchecked(key_value_set as *mut _),
            }
        };
        Self {
            raw,
            key_value_set: ManuallyDrop::new(key_value_set),
        }
    }

    pub fn new2(key_value_set: KeyValueSet, time_stamp: Option<DateTime>) -> Self {
        unsafe {
            let raw = ax_event_new2(key_value_set.raw.as_ptr(), time_stamp.into_glib_ptr());
            // `ax_event_new2` should return null only iff `key_value_set` is null.
            assert!(!raw.is_null());
            Self::from_raw(raw)
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
        unsafe {
            ax_event_free(self.raw);
        }
    }
}

unsafe impl Send for Event {}

/// Please see the ACAP documentation for [`ax_event_handler.h`](https://axiscommunications.github.io/acap-documentation/docs/api/src/api/axevent/html/ax__event__handler_8h.html).
pub struct Handler {
    raw: *mut AXEventHandler,
    declaration_callbacks: Mutex<HashMap<Declaration, Deferred>>,
    subscription_callbacks: Mutex<HashMap<Subscription, Deferred>>,
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
        // returned making it safe to drop the callbacks after this function returns.
        unsafe {
            ax_event_handler_free(self.raw);
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
                declaration_callbacks: Mutex::new(HashMap::new()),
                subscription_callbacks: Mutex::new(HashMap::new()),
            }
        }
    }

    // It is not stated explicitly, but it makes no sense that this callback would be called more
    // than once. If it can be verified that this is the case, it then it makes sense to find a
    // callback pattern that allows the callback to be dropped after being called from C or, if it
    // was never called from C, be dropped from Rust.
    // TODO: Relax the callback to `FnOnce`.
    /// Declare a new event
    ///
    /// # Parameters
    ///
    /// - `key_value_set`: A key-value set describing the event.
    /// - `stateless`: `true` if the event is a stateless event, otherwise `false`.
    /// - `callback`: Called when the declaration has been registered with the event system.
    ///   It is unlikely to be called more than once despite being a `FnMut`.
    ///
    /// # Examples
    ///
    /// To use without a callback, the type of the callback must nonetheless be provided e.g. like:
    ///
    /// ```
    /// # use axevent::flex::{Declaration, Handler, KeyValueSet};
    /// # let handler = Handler::new();
    /// # let key_value_set = KeyValueSet::new();
    /// handler.declare::<fn(Declaration)>(&key_value_set, true, None).unwrap();
    /// ```
    pub fn declare<F>(
        &self,
        key_value_set: &KeyValueSet,
        stateless: bool,
        callback: Option<F>,
    ) -> Result<Declaration>
    where
        F: FnMut(Declaration) + Send + 'static,
    {
        let raw_callback = callback.map(|c| Box::into_raw(Box::new(c)));
        // TODO: Verify these assumptions.
        // SAFETY: There are three ways that the callback can be dropped:
        // * When `ax_event_handler_declare` returns a failure and this function returns.
        //   The `axevent` runtime will presumably never use a callback that it failed to add.
        // * When undeclare is called and removes it from the map of callbacks.
        //   See safety comment in `undeclare`.
        // * When this handler is dropped.
        //   See safety comment ind `drop`.
        let callback = raw_callback.map(|c| unsafe { Deferred::new(c) });
        // TODO: Verify these assumptions.
        // SAFETY: Passing the callback to C is safe because:
        // - It is called at most once by the C code.
        // - If the callback runs, it will have completed by the time`ax_event_handler_undeclare`
        //   returns and it will not be called again after, making it safe to drop the callback.
        // - If the callback runs, it will have completed by the time `ax_event_handler_free`
        //   returns and it will not be called again after, making it safe to drop all callbacks.
        // - A declaration ID will not be issued twice, so the callback will not be overwritten and
        //   dropped while lent to C.
        unsafe {
            let mut declaration = c_uint::default();
            try_func!(
                ax_event_handler_declare,
                self.raw,
                key_value_set.raw.as_ptr(),
                stateless as c_int,
                &mut declaration,
                if raw_callback.is_none() {
                    None
                } else {
                    Some(Declaration::handle_callback::<F>)
                },
                match raw_callback {
                    None => ptr::null_mut(),
                    Some(callback) => callback as *mut c_void,
                },
            )?;

            let handle = Declaration(declaration);

            if let Some(callback) = callback {
                self.declaration_callbacks
                    .lock()
                    .unwrap()
                    .insert(handle, callback);
            }

            Ok(handle)
        }
    }

    /// Remove declaration
    ///
    /// The callback may or may not have been dropped if this returns an error.
    /// If it was not, it will be dropped when this handler goes out of scope.
    pub fn undeclare(&self, declaration: &Declaration) -> Result<()> {
        // TODO: Verify these assumptions.
        // SAFETY: If `ax_event_handler_undeclare` succeeds then, presumably, the callback will
        // not be used again after the function returns and it is safe to drop the callback.
        let result = unsafe { try_func!(ax_event_handler_undeclare, self.raw, declaration.0) };
        if result.is_ok() {
            self.declaration_callbacks
                .lock()
                .unwrap()
                .remove(declaration);
        }
        result
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

    pub fn subscribe<F>(&self, key_value_set: KeyValueSet, callback: F) -> Result<Subscription>
    where
        F: FnMut(Subscription, Event) + Send + 'static,
    {
        let raw_callback = Box::into_raw(Box::new(callback));
        // TODO: Verify these assumptions.
        // SAFETY: There are three ways that the callback can be dropped:
        // * When `ax_event_handler_subscribe returns a failure and this function returns.
        //   The `axevent` runtime will presumably never use a callback that it failed to add.
        // * When unsubscribe is called and removes it from the map of callbacks.
        //   See safety comment in `unsubscribe`.
        // * When this handler is dropped.
        //   See safety comment ind `drop`.
        let callback = unsafe { Deferred::new(raw_callback) };
        unsafe {
            let mut subscription = c_uint::default();
            try_func!(
                ax_event_handler_subscribe,
                self.raw,
                key_value_set.raw.as_ptr(),
                &mut subscription,
                Some(Subscription::handle_callback::<F>),
                raw_callback as *mut c_void,
            )?;

            let handle = Subscription(subscription);

            self.subscription_callbacks
                .lock()
                .unwrap()
                .insert(handle, callback);

            Ok(handle)
        }
    }

    /// Stop subscription
    ///
    /// The callback may or may not have been dropped if this returns an error.
    /// If it was not, it will be dropped when this handler goes out of scope.
    pub fn unsubscribe(&self, subscription: &Subscription) -> Result<()> {
        // TODO: More SAFETY
        // TODO: Verify these assumptions.
        // SAFETY: If `ax_event_handler_unsubscribe` succeeds then, presumably, the callback will
        // not be used again after the function returns and it is safe to drop the callback.
        let result = unsafe { try_func!(ax_event_handler_unsubscribe, self.raw, subscription.0) };
        if result.is_ok() {
            self.subscription_callbacks
                .lock()
                .unwrap()
                .remove(subscription)
                .unwrap();
        }
        result
    }
}

/// Please see the ACAP documentation for [`ax_event_key_value_set.h`](https://axiscommunications.github.io/acap-documentation/docs/api/src/api/axevent/html/ax__event__key__value__set_8h.html).
pub struct KeyValueSet {
    raw: NonNull<AXEventKeyValueSet>,
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
            ax_event_key_value_set_free(self.raw.as_ptr());
        }
    }
}

impl KeyValueSet {
    pub fn new() -> Self {
        unsafe {
            Self {
                raw: NonNull::new_unchecked(ax_event_key_value_set_new()),
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
                self.raw.as_ptr(),
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
                self.raw.as_ptr(),
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
                self.raw.as_ptr(),
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
                self.raw.as_ptr(),
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
                self.raw.as_ptr(),
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
                self.raw.as_ptr(),
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
                self.raw.as_ptr(),
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
                self.raw.as_ptr(),
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
                self.raw.as_ptr(),
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

    pub fn get_string(&self, key: &CStr, namespace: Option<&CStr>) -> Result<CStringPtr> {
        unsafe {
            let mut value: *mut c_char = ptr::null_mut();
            try_func!(
                ax_event_key_value_set_get_string,
                self.raw.as_ptr(),
                key.as_ptr(),
                match namespace {
                    Some(v) => v.as_ptr(),
                    None => ptr::null(),
                },
                &mut value,
            )?;
            // SAFETY: This is safe because:
            // - The foreign function sets the error if the value is null in which case we return
            //   early above.
            // - The foreign function creates the value with `g_strdup` so it will be nul
            //   terminated, reads to up to and including the nul terminator are valid, and it may
            //   be freed using `g_free`.
            // - This function owns the memory and does not mutate it.
            // - Values will never be longer than `isize::MAX` in practice.
            Ok(CStringPtr::from_ptr(value))
        }
    }

    pub fn remove_key(&mut self, key: &CStr, namespace: Option<&CStr>) -> Result<&mut Self> {
        unsafe {
            try_func!(
                ax_event_key_value_set_remove_key,
                self.raw.as_ptr(),
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
    unsafe extern "C" fn handle_callback<F>(
        subscription: c_uint,
        event: *mut AXEvent,
        user_data: gpointer,
    ) where
        F: FnMut(Subscription, Event) + Send,
    {
        abort_unwind!(|| {
            let callback = &mut *(user_data as *mut F);
            let event = Event::from_raw(event);
            callback(Subscription(subscription), event);
        });
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i32),
    Bool(i32),
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
impl_from_value!(Bool, bool, |v| v != 0);
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
impl_typed_value!(bool, Bool, |v| v as i32);
impl_typed_value!(f64, Double, |v| v);
impl_typed_value!(i32, Int, |v| v);
impl_typed_value!(&bool, Bool, |v: &bool| *v as i32);
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
