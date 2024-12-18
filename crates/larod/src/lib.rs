//! A safe warpper around the larod-sys bindings to the larod C library.
//!
//!
//! Example
//! ```rust
//! use larod::Session;
//! let session = Session::new();
//! let devices = session.devices();
//! ```
//!
//! # Gotchas
//! Many of the C functions return either a bool or a pointer to some object.
//! Additionally, one of the out arguments is a pointer to a larodError
//! object. If the normal return type is true, or not NULL in the case of a
//! pointer, the pointer to the larodError struct is expected to be NULL. This
//! represents two potentially conflicting indicators of whether the function
//! succeeded.
//!
//! Crucially, objects pointed to by returned pointers *AND* a non-NULL pointer
//! to a larodError struct need to be dealocated. That is handled appropriately
//! by constructing the LarodError struct if the larodError pointer is non-NULL
//! and the impl Drop for LarodError will dealocate the object appropriately.
//!
//! ## Tensors
//! The larod library supports [creating tensors](https://axiscommunications.github.io/acap-documentation/docs/api/src/api/larod/html/larod_8h.html#aededa9e269d87d0f1b7636a007760cb2).
//! However, it seems that calling that function, as well as [larodCreateModelInputs](https://axiscommunications.github.io/acap-documentation/docs/api/src/api/larod/html/larod_8h.html#adefd8c496e10eddced5be85d93aceb13),
//! allocates some structure on the heap. So, when [larodDestroyTensors](https://axiscommunications.github.io/acap-documentation/docs/api/src/api/larod/html/larod_8h.html#afac99dfef68ffe3d513008aaac354ae0)
//! is called, it deallocates any memory or file descriptors associated with
//! each tensor, but also the container storing the pointers to the tensors.
//! This makes it all but impossible to create a container in Rust storing
//! information about individual tensors and pass something to liblarod to
//! properly deallocate those tensors. This is because C and Rust may use
//! different allocators and objects should be deallocated by the same allocator
//! use for their allocation in the first place.
//!
//! # TODOs:
//! - [ ] [larodDisconnect](https://axiscommunications.github.io/acap-documentation/docs/api/src/api/larod/html/larod_8h.html#ab8f97b4b4d15798384ca25f32ca77bba)
//!     indicates it may fail to "kill a session." What are the implications if it fails to kill a session? Can we clear the sessions?

use core::slice;
pub use larod_sys::larodAccess as LarodAccess;
use larod_sys::*;
use std::{
    ffi::{c_char, CStr, CString},
    fmt::Display,
    fs::File,
    marker::PhantomData,
    os::fd::AsRawFd,
    path::Path,
    ptr::{self, slice_from_raw_parts},
};

type Result<T> = std::result::Result<T, Error>;

macro_rules! try_func {
    ($func:ident $(,)?) => {{
        let mut error: *mut larodError = ptr::null_mut();
        let success = $func(&mut error);
        if error.is_null() {
            (success, None)
        } else {
            (success, Some(Error::LarodError(LarodError{inner: error})))
        }
    }};
    ($func:ident, $($arg:expr),+ $(,)?) => {{
        let mut error: *mut larodError = ptr::null_mut();
        let success = $func($( $arg ),+, &mut error);
        if error.is_null() {
            (success, None)
        } else {
            (success, Some(Error::LarodError(LarodError{inner: error})))
        }

    }}
}

// Most larod functions require a `NULL`` pointer to a larodError AND may
// produce either a `NULL`` output pointer or `false` if an error occurs. This
// results in two potential indicators of whether the function succeeded. If we
// get a`true` output, we expect the larodError to be a pointer to `NULL` still.
// In the possibly rare event that a function succeeds but the larodError
// pointer is not `NULL`, we need to deallocate that memory by calling
// `larodClearError`. The `try_func` macro always checks to see if the
// larodError pointer is `NULL` and return a `LarodError` if not. Doing so will
// call `larodClearError` when it is ultimately dropped.
#[derive(Debug)]
pub struct LarodError {
    inner: *mut larodError,
}

impl LarodError {
    pub fn msg(&self) -> Result<String> {
        if self.inner.is_null() {
            Err(Error::NullLarodPointer)
        } else {
            let msg_slice = unsafe { CStr::from_ptr((*self.inner).msg).to_str() };
            match msg_slice {
                Ok(m) => Ok(m.into()),
                Err(e) => {
                    log::error!("larodError.msg contained invalid UTF-8: {:?}", e);
                    Err(Error::InvalidLarodMessage)
                }
            }
        }
    }

    pub fn code(&self) -> larodErrorCode {
        unsafe { (*self.inner).code }
    }
}

unsafe impl Send for LarodError {}
unsafe impl Sync for LarodError {}

impl std::error::Error for LarodError {}

impl Display for LarodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.msg().unwrap_or("unknown error message".into())
        )
    }
}

impl Drop for LarodError {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { larodClearError(&mut self.inner) }
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    LarodError(#[from] LarodError),
    #[error("liblarod returned an unexpected null pointer")]
    NullLarodPointer,
    #[error("message string returned from liblarod is not valid UTF-8")]
    InvalidLarodMessage,
    #[error("liblarod returned a pointer to invalid data")]
    PointerToInvalidData,
    #[error("could not allocate memory for CString")]
    CStringAllocation,
    #[error("invalid combination of configuration parameters for preprocessor")]
    PreprocessorError(PreProcError),
    #[error("missing error data from liblarod")]
    MissingLarodError,
    #[error(transparent)]
    IOError(std::io::Error),
    #[error("attempted operation without satisfying all required dependencies")]
    UnsatisfiedDependencies,
}

// impl LarodError {
//     /// Convert from liblarod larodError to LarodError
//     /// If larodError is not NULL, it must be dealocated by calling larodClearError
//     fn from(e: *mut larodError) -> Self {
//         if e.is_null() {
//             Self::default()
//         } else {
//             let le = unsafe { *e };
//             let msg: String = unsafe {
//                 CStr::from_ptr(le.msg)
//                     .to_str()
//                     .unwrap_or("Error message invalid")
//                     .into()
//             };
//             let code: LarodErrorCode = le.code.into();
//             // unsafe {
//             //     larodClearError(&mut e);
//             // }
//             Self { msg, code }
//         }
//     }
// }

/// A type representing a larodMap.
pub struct LarodMap {
    raw: *mut larodMap,
}

impl LarodMap {
    /// Create a new larodMap object
    pub fn new() -> Result<Self> {
        let (map, maybe_error): (*mut larodMap, Option<Error>) =
            unsafe { try_func!(larodCreateMap) };
        if !map.is_null() {
            debug_assert!(
                maybe_error.is_none(),
                "larodCreateMap allocated a map AND returned an error!"
            );
            Ok(Self { raw: map })
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    /// Add a string to a larodMap object.
    /// Example
    /// ```rust
    /// use larod::LarodMap;
    ///
    /// let map = LarodMap::new().expect("Error creating map");
    /// map.set_string("key", "value").expect("Error setting string value for larodMap");
    /// ```
    pub fn set_string(&mut self, k: &str, v: &str) -> Result<()> {
        let Ok(key_cstr) = CString::new(k.as_bytes()) else {
            return Err(Error::CStringAllocation);
        };
        let Ok(value_cstr) = CString::new(v.as_bytes()) else {
            return Err(Error::CStringAllocation);
        };
        let (success, maybe_error): (bool, Option<Error>) = unsafe {
            try_func!(
                larodMapSetStr,
                self.raw,
                key_cstr.as_ptr(),
                value_cstr.as_ptr(),
            )
        };
        if success {
            debug_assert!(
                maybe_error.is_none(),
                "larodMapSetStr indicated success AND returned an error!"
            );
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    /// Add an integer to a larodMap object.
    /// Example
    /// ```rust
    /// use larod::LarodMap;
    ///
    /// let map = LarodMap::new().expect("Error creating map");
    /// map.set_int("key", 45).expect("Error setting integer value for larodMap");
    /// ```
    pub fn set_int(&mut self, k: &str, v: i64) -> Result<()> {
        let Ok(key_cstr) = CString::new(k.as_bytes()) else {
            return Err(Error::CStringAllocation);
        };
        let (success, maybe_error): (bool, Option<Error>) =
            unsafe { try_func!(larodMapSetInt, self.raw, key_cstr.as_ptr(), v) };
        if success {
            debug_assert!(
                maybe_error.is_none(),
                "larodMapSetInt indicated success AND returned an error!"
            );
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    /// Add an integer array of two items to a larodMap object.
    /// Example
    /// ```rust
    /// use larod::LarodMap;
    ///
    /// let map = LarodMap::new().expect("Error creating map");
    /// map.set_int_arr2("key", (45, 64)).expect("Error setting integer array for larodMap");
    /// ```
    pub fn set_int_arr2(&mut self, k: &str, v: (i64, i64)) -> Result<()> {
        let Ok(key_cstr) = CString::new(k.as_bytes()) else {
            return Err(Error::CStringAllocation);
        };
        let (success, maybe_error): (bool, Option<Error>) =
            unsafe { try_func!(larodMapSetIntArr2, self.raw, key_cstr.as_ptr(), v.0, v.1) };

        if success {
            debug_assert!(
                maybe_error.is_none(),
                "larodMapSetIntArr2 indicated success AND returned an error!"
            );
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    /// Add an integer array of 4 items to a larodMap object.
    /// Example
    /// ```rust
    /// use larod::LarodMap;
    ///
    /// let map = LarodMap::new().expect("Error creating map");
    /// map.set_int_arr4("key", (45, 64, 36, 23)).expect("Error setting integer array for larodMap");
    /// ```
    pub fn set_int_arr4(&mut self, k: &str, v: (i64, i64, i64, i64)) -> Result<()> {
        let Ok(key_cstr) = CString::new(k.as_bytes()) else {
            return Err(Error::CStringAllocation);
        };
        let (success, maybe_error): (bool, Option<Error>) = unsafe {
            try_func!(
                larodMapSetIntArr4,
                self.raw,
                key_cstr.as_ptr(),
                v.0,
                v.1,
                v.2,
                v.3
            )
        };

        if success {
            debug_assert!(
                maybe_error.is_none(),
                "larodMapSetIntArr4 indicated success AND returned an error!"
            );
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    /// Get a string from a larodMap object.
    /// Example
    /// ```rust
    /// use larod::LarodMap;
    ///
    /// let map = LarodMap::new().expect("Error creating map");
    /// map.set_string("key", "value").expect("Error setting string value for larodMap");
    /// let returned_string = map.get_string("key").expect("Unable to return value for \"key\"");
    /// ```
    pub fn get_string(&self, k: &str) -> Result<String> {
        let Ok(key_cstr) = CString::new(k) else {
            return Err(Error::CStringAllocation);
        };
        let (c_str_ptr, maybe_error): (*const c_char, Option<Error>) =
            unsafe { try_func!(larodMapGetStr, self.raw, key_cstr.as_ptr()) };
        let c_str = unsafe { CStr::from_ptr(c_str_ptr) };
        if let Ok(rs) = c_str.to_str() {
            debug_assert!(
                maybe_error.is_none(),
                "larodMapGetStr returned a string AND returned an error!"
            );
            Ok(String::from(rs))
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    /// Get an integer array of 4 items from a larodMap object.
    /// Example
    /// ```rust
    /// use larod::LarodMap;
    ///
    /// let map = LarodMap::new().expect("Error creating map");
    /// map.set_int("key", 45).expect("Error setting integer array for larodMap");
    /// let value = map.get_int("key").expect("Unable to get array values for \"key\"");
    /// ```
    pub fn get_int(&self, k: &str) -> Result<i64> {
        let Ok(key_cstr) = CString::new(k) else {
            return Err(Error::CStringAllocation);
        };
        let mut v: i64 = 0;
        let (success, maybe_error): (bool, Option<Error>) =
            unsafe { try_func!(larodMapGetInt, self.raw, key_cstr.as_ptr(), &mut v) };
        if success {
            debug_assert!(
                maybe_error.is_none(),
                "larodMapGetInt indicated success AND returned an error!"
            );
            Ok(v)
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    /// Get an integer array of 4 items from a larodMap object.
    /// Example
    /// ```rust
    /// use larod::LarodMap;
    ///
    /// let map = LarodMap::new().expect("Error creating map");
    /// map.set_int_arr2("key", (45, 64)).expect("Error setting integer array for larodMap");
    /// let returned_array = map.get_int_arr2("key").expect("Unable to get array values for \"key\"");
    /// ```
    pub fn get_int_arr2(&self, k: &str) -> Result<&[i64; 2]> {
        let Ok(key_cstr) = CString::new(k) else {
            return Err(Error::CStringAllocation);
        };
        let (out_arr, maybe_error) =
            unsafe { try_func!(larodMapGetIntArr2, self.raw, key_cstr.as_ptr()) };
        if !out_arr.is_null() {
            debug_assert!(
                maybe_error.is_none(),
                "larodMapGetInt indicated success AND returned an error!"
            );
            unsafe {
                slice::from_raw_parts(out_arr, 2)
                    .try_into()
                    .or(Err(Error::PointerToInvalidData))
            }
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    /// Get an integer array of 4 items from a larodMap object.
    /// Example
    /// ```rust
    /// use larod::LarodMap;
    ///
    /// let map = LarodMap::new().expect("Error creating map");
    /// map.set_int_arr4("key", (45, 64, 36, 23)).expect("Error setting integer array for larodMap");
    /// let returned_array = map.get_int_arr4("key").expect("Unable to get array values for \"key\"");
    /// ```
    pub fn get_int_arr4(&self, k: &str) -> Result<&[i64; 4]> {
        let Ok(key_cstr) = CString::new(k) else {
            return Err(Error::CStringAllocation);
        };
        let (out_arr, maybe_error) =
            unsafe { try_func!(larodMapGetIntArr4, self.raw, key_cstr.as_ptr()) };
        if !out_arr.is_null() {
            debug_assert!(
                maybe_error.is_none(),
                "larodMapGetIntArr4 indicated success AND returned an error!"
            );
            unsafe {
                slice::from_raw_parts(out_arr, 4)
                    .try_into()
                    .or(Err(Error::PointerToInvalidData))
            }
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }
}

impl std::ops::Drop for LarodMap {
    fn drop(&mut self) {
        unsafe {
            larodDestroyMap(&mut self.raw);
        }
    }
}

// #[derive(Eq, PartialEq, Hash)]
// pub struct Tensor<'a> {
//     ptr: *mut *mut larodTensor,
//     phantom: PhantomData<&'a Session>,
// }

struct LarodTensorContainer {
    ptr: *mut *mut larodTensor,
    num_tensors: usize,
}
pub struct Tensor(*mut larodTensor);

/// A structure representing a larodTensor.
impl Tensor {
    // fn as_ptr(&self) -> *const larodTensor {
    //     self.ptr.cast_const()
    // }

    // fn as_mut_ptr(&self) -> *mut larodTensor {
    //     self.ptr
    // }

    pub fn name() {}

    pub fn byte_size() {}

    pub fn dims(&self) -> Result<Vec<usize>> {
        let (dims, maybe_error) = unsafe { try_func!(larodGetTensorDims, self.0) };
        if !dims.is_null() {
            debug_assert!(
                maybe_error.is_none(),
                "larodGetTensorDims indicated success AND returned an error!"
            );
            let d = unsafe {
                (*dims)
                    .dims
                    .into_iter()
                    .take((*dims).len)
                    .collect::<Vec<usize>>()
            };
            Ok(d)
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    pub fn set_dims(&self, dims: &[usize]) -> Result<()> {
        let mut dim_array: [usize; 12] = [0; 12];
        for (idx, dim) in dims.iter().take(12).enumerate() {
            dim_array[idx] = *dim;
        }
        let dims_struct = larodTensorDims {
            dims: dim_array,
            len: dims.len(),
        };
        let (success, maybe_error) = unsafe { try_func!(larodSetTensorDims, self.0, &dims_struct) };
        if success {
            debug_assert!(
                maybe_error.is_none(),
                "larodSetTensorDims indicated success AND returned an error!"
            );
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    pub fn pitches() {}
    pub fn set_pitches() {}
    pub fn data_type() {}
    pub fn set_data_type() {}
    pub fn layout() {}
    pub fn set_layout() {}
    pub fn fd() {}
    pub fn set_fd() {}
    pub fn fd_size() {}
    pub fn set_fd_size() {}
    pub fn fd_offset() {}
    pub fn set_fd_offset() {}
    pub fn fd_props() {}
    pub fn set_fd_props() {}
    // pub fn destroy(mut self, session: &Session) -> Result<()> {
    //     let (success, maybe_error) =
    //         unsafe { try_func!(larodDestroyTensors, session.conn, &mut self.ptr, 1) };
    //     if success {
    //         debug_assert!(
    //             maybe_error.is_none(),
    //             "larodDestroyTensors indicated success AND returned an error!"
    //         );
    //         Ok(())
    //     } else {
    //         Err(maybe_error.unwrap_or(Error::MissingLarodError))
    //     }
    // }
}

/// A type representing a larodDevice.
/// The lifetime of LarodDevice is explicitly tied to the lifetime of a
/// [Session]. So using a LarodDevice after the Session it was acquired from
/// will cause compilation to fail.
/// ```compile_fail
/// use larod::Session;
/// let sess = Session::new();
/// let first_device = sess
///     .devices()
///     .expect("unable to get devices")
///     .pop()
///     .expect("empty devices list!");
/// drop(sess);
/// println!("{:?}", first_device.name());
/// ```
#[derive(Debug)]
pub struct LarodDevice<'a> {
    // The caller does not get ownership of the returned pointer and must not
    // attempt to free it. The lifetime of the memory pointed to expires when
    // conn closes.
    ptr: *const larodDevice,
    phantom: PhantomData<&'a Session>,
}

impl<'a> LarodDevice<'a> {
    /// Get the name of a larodDevice.
    pub fn name(&self) -> Result<String> {
        unsafe {
            let (c_char_ptr, maybe_error) = try_func!(larodGetDeviceName, self.ptr);
            if !c_char_ptr.is_null() {
                debug_assert!(
                    maybe_error.is_none(),
                    "larodGetDeviceName returned an object pointer AND returned an error!"
                );
                let c_name = CStr::from_ptr(c_char_ptr);
                c_name
                    .to_str()
                    .map(String::from)
                    .map_err(|_e| Error::InvalidLarodMessage)
            } else {
                Err(maybe_error.unwrap_or(Error::MissingLarodError))
            }
        }
    }

    /// Get the instance of a larodDevice.
    /// From the larod documentation
    /// > *In case there are multiple identical devices that are available in the service, they are distinguished by an instance number, with the first instance starting from zero.*
    pub fn instance(&self) -> Result<u32> {
        unsafe {
            let mut instance: u32 = 0;
            let (success, maybe_error) = try_func!(larodGetDeviceInstance, self.ptr, &mut instance);
            if success {
                debug_assert!(
                    maybe_error.is_none(),
                    "larodGetDeviceInstance returned success AND returned an error!"
                );
                Ok(instance)
            } else {
                Err(maybe_error.unwrap_or(Error::MissingLarodError))
            }
        }
    }
}

pub trait LarodModel {
    fn create_model_inputs(&mut self) -> Result<()>;
    fn num_inputs(&self) -> usize;
    fn start_job(&self) -> Result<()>;
    fn stop(&self);
}

#[derive(Default)]
pub enum ImageFormat {
    #[default]
    NV12,
    RGBInterleaved,
    RGBPlanar,
}

#[derive(Default)]
pub enum PreProcBackend {
    #[default]
    LibYUV,
    ACE,
    VProc,
    OpenCLDLPU,
    OpenCLGPU,
    RemoteLibYuv,
    RemoteOpenCLDLPU,
    RemoteOpenCLGPU,
}

#[derive(Debug, Default)]
pub enum InferenceChip {
    #[default]
    TFLiteCPU,
    TFLiteDLPU,
}

#[derive(Debug, Default)]
pub enum PreProcError {
    #[default]
    UnsupportedOperation,
}

struct Resolution {
    width: u32,
    height: u32,
}

#[derive(Default)]
pub struct PreprocessorBuilder {
    backend: PreProcBackend,
    input_size: Option<Resolution>,
    crop: Option<(i64, i64, i64, i64)>,
    output_size: Option<Resolution>,
    input_format: ImageFormat,
    output_format: ImageFormat,
}

impl PreprocessorBuilder {
    pub fn new() -> Self {
        PreprocessorBuilder::default()
    }

    pub fn backend(mut self, backend: PreProcBackend) -> Self {
        self.backend = backend;
        self
    }

    /// Crop a portion of the input stream
    /// (X offset, Y offset, Width, Height)
    pub fn crop(mut self, crop: (i64, i64, i64, i64)) -> Self {
        self.crop = Some(crop);
        self
    }

    /// Scale the input image width and height to the desired output width and
    /// height. The aspect ratio is not preserved. Size indicates the desired
    /// final output size.
    pub fn output_size(mut self, width: u32, height: u32) -> Self {
        self.output_size = Some(Resolution { width, height });
        self
    }

    pub fn input_size(mut self, width: u32, height: u32) -> Self {
        self.input_size = Some(Resolution { width, height });
        self
    }

    pub fn input_format(mut self, format: ImageFormat) -> Self {
        self.input_format = format;
        self
    }

    pub fn output_format(mut self, format: ImageFormat) -> Self {
        self.output_format = format;
        self
    }

    pub fn load(self, session: &Session) -> Result<Preprocessor> {
        let mut map = LarodMap::new()?;
        match self.input_format {
            ImageFormat::NV12 => map.set_string("image.input.format", "nv12")?,
            ImageFormat::RGBInterleaved => {
                if !matches!(
                    self.backend,
                    PreProcBackend::LibYUV | PreProcBackend::RemoteLibYuv
                ) {
                    return Err(Error::PreprocessorError(PreProcError::UnsupportedOperation));
                } else {
                    map.set_string("image.input.format", "rgb-interleaved")?;
                }
            }
            ImageFormat::RGBPlanar => {
                if !matches!(
                    self.backend,
                    PreProcBackend::LibYUV | PreProcBackend::RemoteLibYuv
                ) {
                    return Err(Error::PreprocessorError(PreProcError::UnsupportedOperation));
                } else {
                    map.set_string("image.input.format", "rgb-planar")?;
                }
            }
        }
        match self.output_format {
            ImageFormat::NV12 => {
                if matches!(
                    self.backend,
                    PreProcBackend::LibYUV | PreProcBackend::RemoteLibYuv
                ) {
                    map.set_string("image.output.format", "nv12")?;
                } else {
                    return Err(Error::PreprocessorError(PreProcError::UnsupportedOperation));
                }
            }
            ImageFormat::RGBInterleaved => {
                if matches!(self.backend, PreProcBackend::VProc) {
                    return Err(Error::PreprocessorError(PreProcError::UnsupportedOperation));
                } else {
                    map.set_string("image.output.format", "rgb-interleaved")?;
                }
            }
            ImageFormat::RGBPlanar => {
                if matches!(
                    self.backend,
                    PreProcBackend::LibYUV | PreProcBackend::VProc | PreProcBackend::RemoteLibYuv
                ) {
                    map.set_string("image.output.format", "rgb-planar")?;
                } else {
                    return Err(Error::PreprocessorError(PreProcError::UnsupportedOperation));
                }
            }
        }
        if let Some(s) = self.input_size {
            map.set_int_arr2(
                "image.input.size",
                (i64::from(s.width), i64::from(s.height)),
            )?;
        }

        let mut crop_map: Option<LarodMap> = None;
        if let Some(crop) = self.crop {
            crop_map = Some(LarodMap::new()?);
            crop_map
                .as_mut()
                .unwrap()
                .set_int_arr4("image.input.crop", crop)?;
        }

        if let Some(s) = self.output_size {
            map.set_int_arr2(
                "image.output.size",
                (i64::from(s.width), i64::from(s.height)),
            )?;
        }

        let device_name = match self.backend {
            PreProcBackend::LibYUV => "cpu-proc",
            PreProcBackend::ACE => "axis-ace-proc",
            PreProcBackend::VProc => "ambarella-cvflow-proc",
            PreProcBackend::OpenCLDLPU => "axis-a8-dlpu-proc",
            PreProcBackend::OpenCLGPU => "axis-a8-gpu-proc",
            PreProcBackend::RemoteLibYuv => "remote-cpu-proc",
            PreProcBackend::RemoteOpenCLDLPU => "remote-axis-a8-dlpu-proc",
            PreProcBackend::RemoteOpenCLGPU => "remote-axis-a8-gpu-proc",
        };
        let (device, maybe_device_error) = unsafe {
            try_func!(
                larodGetDevice,
                session.conn,
                CString::new(device_name)
                    .map_err(|_| Error::CStringAllocation)?
                    .as_ptr(),
                0
            )
        };
        if device.is_null() {
            return Err(maybe_device_error.unwrap_or(Error::MissingLarodError));
        }
        debug_assert!(
            maybe_device_error.is_none(),
            "larodGetDevice indicated success AND returned an error!"
        );
        let (model_ptr, maybe_error) = unsafe {
            try_func!(
                larodLoadModel,
                session.conn,
                -1,
                device,
                LarodAccess::LAROD_ACCESS_PRIVATE,
                CString::new("")
                    .map_err(|_| Error::CStringAllocation)?
                    .as_ptr(),
                map.raw
            )
        };
        if !model_ptr.is_null() {
            debug_assert!(
                maybe_error.is_none(),
                "larodLoadModel indicated success AND returned an error!"
            );
            Ok(Preprocessor {
                session,
                ptr: model_ptr,
                input_tensors: None,
                num_inputs: 0,
                output_tensors: None,
                num_outputs: 0,
                crop: crop_map,
            })
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }
}

pub struct Preprocessor<'a> {
    session: &'a Session,
    ptr: *mut larodModel,
    input_tensors: Option<LarodTensorContainer>,
    num_inputs: usize,
    output_tensors: Option<LarodTensorContainer>,
    num_outputs: usize,
    crop: Option<LarodMap>,
}

impl<'a> Preprocessor<'a> {
    pub fn builder() -> PreprocessorBuilder {
        PreprocessorBuilder::new()
    }
}

impl<'a> LarodModel for Preprocessor<'a> {
    fn create_model_inputs(&mut self) -> Result<()> {
        let (tensors, maybe_error) =
            unsafe { try_func!(larodCreateModelInputs, self.ptr, &mut self.num_inputs) };
        if !tensors.is_null() {
            debug_assert!(
                maybe_error.is_none(),
                "larodCreateModelInputs indicated success AND returned an error!"
            );
            self.input_tensors = Some(LarodTensorContainer {
                ptr: tensors,
                num_tensors: self.num_inputs,
            });
            // let tensor_slice =
            //     unsafe { slice::from_raw_parts::<*mut larodTensor>(tensors, self.num_inputs) };
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    fn num_inputs(&self) -> usize {
        self.num_inputs
    }

    fn start_job(&self) -> Result<()> {
        if self.input_tensors.is_none() || self.output_tensors.is_none() {
            return Err(Error::UnsatisfiedDependencies);
        }
        unsafe {
            try_func!(
                larodCreateJobRequest,
                self.ptr,
                self.input_tensors.as_ref().unwrap().ptr,
                self.num_inputs,
                self.output_tensors.as_ref().unwrap().ptr,
                self.num_outputs,
                self.crop
                    .as_ref()
                    .map_or(ptr::null_mut::<larodMap>(), |m| m.raw)
            );
        }
        Ok(())
    }
    fn stop(&self) {}
}

impl<'a> Drop for Preprocessor<'a> {
    fn drop(&mut self) {
        if let Some(ref mut tensor_container) = self.input_tensors {
            log::debug!("Dropping Preprocessor input tensors!");
            unsafe {
                try_func!(
                    larodDestroyTensors,
                    self.session.conn,
                    &mut tensor_container.ptr,
                    tensor_container.num_tensors
                )
            };
        }
        unsafe { larodDestroyModel(&mut self.ptr) };
    }
}

// #[derive(Default)]
// pub struct ModelBuilder {
//     file_path: Option<PathBuf>,
//     device: InferenceChip,
//     crop: Option<(u32, u32, u32, u32)>,
// }

// impl ModelBuilder {
//     pub fn new() -> Self {
//         ModelBuilder::default()
//     }

//     pub fn source_file(mut self, path: PathBuf) -> Self {
//         self.file_path = Some(path);
//         self
//     }

//     pub fn on_chip(mut self, device: InferenceChip) -> Self {
//         self.device = device;
//         self
//     }

//     pub fn with_crop(mut self, crop: (u32, u32, u32, u32)) -> Self {
//         self.crop = Some(crop);
//         self
//     }

//     pub fn load(self, session: Session) -> Model {}
// }

pub struct InferenceModel<'a> {
    session: &'a Session,
    ptr: *mut larodModel,
    input_tensors: Option<LarodTensorContainer>,
    num_inputs: usize,
}

impl<'a> InferenceModel<'a> {
    pub fn id() -> Result<()> {
        Ok(())
    }
    pub fn chip() -> Result<()> {
        Ok(())
    }
    pub fn device() -> Result<()> {
        Ok(())
    }
    pub fn size() -> Result<()> {
        Ok(())
    }
    pub fn name() -> Result<()> {
        Ok(())
    }
    pub fn access() -> Result<()> {
        Ok(())
    }
    pub fn num_inputs() -> Result<()> {
        Ok(())
    }
    pub fn num_outputs() -> Result<()> {
        Ok(())
    }

    pub fn create_model_outputs() -> Result<()> {
        Ok(())
    }
}

impl<'a> LarodModel for InferenceModel<'a> {
    fn create_model_inputs(&mut self) -> Result<()> {
        let (tensors, maybe_error) =
            unsafe { try_func!(larodCreateModelInputs, self.ptr, &mut self.num_inputs) };
        if !tensors.is_null() {
            debug_assert!(
                maybe_error.is_none(),
                "larodCreateModelInputs indicated success AND returned an error!"
            );
            self.input_tensors = Some(LarodTensorContainer {
                ptr: tensors,
                num_tensors: self.num_inputs,
            });
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    fn num_inputs(&self) -> usize {
        self.num_inputs
    }

    fn start_job(&self) -> Result<()> {
        Ok(())
    }
    fn stop(&self) {}
}

impl<'a> Drop for InferenceModel<'a> {
    fn drop(&mut self) {
        if let Some(ref mut tensor_container) = self.input_tensors {
            unsafe {
                try_func!(
                    larodDestroyTensors,
                    self.session.conn,
                    &mut tensor_container.ptr,
                    tensor_container.num_tensors
                )
            };
        }
        unsafe { larodDestroyModel(&mut self.ptr) };
    }
}

pub struct SessionBuilder {}

impl SessionBuilder {
    pub fn new() -> SessionBuilder {
        SessionBuilder {}
    }
    pub fn build(&self) -> Result<Session> {
        let mut conn: *mut larodConnection = ptr::null_mut();
        let (success, maybe_error): (bool, Option<Error>) =
            unsafe { try_func!(larodConnect, &mut conn) };
        if success {
            debug_assert!(
                maybe_error.is_none(),
                "larodConnect indicated success AND returned an error!"
            );
            Ok(Session { conn })
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }
}

impl Default for SessionBuilder {
    fn default() -> Self {
        SessionBuilder::new()
    }
}

pub struct Session {
    conn: *mut larodConnection,
}

// Using a session builder might not be necessary.
// There's little to configure when starting a session.
impl Session {
    /// Constructs a new `Session`.
    ///
    /// # Panics
    ///
    /// Use `Session::builder()` if you wish to handle the failure as an `Error`
    /// instead of panicking.
    pub fn new() -> Session {
        SessionBuilder::new().build().expect("Session::new()")
    }
    pub fn builder() -> SessionBuilder {
        SessionBuilder::new()
    }
    pub fn disconnect(&mut self) -> Result<()> {
        let (success, maybe_error): (bool, Option<Error>) =
            unsafe { try_func!(larodDisconnect, &mut self.conn) };
        if success {
            debug_assert!(
                maybe_error.is_none(),
                "larodDisconnect indicated success AND returned an error!"
            );
            Ok(())
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }
    pub fn num_sessions() -> Result<()> {
        Ok(())
    }

    /// Returns a reference to an available device
    pub fn device(&self, name: &str, instance: u32) -> Result<LarodDevice> {
        let Ok(name_cstr) = CString::new(name) else {
            return Err(Error::CStringAllocation);
        };
        let (device_ptr, maybe_error) =
            unsafe { try_func!(larodGetDevice, self.conn, name_cstr.as_ptr(), instance) };
        if !device_ptr.is_null() {
            debug_assert!(
                maybe_error.is_none(),
                "larodGetDevice indicated success AND returned an error!"
            );
            Ok(LarodDevice {
                ptr: device_ptr,
                phantom: PhantomData,
            })
        } else {
            Err(maybe_error.unwrap_or(Error::MissingLarodError))
        }
    }

    pub fn list_chips() -> Result<()> {
        Ok(())
    }

    /// Get a reference to a HashMap of name LarodDevice pairs.
    pub fn devices(&self) -> Result<Vec<LarodDevice>> {
        let mut num_devices: usize = 0;
        let (dev_ptr, maybe_error) =
            unsafe { try_func!(larodListDevices, self.conn, &mut num_devices) };
        if dev_ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::MissingLarodError));
        }
        let raw_devices =
            unsafe { slice::from_raw_parts::<*const larodDevice>(dev_ptr, num_devices) };

        let devices: Vec<LarodDevice> = raw_devices
            .iter()
            .map(|ptr| LarodDevice {
                ptr: *ptr,
                phantom: PhantomData,
            })
            .collect();

        Ok(devices)
    }

    pub fn models() -> Result<()> {
        Ok(())
    }
    pub fn delete_model(&self) -> Result<()> {
        Ok(())
    }
    pub fn alloc_model_inputs() -> Result<()> {
        Ok(())
    }
    pub fn alloc_model_outputs() -> Result<()> {
        Ok(())
    }
    pub fn destroy_tensors() -> Result<()> {
        Ok(())
    }
    // pub fn track_tensor(&self, tensor: &Tensor) -> Result<()> {
    //     let (success, maybe_error) =
    //         unsafe { try_func!(larodTrackTensor, self.conn, tensor.as_mut_ptr()) };
    //     if success {
    //         debug_assert!(
    //             maybe_error.is_none(),
    //             "larodTrackTensor indicated success AND returned an error!"
    //         );
    //         Ok(())
    //     } else {
    //         Err(maybe_error.unwrap_or(Error::MissingLarodError))
    //     }
    // }
    pub fn run_job() -> Result<()> {
        Ok(())
    }
    pub fn run_inference() -> Result<()> {
        Ok(())
    }
    pub fn chip_id() -> Result<()> {
        Ok(())
    }
    pub fn chip_type() -> Result<()> {
        Ok(())
    }
}

impl Default for Session {
    fn default() -> Self {
        SessionBuilder::default()
            .build()
            .expect("Session::default()")
    }
}

impl std::ops::Drop for Session {
    fn drop(&mut self) {
        log::debug!("Dropping Session!");
        // unsafe {
        //     try_func!(larodDisconnect, &mut self.conn);
        // }
    }
}

#[cfg(all(test, target_arch = "aarch64", feature = "device-tests"))]
mod tests {
    use super::*;

    #[test]
    fn it_creates_larod_map() {
        env_logger::builder().is_test(true).try_init();
        assert!(LarodMap::new().is_ok());
    }

    #[test]
    fn it_drops_map() {
        env_logger::builder().is_test(true).try_init();
        let map = LarodMap::new().unwrap();
        std::mem::drop(map);
    }

    #[test]
    fn larod_map_can_set_str() {
        env_logger::builder().is_test(true).try_init();
        let mut map = LarodMap::new().unwrap();
        map.set_string("test_key", "test_value").unwrap();
    }

    #[test]
    fn larod_map_can_get_str() {
        let mut map = LarodMap::new().unwrap();
        map.set_string("test_key", "this_value").unwrap();
        let s = map.get_string("test_key").unwrap();
        assert_eq!(s, String::from("this_value"));
    }

    #[test]
    fn larod_map_can_set_int() {
        env_logger::builder().is_test(true).try_init();
        let mut map = LarodMap::new().unwrap();
        map.set_int("test_key", 10).unwrap();
    }

    #[test]
    fn larod_map_can_get_int() {
        env_logger::builder().is_test(true).try_init();
        let mut map = LarodMap::new().unwrap();
        map.set_int("test_key", 9).unwrap();
        let i = map.get_int("test_key").unwrap();
        assert_eq!(i, 9);
    }

    #[test]
    fn larod_map_can_set_2_tuple() {
        env_logger::builder().is_test(true).try_init();
        let mut map = LarodMap::new().unwrap();
        map.set_int_arr2("test_key", (1, 2)).unwrap();
    }
    #[test]
    fn larod_map_can_get_2_tuple() {
        env_logger::builder().is_test(true).try_init();
        let mut map = LarodMap::new().unwrap();
        map.set_int_arr2("test_key", (5, 6)).unwrap();
        let arr = map.get_int_arr2("test_key").unwrap();
        assert_eq!(arr[0], 5);
        assert_eq!(arr[1], 6);
    }

    #[test]
    fn larod_map_can_set_4_tuple() {
        env_logger::builder().is_test(true).try_init();
        let mut map = LarodMap::new().unwrap();
        map.set_int_arr4("test_key", (1, 2, 3, 4)).unwrap();
    }

    #[test]
    fn larod_map_can_get_4_tuple() {
        env_logger::builder().is_test(true).try_init();
        let mut map = LarodMap::new().unwrap();
        map.set_int_arr4("test_key", (1, 2, 3, 4)).unwrap();
        let arr = map.get_int_arr4("test_key").unwrap();
        assert_eq!(arr[0], 1);
        assert_eq!(arr[1], 2);
        assert_eq!(arr[2], 3);
        assert_eq!(arr[3], 4);
    }

    #[test]
    fn it_establishes_session() {
        env_logger::builder().is_test(true).try_init();
        Session::new();
    }

    #[test]
    fn it_lists_devices() {
        env_logger::builder().is_test(true).try_init();
        let sess = Session::new();
        let devices = sess.devices().unwrap();
        for device in devices {
            log::info!(
                "device: {}, id: {}, addr: {:?}",
                device.name().unwrap(),
                device.instance().unwrap(),
                unsafe { std::ptr::addr_of!(*device.ptr) },
            );
        }
    }

    #[test]
    fn it_creates_and_destroys_preprocessor() {
        env_logger::builder().is_test(true).try_init();
        let session = Session::new();
        let mut preprocessor = match Preprocessor::builder()
            .input_format(ImageFormat::NV12)
            .input_size(1920, 1080)
            .output_size(1920, 1080)
            .backend(PreProcBackend::LibYUV)
            .load(&session)
        {
            Ok(p) => p,
            Err(Error::LarodError(e)) => {
                log::error!("Error building preprocessor: {:?}", e.msg());
                panic!()
            }
            Err(e) => {
                log::error!("Unexpected error while building preprocessor: {:?}", e);
                panic!()
            }
        };
        if let Err(Error::LarodError(e)) = preprocessor.create_model_inputs() {
            log::error!("Error creating preprocessor inputs: {:?}", e.msg());
        }
        log::info!("Number of model inputs: {}", preprocessor.num_inputs);
    }

    #[test]
    fn model_errors_with_no_tensors() {
        env_logger::builder().is_test(true).try_init();
        let session = Session::new();
        let mut preprocessor = match Preprocessor::builder()
            .input_format(ImageFormat::NV12)
            .input_size(1920, 1080)
            .output_size(1920, 1080)
            .backend(PreProcBackend::LibYUV)
            .load(&session)
        {
            Ok(p) => p,
            Err(Error::LarodError(e)) => {
                log::error!("Error building preprocessor: {:?}", e.msg());
                panic!()
            }
            Err(e) => {
                log::error!("Unexpected error while building preprocessor: {:?}", e);
                panic!()
            }
        };
        if let Err(Error::LarodError(e)) = preprocessor.create_model_inputs() {
            log::error!("Error creating preprocessor inputs: {:?}", e.msg());
        }
        log::info!("Number of model inputs: {}", preprocessor.num_inputs);
    }
}
