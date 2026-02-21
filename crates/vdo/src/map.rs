//! Key-value map for VDO settings and a GLib-allocated C string type.

use gobject_sys::{g_object_unref, GObject};
use std::ffi::{c_char, c_void, CStr};
use std::fmt;
use std::ops::Deref;
use std::ptr::{self, NonNull};
use vdo_sys::VdoMap;

/// An owned pointer to a C string allocated by GLib.
///
/// The string is freed with `g_free` when dropped.
#[repr(transparent)]
pub struct CStringPtr(NonNull<c_char>);

impl CStringPtr {
    /// # Safety
    ///
    /// In addition to the safety preconditions for [`CStr::from_ptr`] the memory must have been
    /// allocated in a manner compatible with [`glib_sys::g_free`] and there must be no other
    /// users of this memory.
    pub(crate) unsafe fn from_ptr(ptr: *mut c_char) -> Self {
        assert!(!ptr.is_null(), "CStringPtr::from_ptr called with null");
        Self(NonNull::new_unchecked(ptr))
    }

    pub fn as_c_str(&self) -> &CStr {
        // SAFETY: The preconditions for instantiating this type include all preconditions
        // for `CStr::from_ptr`.
        unsafe { CStr::from_ptr(self.0.as_ptr() as *const c_char) }
    }
}

impl Deref for CStringPtr {
    type Target = CStr;

    fn deref(&self) -> &CStr {
        self.as_c_str()
    }
}

impl fmt::Debug for CStringPtr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_c_str())
    }
}

impl Drop for CStringPtr {
    fn drop(&mut self) {
        // SAFETY: We have full ownership, allocated in a manner compatible with `g_free`.
        unsafe {
            glib_sys::g_free(self.0.as_ptr() as *mut c_void);
        }
    }
}

/// A key-value map for VDO settings.
///
/// Used to configure stream parameters and retrieve stream information.
/// All methods assume `self.raw` is a valid `VdoMap` pointer, which is
/// guaranteed by the constructors.
pub struct Map {
    raw: *mut VdoMap,
}

impl Map {
    pub fn try_new() -> std::result::Result<Self, super::Error> {
        let map = unsafe { vdo_sys::vdo_map_new() };
        if map.is_null() {
            Err(super::Error::NullPointer)
        } else {
            Ok(Self { raw: map })
        }
    }

    /// # Safety
    ///
    /// `ptr` must be a non-null, valid `VdoMap` pointer with ownership
    /// transferred to this `Map` (it will be unreferenced on drop).
    pub(crate) unsafe fn from_raw(ptr: *mut VdoMap) -> Self {
        assert!(!ptr.is_null(), "Map::from_raw called with null");
        Self { raw: ptr }
    }

    pub fn set_u32(&mut self, key: &CStr, value: u32) {
        unsafe { vdo_sys::vdo_map_set_uint32(self.raw, key.as_ptr(), value) }
    }

    pub fn get_u32(&self, key: &CStr, default: u32) -> u32 {
        unsafe { vdo_sys::vdo_map_get_uint32(self.raw, key.as_ptr(), default) }
    }

    pub fn set_string(&mut self, key: &CStr, value: &CStr) {
        unsafe { vdo_sys::vdo_map_set_string(self.raw, key.as_ptr(), value.as_ptr()) }
    }

    /// Returns `None` if the key doesn't exist or the value is null.
    pub fn get_string(&self, key: &CStr) -> Option<CStringPtr> {
        // Passing null as default so missing keys yield null -> None.
        let ptr = unsafe { vdo_sys::vdo_map_dup_string(self.raw, key.as_ptr(), ptr::null::<c_char>()) };
        if ptr.is_null() {
            return None;
        }
        // SAFETY: ptr is non-null, allocated by g_malloc via vdo_map_dup_string, and we own it.
        Some(unsafe { CStringPtr::from_ptr(ptr) })
    }

    pub fn set_bool(&mut self, key: &CStr, value: bool) {
        let gvalue = if value {
            glib_sys::GTRUE
        } else {
            glib_sys::GFALSE
        };
        unsafe { vdo_sys::vdo_map_set_boolean(self.raw, key.as_ptr(), gvalue) }
    }

    pub fn get_bool(&self, key: &CStr, default: bool) -> bool {
        let gdefault = if default {
            glib_sys::GTRUE
        } else {
            glib_sys::GFALSE
        };
        unsafe { vdo_sys::vdo_map_get_boolean(self.raw, key.as_ptr(), gdefault) != glib_sys::GFALSE }
    }

    /// Dumps the map contents to stdout (for debugging).
    pub fn dump(&self) {
        unsafe { vdo_sys::vdo_map_dump(self.raw) }
    }

    // Returns *mut because GLib's C API takes *mut even for read-only operations.
    pub(crate) fn as_ptr(&self) -> *mut VdoMap {
        self.raw
    }
}

impl fmt::Debug for Map {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Map").field("raw", &self.raw).finish()
    }
}

// SAFETY: We hold exclusive ownership of the GObject reference.
// Sync is NOT implemented because GLib objects are not safe for concurrent access.
unsafe impl Send for Map {}

impl Drop for Map {
    fn drop(&mut self) {
        unsafe { g_object_unref(self.raw as *mut GObject) }
    }
}
