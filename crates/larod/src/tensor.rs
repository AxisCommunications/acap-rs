use std::ffi::CStr;
use std::marker::PhantomData;
use std::os::raw::c_int;

use crate::connection::Connection;
use crate::Error;

pub use larod_sys::{larodTensorDataType, larodTensorDims, larodTensorLayout, larodTensorPitches};

/// An owned array of tensor descriptors.
///
/// Tensors in larod are always created and destroyed as arrays. This wrapper
/// owns the entire C-allocated array and provides indexed access to individual
/// tensors.
///
/// Requires a connection because `larodDestroyTensors` needs it to release
/// any server-tracked file descriptors.
pub struct Tensors<'conn> {
    raw: *mut *mut larod_sys::larodTensor,
    len: usize,
    // Raw pointer (not &Connection) so Send doesn't require Connection: Sync.
    conn_raw: *mut larod_sys::larodConnection,
    _conn: PhantomData<&'conn Connection>,
}

impl<'conn> Tensors<'conn> {
    /// # Safety
    ///
    /// `raw` must be a valid tensor array pointer returned by larod, with
    /// `len` elements. `conn` must be the connection that created or will
    /// manage these tensors.
    pub(crate) unsafe fn from_raw(
        raw: *mut *mut larod_sys::larodTensor,
        len: usize,
        conn: &'conn Connection,
    ) -> Self {
        Self { raw, len, conn_raw: conn.raw, _conn: PhantomData }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Access a single tensor by index.
    pub fn get(&self, index: usize) -> Option<TensorRef<'_>> {
        if index >= self.len {
            return None;
        }
        // SAFETY: index is in bounds. The tensor pointer is valid for the
        // lifetime of this Tensors array.
        let ptr = unsafe { *self.raw.add(index) };
        Some(TensorRef { raw: ptr, _tensors: PhantomData })
    }

    /// Access a single mutable tensor by index.
    pub fn get_mut(&mut self, index: usize) -> Option<TensorMut<'_>> {
        if index >= self.len {
            return None;
        }
        // SAFETY: index is in bounds (checked above). The pointer is valid for
        // the lifetime of this Tensors array. The &mut self borrow prevents
        // concurrent access to any other tensor in the array.
        let ptr = unsafe { *self.raw.add(index) };
        Some(TensorMut { raw: ptr, _tensors: PhantomData })
    }

    /// Returns the raw pointer array for passing to C functions like
    /// `larodCreateJobRequest`.
    pub(crate) fn as_ptr(&self) -> *mut *mut larod_sys::larodTensor {
        self.raw
    }

    /// Returns an iterator over immutable tensor references.
    pub fn iter(&self) -> TensorsIter<'_> {
        TensorsIter { raw: self.raw, len: self.len, index: 0, _marker: PhantomData }
    }

    /// Returns an iterator over mutable tensor references.
    pub fn iter_mut(&mut self) -> TensorsIterMut<'_> {
        TensorsIterMut { raw: self.raw, len: self.len, index: 0, _marker: PhantomData }
    }
}

impl<'a, 'conn> IntoIterator for &'a Tensors<'conn> {
    type Item = TensorRef<'a>;
    type IntoIter = TensorsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, 'conn> IntoIterator for &'a mut Tensors<'conn> {
    type Item = TensorMut<'a>;
    type IntoIter = TensorsIterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// Iterator over immutable tensor references.
pub struct TensorsIter<'a> {
    raw: *mut *mut larod_sys::larodTensor,
    len: usize,
    index: usize,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Iterator for TensorsIter<'a> {
    type Item = TensorRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.len {
            return None;
        }
        // SAFETY: index is in-bounds (checked above). The pointer is valid for
        // the lifetime of this iterator which borrows from Tensors.
        let ptr = unsafe { *self.raw.add(self.index) };
        self.index += 1;
        Some(TensorRef { raw: ptr, _tensors: PhantomData })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len - self.index;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for TensorsIter<'_> {}
impl std::iter::FusedIterator for TensorsIter<'_> {}

// SAFETY: TensorsIter holds only a raw pointer into a Send tensor array.
// The lifetime 'a ensures the source Tensors (and its connection) outlives
// this iterator.
unsafe impl Send for TensorsIter<'_> {}

/// Iterator over mutable tensor references.
pub struct TensorsIterMut<'a> {
    raw: *mut *mut larod_sys::larodTensor,
    len: usize,
    index: usize,
    _marker: PhantomData<&'a mut ()>,
}

impl<'a> Iterator for TensorsIterMut<'a> {
    type Item = TensorMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.len {
            return None;
        }
        // SAFETY: Each index is yielded exactly once, so we don't create
        // overlapping mutable references. The exclusive &mut Tensors borrow
        // from iter_mut() prevents any concurrent access through the original.
        let ptr = unsafe { *self.raw.add(self.index) };
        self.index += 1;
        Some(TensorMut { raw: ptr, _tensors: PhantomData })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len - self.index;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for TensorsIterMut<'_> {}
impl std::iter::FusedIterator for TensorsIterMut<'_> {}

// SAFETY: TensorsIterMut holds only a raw pointer into a Send tensor array.
// The lifetime 'a and exclusive &mut borrow ensure no concurrent access.
unsafe impl Send for TensorsIterMut<'_> {}

impl std::fmt::Debug for Tensors<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tensors")
            .field("len", &self.len)
            .field("raw", &self.raw)
            .finish()
    }
}

// SAFETY: Exclusive ownership of tensor array. No &Connection held (only
// conn_raw), so this does not require Connection: Sync.
unsafe impl Send for Tensors<'_> {}

impl Drop for Tensors<'_> {
    fn drop(&mut self) {
        // larodDestroyTensors takes *mut *mut *mut larodTensor (triple pointer).
        // We pass &mut self.raw which gives *mut (*mut *mut larodTensor).
        let mut error: *mut larod_sys::larodError = std::ptr::null_mut();
        let success = unsafe {
            larod_sys::larodDestroyTensors(
                self.conn_raw,
                &mut self.raw,
                self.len,
                &mut error,
            )
        };
        if !error.is_null() {
            let err = crate::LarodError::from_raw(error);
            if !success {
                log::error!("Failed to destroy tensors: {err}");
            }
        } else if !success {
            log::error!("Failed to destroy tensors (no error details)");
        }
    }
}

/// Immutable reference to a single tensor within a [`Tensors`] array.
pub struct TensorRef<'a> {
    raw: *mut larod_sys::larodTensor,
    _tensors: PhantomData<&'a ()>,
}

impl<'a> TensorRef<'a> {

    pub fn dims(&self) -> Result<&larodTensorDims, Error> {
        let (ptr, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodGetTensorDims,
                self.raw as *const _,
            )
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        // SAFETY: ptr is non-null and points into the tensor's internal storage.
        Ok(unsafe { &*ptr })
    }

    pub fn pitches(&self) -> Result<&larodTensorPitches, Error> {
        let (ptr, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodGetTensorPitches,
                self.raw as *const _,
            )
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        Ok(unsafe { &*ptr })
    }

    pub fn data_type(&self) -> Result<larodTensorDataType, Error> {
        let (dt, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodGetTensorDataType,
                self.raw as *const _,
            )
        };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        Ok(dt)
    }

    pub fn layout(&self) -> Result<larodTensorLayout, Error> {
        let (layout, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodGetTensorLayout,
                self.raw as *const _,
            )
        };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        Ok(layout)
    }

    /// Returns the tensor's file descriptor, or `None` if no fd has been assigned.
    pub fn fd(&self) -> Result<Option<c_int>, Error> {
        let (fd, maybe_error) = unsafe {
            try_func!(larod_sys::larodGetTensorFd, self.raw as *const _)
        };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        if fd == -1 { Ok(None) } else { Ok(Some(fd)) }
    }

    pub fn fd_size(&self) -> Result<usize, Error> {
        let mut size: usize = 0;
        let (success, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodGetTensorFdSize,
                self.raw as *const _,
                &mut size,
            )
        };
        if success {
            Ok(size)
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub fn fd_offset(&self) -> Result<i64, Error> {
        let (offset, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodGetTensorFdOffset,
                self.raw as *const _,
            )
        };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        Ok(offset)
    }

    pub fn name(&self) -> Result<&CStr, Error> {
        let (ptr, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodGetTensorName,
                self.raw as *const _,
            )
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        // SAFETY: ptr is non-null and points into the tensor's internal storage.
        // Lifetime is tied to 'a via the PhantomData borrow from the Tensors array.
        Ok(unsafe { CStr::from_ptr(ptr) })
    }

    pub fn byte_size(&self) -> Result<usize, Error> {
        let mut size: usize = 0;
        let (success, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodGetTensorByteSize,
                self.raw as *const _,
                &mut size,
            )
        };
        if success {
            Ok(size)
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }
}

/// Mutable reference to a single tensor within a [`Tensors`] array.
// Invariant: raw pointer is from a valid Tensors element, Tensors outlives
// this via 'a, and no aliasing TensorMut exists (enforced by &mut self on
// get_mut and exclusive iteration).
pub struct TensorMut<'a> {
    raw: *mut larod_sys::larodTensor,
    _tensors: PhantomData<&'a mut ()>,
}

impl<'a> TensorMut<'a> {

    pub fn set_dims(&mut self, dims: &larodTensorDims) -> Result<(), Error> {
        let (success, maybe_error) = unsafe {
            try_func!(larod_sys::larodSetTensorDims, self.raw, dims)
        };
        if success {
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub fn set_pitches(&mut self, pitches: &larodTensorPitches) -> Result<(), Error> {
        let (success, maybe_error) = unsafe {
            try_func!(larod_sys::larodSetTensorPitches, self.raw, pitches)
        };
        if success {
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub fn set_data_type(&mut self, data_type: larodTensorDataType) -> Result<(), Error> {
        let (success, maybe_error) = unsafe {
            try_func!(larod_sys::larodSetTensorDataType, self.raw, data_type)
        };
        if success {
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub fn set_layout(&mut self, layout: larodTensorLayout) -> Result<(), Error> {
        let (success, maybe_error) = unsafe {
            try_func!(larod_sys::larodSetTensorLayout, self.raw, layout)
        };
        if success {
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub fn set_fd(&mut self, fd: c_int) -> Result<(), Error> {
        let (success, maybe_error) = unsafe {
            try_func!(larod_sys::larodSetTensorFd, self.raw, fd)
        };
        if success {
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub fn set_fd_size(&mut self, size: usize) -> Result<(), Error> {
        let (success, maybe_error) = unsafe {
            try_func!(larod_sys::larodSetTensorFdSize, self.raw, size)
        };
        if success {
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub fn set_fd_offset(&mut self, offset: i64) -> Result<(), Error> {
        let (success, maybe_error) = unsafe {
            try_func!(larod_sys::larodSetTensorFdOffset, self.raw, offset)
        };
        if success {
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    // Read-only accessors (delegate to the same C functions as TensorRef).
    pub fn dims(&self) -> Result<&larodTensorDims, Error> {
        let (ptr, maybe_error) = unsafe {
            try_func!(larod_sys::larodGetTensorDims, self.raw as *const _)
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        Ok(unsafe { &*ptr })
    }

    pub fn pitches(&self) -> Result<&larodTensorPitches, Error> {
        let (ptr, maybe_error) = unsafe {
            try_func!(larod_sys::larodGetTensorPitches, self.raw as *const _)
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        Ok(unsafe { &*ptr })
    }

    pub fn data_type(&self) -> Result<larodTensorDataType, Error> {
        let (dt, maybe_error) = unsafe {
            try_func!(larod_sys::larodGetTensorDataType, self.raw as *const _)
        };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        Ok(dt)
    }

    pub fn layout(&self) -> Result<larodTensorLayout, Error> {
        let (layout, maybe_error) = unsafe {
            try_func!(larod_sys::larodGetTensorLayout, self.raw as *const _)
        };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        Ok(layout)
    }

    /// Returns the tensor's file descriptor, or `None` if no fd has been assigned.
    pub fn fd(&self) -> Result<Option<c_int>, Error> {
        let (fd, maybe_error) = unsafe {
            try_func!(larod_sys::larodGetTensorFd, self.raw as *const _)
        };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        if fd == -1 { Ok(None) } else { Ok(Some(fd)) }
    }

    pub fn fd_size(&self) -> Result<usize, Error> {
        let mut size: usize = 0;
        let (success, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodGetTensorFdSize,
                self.raw as *const _,
                &mut size,
            )
        };
        if success {
            Ok(size)
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub fn fd_offset(&self) -> Result<i64, Error> {
        let (offset, maybe_error) = unsafe {
            try_func!(larod_sys::larodGetTensorFdOffset, self.raw as *const _)
        };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        Ok(offset)
    }

    pub fn byte_size(&self) -> Result<usize, Error> {
        let mut size: usize = 0;
        let (success, maybe_error) = unsafe {
            try_func!(
                larod_sys::larodGetTensorByteSize,
                self.raw as *const _,
                &mut size,
            )
        };
        if success {
            Ok(size)
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub fn name(&self) -> Result<&CStr, Error> {
        let (ptr, maybe_error) = unsafe {
            try_func!(larod_sys::larodGetTensorName, self.raw as *const _)
        };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        // SAFETY: ptr is non-null and points into the tensor's internal storage.
        // Lifetime is tied to 'a via the PhantomData borrow from the Tensors array.
        Ok(unsafe { CStr::from_ptr(ptr) })
    }
}
