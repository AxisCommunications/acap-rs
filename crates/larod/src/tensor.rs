use std::{
    ffi::CStr,
    marker::PhantomData,
    ops::Deref,
    os::{
        fd::{BorrowedFd, RawFd},
        raw::c_int,
    },
};

pub use larod_sys::{larodTensorDataType, larodTensorDims, larodTensorLayout, larodTensorPitches};

use crate::{connection::Connection, Error};

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
    conn_raw: *mut larod_sys::larodConnection,
    _conn: PhantomData<&'conn Connection>,
}

impl<'conn> Tensors<'conn> {
    pub(crate) fn from_raw(
        raw: *mut *mut larod_sys::larodTensor,
        len: usize,
        conn: &'conn Connection,
    ) -> Result<Self, Error> {
        if raw.is_null() {
            return Err(Error::NullPointer);
        }

        // TensorMut relies on each element being a distinct, non-null tensor.
        // larod.h promises that, so a violation means larod is broken. The
        // array is then leaked rather than destroyed, since destroying an array
        // that repeats a tensor would free it twice.
        for i in 0..len {
            // SAFETY: larod returned an array of len tensor pointers.
            let tensor = unsafe { *raw.add(i) };
            if tensor.is_null() {
                return Err(Error::NullPointer);
            }
            for j in 0..i {
                // SAFETY: j < i < len.
                let prior = unsafe { *raw.add(j) };
                if prior == tensor {
                    return Err(Error::DuplicateTensor);
                }
            }
        }

        Ok(Self {
            raw,
            len,
            conn_raw: conn.raw,
            _conn: PhantomData,
        })
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
        Some(TensorRef {
            raw: ptr,
            _tensors: PhantomData,
        })
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
        Some(TensorMut::from_raw(ptr))
    }

    /// Returns the raw pointer array for passing to C functions like
    /// `larodCreateJobRequest`.
    pub(crate) fn as_ptr(&self) -> *mut *mut larod_sys::larodTensor {
        self.raw
    }

    /// Returns an iterator over immutable tensor references.
    pub fn iter(&self) -> TensorsIter<'_> {
        TensorsIter {
            raw: self.raw,
            len: self.len,
            index: 0,
            _marker: PhantomData,
        }
    }

    /// Returns an iterator over mutable tensor references.
    pub fn iter_mut(&mut self) -> TensorsIterMut<'_> {
        TensorsIterMut {
            raw: self.raw,
            len: self.len,
            index: 0,
            _marker: PhantomData,
        }
    }
}

impl<'a> IntoIterator for &'a Tensors<'_> {
    type Item = TensorRef<'a>;
    type IntoIter = TensorsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut Tensors<'_> {
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
        Some(TensorRef {
            raw: ptr,
            _tensors: PhantomData,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len - self.index;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for TensorsIter<'_> {}
impl std::iter::FusedIterator for TensorsIter<'_> {}

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
        Some(TensorMut::from_raw(ptr))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len - self.index;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for TensorsIterMut<'_> {}
impl std::iter::FusedIterator for TensorsIterMut<'_> {}

impl std::fmt::Debug for Tensors<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tensors")
            .field("len", &self.len)
            .field("raw", &self.raw)
            .finish()
    }
}

impl Drop for Tensors<'_> {
    fn drop(&mut self) {
        // larodDestroyTensors takes *mut *mut *mut larodTensor (triple pointer).
        // We pass &mut self.raw which gives *mut (*mut *mut larodTensor).
        let (success, maybe_error) = crate::with_larod(|| {
            let mut error: *mut larod_sys::larodError = std::ptr::null_mut();
            // SAFETY: self.raw holds self.len tensors that are released
            // exactly once, here. self.conn_raw outlives self via 'conn; it is
            // used only for tensors allocated or tracked by the service
            // (larod.h, "Destroy a list of larodTensor").
            let success = unsafe {
                larod_sys::larodDestroyTensors(self.conn_raw, &mut self.raw, self.len, &mut error)
            };
            let maybe_error = if error.is_null() {
                None
            } else {
                // SAFETY: error was allocated by larodDestroyTensors and the
                // process-wide larod lock is held by with_larod.
                Some(unsafe { crate::LarodError::from_raw_locked(error) })
            };
            (success, maybe_error)
        });
        if let Some(err) = maybe_error {
            if !success {
                log::error!("Failed to destroy tensors: {err}");
            }
        } else if !success {
            log::error!("Failed to destroy tensors (no error details)");
        }
    }
}

/// Immutable reference to a single tensor within a [`Tensors`] array.
// Invariant: raw is an element of a Tensors array that outlives 'a.
//
// Not Clone or Copy: TensorMut derefs to a TensorRef, so a copy would outlive
// the deref borrow and alias the mutable tensor.
pub struct TensorRef<'a> {
    raw: *mut larod_sys::larodTensor,
    _tensors: PhantomData<&'a ()>,
}

impl TensorRef<'_> {
    pub fn dims(&self) -> Result<&larodTensorDims, Error> {
        // SAFETY: self.raw is a valid tensor (type invariant).
        let (ptr, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetTensorDims, self.raw as *const _) };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        // SAFETY: ptr is non-null and valid as long as the tensor is (larod.h,
        // "Get dimensions of a tensor"), which outlives the borrow of &self.
        Ok(unsafe { &*ptr })
    }

    pub fn pitches(&self) -> Result<&larodTensorPitches, Error> {
        // SAFETY: self.raw is a valid tensor (type invariant).
        let (ptr, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetTensorPitches, self.raw as *const _) };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        // SAFETY: ptr is non-null and valid as long as the tensor is (larod.h,
        // "Get pitches of a tensor"), which outlives the borrow of &self.
        Ok(unsafe { &*ptr })
    }

    pub fn data_type(&self) -> Result<larodTensorDataType, Error> {
        // SAFETY: self.raw is a valid tensor (type invariant).
        let (dt, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetTensorDataType, self.raw as *const _) };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        if dt == larodTensorDataType::LAROD_TENSOR_DATA_TYPE_INVALID {
            return Err(Error::MissingError);
        }
        Ok(dt)
    }

    pub fn layout(&self) -> Result<larodTensorLayout, Error> {
        // SAFETY: self.raw is a valid tensor (type invariant).
        let (layout, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetTensorLayout, self.raw as *const _) };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        if layout == larodTensorLayout::LAROD_TENSOR_LAYOUT_INVALID {
            return Err(Error::MissingError);
        }
        Ok(layout)
    }

    /// Returns the tensor's file descriptor, or `None` if none is assigned.
    ///
    /// Tensors created from a model start out with fd `-1` until one is
    /// assigned with [`TensorMut::set_fd`] (larod.h, "Create input tensors from
    /// a model"). larod.h does not say what `larodGetTensorFd` returns for such
    /// a tensor; it returns `-1` (tested on AXIS P3268-LV, AXIS OS 12.11.77).
    /// Tensors allocated by the service always have an fd. `LAROD_INVALID_FD`
    /// is reported as an error.
    pub fn fd(&self) -> Result<Option<BorrowedFd<'_>>, Error> {
        // SAFETY: self.raw is a valid tensor (type invariant).
        let (fd, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetTensorFd, self.raw as *const _) };
        let fd = decode_fd(fd, maybe_error)?;
        // SAFETY: The fd is either allocated by the service and closed by
        // larodDestroyTensors, which cannot run while this borrow is alive, or
        // it was set with the unsafe `set_fd`, whose caller promised to keep it
        // open for as long as the tensor exists.
        Ok(fd.map(|fd| unsafe { BorrowedFd::borrow_raw(fd) }))
    }

    pub fn fd_size(&self) -> Result<usize, Error> {
        let mut size: usize = 0;
        // SAFETY: self.raw is a valid tensor (type invariant).
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
        // SAFETY: self.raw is a valid tensor (type invariant).
        let (offset, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetTensorFdOffset, self.raw as *const _) };
        if let Some(err) = maybe_error {
            return Err(err);
        }
        // -1 signals an error (larod.h, "Get file offset for a tensor").
        if offset == -1 {
            return Err(Error::MissingError);
        }
        Ok(offset)
    }

    pub fn name(&self) -> Result<&CStr, Error> {
        // SAFETY: self.raw is a valid tensor (type invariant).
        let (ptr, maybe_error) =
            unsafe { try_func!(larod_sys::larodGetTensorName, self.raw as *const _) };
        if ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::NullPointer));
        }
        // SAFETY: ptr is a non-null, NUL-terminated string valid as long as the
        // tensor is (larod.h, "Get name of tensor"), which outlives the borrow
        // of &self.
        Ok(unsafe { CStr::from_ptr(ptr) })
    }

    pub fn byte_size(&self) -> Result<usize, Error> {
        let mut size: usize = 0;
        // SAFETY: self.raw is a valid tensor (type invariant).
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
///
/// Read accessors are available through [`Deref`] to [`TensorRef`].
// Invariant: as for TensorRef, and no other TensorRef or TensorMut for the same
// tensor exists for 'a (enforced by &mut self on get_mut and exclusive
// iteration).
pub struct TensorMut<'a> {
    inner: TensorRef<'a>,
    _mut: PhantomData<&'a mut ()>,
}

impl<'a> Deref for TensorMut<'a> {
    type Target = TensorRef<'a>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl TensorMut<'_> {
    fn from_raw(raw: *mut larod_sys::larodTensor) -> Self {
        Self {
            inner: TensorRef {
                raw,
                _tensors: PhantomData,
            },
            _mut: PhantomData,
        }
    }

    fn raw(&mut self) -> *mut larod_sys::larodTensor {
        self.inner.raw
    }

    pub fn set_dims(&mut self, dims: &larodTensorDims) -> Result<(), Error> {
        // SAFETY: The tensor is valid and exclusively borrowed (type invariant).
        let (success, maybe_error) =
            unsafe { try_func!(larod_sys::larodSetTensorDims, self.raw(), dims) };
        if success {
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub fn set_pitches(&mut self, pitches: &larodTensorPitches) -> Result<(), Error> {
        // SAFETY: The tensor is valid and exclusively borrowed (type invariant).
        let (success, maybe_error) =
            unsafe { try_func!(larod_sys::larodSetTensorPitches, self.raw(), pitches) };
        if success {
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub fn set_data_type(&mut self, data_type: larodTensorDataType) -> Result<(), Error> {
        // SAFETY: The tensor is valid and exclusively borrowed (type invariant).
        let (success, maybe_error) =
            unsafe { try_func!(larod_sys::larodSetTensorDataType, self.raw(), data_type) };
        if success {
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub fn set_layout(&mut self, layout: larodTensorLayout) -> Result<(), Error> {
        // SAFETY: The tensor is valid and exclusively borrowed (type invariant).
        let (success, maybe_error) =
            unsafe { try_func!(larod_sys::larodSetTensorLayout, self.raw(), layout) };
        if success {
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    /// Set the tensor's file descriptor.
    ///
    /// larod does not take ownership of `fd`.
    ///
    /// # Safety
    ///
    /// - `fd` must stay open for as long as this tensor exists or until another
    ///   fd is set, since [`TensorRef::fd`] and job requests use it.
    /// - `fd` must match the tensor's fd properties. This crate does not expose
    ///   `larodSetTensorFdProps`, so tensors keep their default
    ///   `LAROD_FD_PROP_READWRITE`, i.e. `fd` must support `read` and `write`.
    ///   Invalid properties "could result in undefined behavior" (larod.h,
    ///   "Flags for specifying how one can access a fd of a larodTensor").
    pub unsafe fn set_fd(&mut self, fd: c_int) -> Result<(), Error> {
        if fd < 0 {
            return Err(Error::InvalidFd(fd));
        }
        // SAFETY: The tensor is valid and exclusively borrowed (type invariant).
        let (success, maybe_error) =
            unsafe { try_func!(larod_sys::larodSetTensorFd, self.raw(), fd) };
        if success {
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub fn set_fd_size(&mut self, size: usize) -> Result<(), Error> {
        // SAFETY: The tensor is valid and exclusively borrowed (type invariant).
        let (success, maybe_error) =
            unsafe { try_func!(larod_sys::larodSetTensorFdSize, self.raw(), size) };
        if success {
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }

    pub fn set_fd_offset(&mut self, offset: i64) -> Result<(), Error> {
        // SAFETY: The tensor is valid and exclusively borrowed (type invariant).
        let (success, maybe_error) =
            unsafe { try_func!(larod_sys::larodSetTensorFdOffset, self.raw(), offset) };
        if success {
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingError))
        }
    }
}

/// `#define LAROD_INVALID_FD (INT_MIN)` in larod.h; bindgen does not emit it.
const LAROD_INVALID_FD: c_int = c_int::MIN;

/// Decode the return value of `larodGetTensorFd`.
///
/// `LAROD_INVALID_FD` (`INT_MIN`) is the documented failure sentinel and larod
/// normally populates an error object alongside it. `-1` is the default for a
/// tensor without an fd (larod.h, "Create input tensors from a model"). Any
/// other negative value is not a valid fd and is reported as such.
fn decode_fd(fd: c_int, maybe_error: Option<Error>) -> Result<Option<RawFd>, Error> {
    if let Some(err) = maybe_error {
        return Err(err);
    }
    match fd {
        LAROD_INVALID_FD => Err(Error::MissingError),
        -1 => Ok(None),
        fd if fd < 0 => Err(Error::InvalidFd(fd)),
        fd => Ok(Some(fd)),
    }
}

#[cfg(test)]
mod tests {
    use std::os::raw::c_int;

    use crate::Error;

    #[test]
    fn invalid_fd_sentinel_without_error_is_missing_error() {
        assert!(matches!(
            super::decode_fd(c_int::MIN, None),
            Err(Error::MissingError)
        ));
    }

    #[test]
    fn invalid_fd_sentinel_with_error_reports_that_error() {
        assert!(matches!(
            super::decode_fd(c_int::MIN, Some(Error::NullPointer)),
            Err(Error::NullPointer)
        ));
    }

    #[test]
    fn unassigned_fd_is_none() {
        assert!(matches!(super::decode_fd(-1, None), Ok(None)));
    }

    #[test]
    fn other_negative_fd_is_invalid_fd() {
        assert!(matches!(
            super::decode_fd(-2, None),
            Err(Error::InvalidFd(-2))
        ));
    }

    #[test]
    fn non_negative_fd_is_valid() {
        assert!(matches!(super::decode_fd(0, None), Ok(Some(0))));
    }
}
