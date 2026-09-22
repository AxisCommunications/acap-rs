//! Key-value map for VDO settings and a GLib-allocated C string type.

use std::{
    ffi::{c_char, c_void, CStr},
    fmt,
    ops::Deref,
    ptr::{self, NonNull},
};

use glib::translate::{from_glib, IntoGlib};
use gobject_sys::{g_object_unref, GObject};
use vdo_sys::{VdoMap, VdoQuad32i, VdoQuad32u};

/// Four signed 32-bit integers, stored in a [`Map`] under one key.
///
/// VDO uses this for values such as crop rectangles, where the fields are
/// interpreted as `x`, `y`, `w`, `h`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Quad32i {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Quad32i {
    fn into_raw(self) -> VdoQuad32i {
        // The union stores four 32-bit words; `as u32` is bit-preserving.
        VdoQuad32i {
            __bindgen_anon_1: Default::default(),
            val: Default::default(),
            bindgen_union_field: [self.x as u32, self.y as u32, self.w as u32, self.h as u32],
        }
    }

    fn from_raw(raw: VdoQuad32i) -> Self {
        let [x, y, w, h] = raw.bindgen_union_field;
        Self {
            x: x as i32,
            y: y as i32,
            w: w as i32,
            h: h as i32,
        }
    }
}

/// Four unsigned 32-bit integers, stored in a [`Map`] under one key.
///
/// VDO interprets the fields either as a rectangle (`x`, `y`, `w`, `h`) or,
/// for some keys, as a range (`min`, `target`, `max`) occupying the first
/// three fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Quad32u {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Quad32u {
    fn into_raw(self) -> VdoQuad32u {
        VdoQuad32u {
            __bindgen_anon_1: Default::default(),
            __bindgen_anon_2: Default::default(),
            val: Default::default(),
            bindgen_union_field: [self.x, self.y, self.w, self.h],
        }
    }

    fn from_raw(raw: VdoQuad32u) -> Self {
        let [x, y, w, h] = raw.bindgen_union_field;
        Self { x, y, w, h }
    }
}

/// An owned pointer to a C string allocated by GLib.
///
/// The string is freed with `g_free` when dropped.
#[repr(transparent)]
pub struct CStringPtr(NonNull<c_char>);

impl CStringPtr {
    /// # Safety
    ///
    /// The memory must satisfy the preconditions for [`CStr::from_ptr`], must have been
    /// allocated in a manner compatible with [`glib_sys::g_free`], and there must be no other
    /// users of this memory.
    ///
    /// # Panics
    ///
    /// Panics if `ptr` is null.
    pub(crate) unsafe fn from_ptr(ptr: *mut c_char) -> Self {
        Self(NonNull::new(ptr).expect("CStringPtr::from_ptr called with null"))
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
    pub fn new() -> Self {
        // `vdo_map_new` is a thin wrapper around `g_object_new`, which aborts
        // the program if allocation fails, so the returned pointer is never null.
        let map = unsafe { vdo_sys::vdo_map_new() };
        Self { raw: map }
    }

    /// # Safety
    ///
    /// `ptr` must be a valid `VdoMap` pointer with ownership
    /// transferred to this `Map` (it will be unreferenced on drop).
    ///
    /// # Panics
    ///
    /// Panics if `ptr` is null.
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

    pub fn set_i32(&mut self, key: &CStr, value: i32) {
        unsafe { vdo_sys::vdo_map_set_int32(self.raw, key.as_ptr(), value) }
    }

    pub fn get_i32(&self, key: &CStr, default: i32) -> i32 {
        unsafe { vdo_sys::vdo_map_get_int32(self.raw, key.as_ptr(), default) }
    }

    pub fn set_string(&mut self, key: &CStr, value: &CStr) {
        unsafe { vdo_sys::vdo_map_set_string(self.raw, key.as_ptr(), value.as_ptr()) }
    }

    /// Returns `None` if the key doesn't exist or the value is null.
    pub fn get_string(&self, key: &CStr) -> Option<CStringPtr> {
        // Passing null as default so missing keys yield null -> None.
        let ptr =
            unsafe { vdo_sys::vdo_map_dup_string(self.raw, key.as_ptr(), ptr::null::<c_char>()) };
        if ptr.is_null() {
            return None;
        }
        // SAFETY: ptr is non-null, allocated by g_malloc via vdo_map_dup_string, and we own it.
        Some(unsafe { CStringPtr::from_ptr(ptr) })
    }

    pub fn set_bool(&mut self, key: &CStr, value: bool) {
        unsafe { vdo_sys::vdo_map_set_boolean(self.raw, key.as_ptr(), value.into_glib()) }
    }

    pub fn get_bool(&self, key: &CStr, default: bool) -> bool {
        unsafe {
            from_glib(vdo_sys::vdo_map_get_boolean(
                self.raw,
                key.as_ptr(),
                default.into_glib(),
            ))
        }
    }

    pub fn set_quad32i(&mut self, key: &CStr, value: Quad32i) {
        unsafe { vdo_sys::vdo_map_set_quad32i(self.raw, key.as_ptr(), value.into_raw()) }
    }

    pub fn get_quad32i(&self, key: &CStr, default: Quad32i) -> Quad32i {
        Quad32i::from_raw(unsafe {
            vdo_sys::vdo_map_get_quad32i(self.raw, key.as_ptr(), default.into_raw())
        })
    }

    pub fn set_quad32u(&mut self, key: &CStr, value: Quad32u) {
        unsafe { vdo_sys::vdo_map_set_quad32u(self.raw, key.as_ptr(), value.into_raw()) }
    }

    pub fn get_quad32u(&self, key: &CStr, default: Quad32u) -> Quad32u {
        Quad32u::from_raw(unsafe {
            vdo_sys::vdo_map_get_quad32u(self.raw, key.as_ptr(), default.into_raw())
        })
    }

    /// Dumps the map contents to stdout. Intended for debugging only;
    /// may expose sensitive configuration values in production logs.
    pub fn dump(&self) {
        unsafe { vdo_sys::vdo_map_dump(self.raw) }
    }

    // Returns *mut because GLib's C API takes *mut even for read-only operations.
    pub(crate) fn as_ptr(&self) -> *mut VdoMap {
        self.raw
    }
}

impl Clone for Map {
    /// Creates a deep copy of the map.
    fn clone(&self) -> Self {
        // `vdo_map_clone` allocates through GObject, which aborts on allocation
        // failure, so the returned pointer is never null. Ownership is
        // transferred to the caller.
        let raw = unsafe { vdo_sys::vdo_map_clone(self.raw) };
        // SAFETY: raw is a valid, owned VdoMap.
        unsafe { Self::from_raw(raw) }
    }
}

impl Default for Map {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Map {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Map").field("raw", &self.raw).finish()
    }
}

// SAFETY: We hold exclusive ownership of the raw pointer and VdoMap
// does not require access from a specific thread.
unsafe impl Send for Map {}

impl Drop for Map {
    fn drop(&mut self) {
        unsafe { g_object_unref(self.raw as *mut GObject) }
    }
}
