//! Safe Rust bindings for the [VDO (Video Capture) API](https://axiscommunications.github.io/acap-documentation/docs/api/src/api/vdostream/html/index.html).
//!
//! VDO provides access to video streams from Axis cameras, supporting various
//! video formats including H.264, H.265, JPEG, and raw YUV/RGB formats.
//!
//! # Platform Compatibility
//!
//! Video format support varies by hardware platform:
//!
//! | Format | Artpec-6 | Artpec-7 | Artpec-8 | Artpec-9 | Ambarella CV |
//! |--------|----------|----------|----------|----------|--------------|
//! | H.264 | Yes | Yes | Yes | Yes | Yes |
//! | H.265 | No | Yes | Yes | Yes | Yes |
//! | JPEG | Yes | Yes | Yes | Yes | Yes |
//! | YUV (NV12, Y800) | Yes | Yes | Yes | Yes | Yes |
//! | RGB | No | No | No | Yes | Yes |
//! | PLANAR_RGB | No | No | Yes | Yes | Yes |
//! | AV1 | No | No | No | Yes | No |
//!
//! For maximum portability, use `VdoFormat::VDO_FORMAT_YUV`.
//!
//! # Example
//!
//! ```no_run
//! use vdo::{Resolution, Stream, VdoFormat};
//!
//! let stream = Stream::builder()
//!     .channel(0)
//!     .format(VdoFormat::VDO_FORMAT_YUV)
//!     .resolution(Resolution::Exact { width: 1920, height: 1080 })
//!     .build()
//!     .expect("Failed to create stream");
//!
//! let running = stream.start().expect("Failed to start stream");
//!
//! for _ in 0..10 {
//!     let buffer = running.next_buffer().expect("Failed to get buffer");
//!     println!("Frame size: {} bytes", buffer.size());
//! }
//!
//! drop(running);
//! ```
//!
//! # Known Issues
//!
//! - Image rotation may vary between platforms. Check the `rotation` property in stream info.
//! - Some formats (RGB, PLANAR_RGB) may produce upside-down images on certain platforms.

mod map;
pub use map::{CStringPtr, Map};

use glib_sys::GError;
use gobject_sys::{g_object_unref, GObject};
use std::fmt::{Debug, Display};
use std::mem;
use std::ptr;
use vdo_sys::{VdoBuffer, VdoBufferStrategy, VdoStream};

pub use vdo_sys::{VdoFormat, VdoFrameType, VdoRateControlMode, VdoRateControlPriority};

/// Macro for calling VDO functions that take a GError** parameter.
/// Returns a tuple of `(result, Option<Error>)`.
macro_rules! try_func {
    ($func:path, $($arg:expr),+ $(,)?) => {{
        let mut error: *mut GError = ptr::null_mut();
        let success = $func($( $arg ),+, &mut error);
        if error.is_null() {
            (success, None)
        } else {
            (success, Some(Error::Vdo(VdoError::from_gerror(error))))
        }
    }};
}

/// Error type for VDO operations.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Vdo(#[from] VdoError),
    #[error("VDO returned an unexpected null pointer")]
    NullPointer,
    #[error("VDO returned an invalid file descriptor")]
    InvalidFd,
    #[error("Missing error data from VDO library")]
    MissingVdoError,
}

/// Error from the VDO library.
pub struct VdoError {
    code: i32,
    message: String,
}

impl VdoError {
    fn from_gerror(gerror: *mut GError) -> Self {
        if gerror.is_null() {
            return VdoError {
                code: 0,
                message: String::new(),
            };
        }

        // SAFETY: gerror is non-null. We dereference the struct to copy its fields
        // (code, message pointer), then read the message string, all before calling
        // g_error_free which invalidates the GError and its contents.
        let g_error = unsafe { *gerror };
        let message = if g_error.message.is_null() {
            String::from("Unknown error")
        } else {
            unsafe { std::ffi::CStr::from_ptr(g_error.message) }
                .to_str()
                .unwrap_or("Invalid UTF-8 in error message")
                .to_string()
        };

        unsafe { glib_sys::g_error_free(gerror) };

        VdoError {
            code: g_error.code,
            message,
        }
    }

    /// Returns a human-readable name for the VDO error code.
    pub fn code_name(&self) -> &'static str {
        // Compare as i32 to avoid wrapping negative GError codes from non-VDO domains.
        match self.code {
            x if x == vdo_sys::VDO_ERROR_NOT_FOUND.0 as i32 => "VDO_ERROR_NOT_FOUND",
            x if x == vdo_sys::VDO_ERROR_EXISTS.0 as i32 => "VDO_ERROR_EXISTS",
            x if x == vdo_sys::VDO_ERROR_INVALID_ARGUMENT.0 as i32 => "VDO_ERROR_INVALID_ARGUMENT",
            x if x == vdo_sys::VDO_ERROR_PERMISSION_DENIED.0 as i32 => {
                "VDO_ERROR_PERMISSION_DENIED"
            }
            x if x == vdo_sys::VDO_ERROR_NOT_SUPPORTED.0 as i32 => "VDO_ERROR_NOT_SUPPORTED",
            x if x == vdo_sys::VDO_ERROR_CLOSED.0 as i32 => "VDO_ERROR_CLOSED",
            x if x == vdo_sys::VDO_ERROR_BUSY.0 as i32 => "VDO_ERROR_BUSY",
            x if x == vdo_sys::VDO_ERROR_IO.0 as i32 => "VDO_ERROR_IO",
            x if x == vdo_sys::VDO_ERROR_HAL.0 as i32 => "VDO_ERROR_HAL",
            x if x == vdo_sys::VDO_ERROR_DBUS.0 as i32 => "VDO_ERROR_DBUS",
            x if x == vdo_sys::VDO_ERROR_OOM.0 as i32 => "VDO_ERROR_OOM",
            x if x == vdo_sys::VDO_ERROR_IDLE.0 as i32 => "VDO_ERROR_IDLE",
            x if x == vdo_sys::VDO_ERROR_NO_DATA.0 as i32 => "VDO_ERROR_NO_DATA",
            x if x == vdo_sys::VDO_ERROR_NO_BUFFER_SPACE.0 as i32 => "VDO_ERROR_NO_BUFFER_SPACE",
            x if x == vdo_sys::VDO_ERROR_BUFFER_FAILURE.0 as i32 => "VDO_ERROR_BUFFER_FAILURE",
            x if x == vdo_sys::VDO_ERROR_INTERFACE_DOWN.0 as i32 => "VDO_ERROR_INTERFACE_DOWN",
            x if x == vdo_sys::VDO_ERROR_FAILED.0 as i32 => "VDO_ERROR_FAILED",
            x if x == vdo_sys::VDO_ERROR_FATAL.0 as i32 => "VDO_ERROR_FATAL",
            x if x == vdo_sys::VDO_ERROR_NOT_CONTROLLED.0 as i32 => "VDO_ERROR_NOT_CONTROLLED",
            x if x == vdo_sys::VDO_ERROR_NO_EVENT.0 as i32 => "VDO_ERROR_NO_EVENT",
            x if x == vdo_sys::VDO_ERROR_NO_VIDEO.0 as i32 => "VDO_ERROR_NO_VIDEO",
            _ => "VDO_ERROR_UNKNOWN",
        }
    }

    pub fn code(&self) -> i32 {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for VdoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}): {}", self.code_name(), self.code, self.message)
    }
}

impl Debug for VdoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VdoError")
            .field("code", &self.code)
            .field("code_name", &self.code_name())
            .field("message", &self.message)
            .finish()
    }
}

impl std::error::Error for VdoError {}

/// Specifies the video resolution for a stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    /// Use the camera's native resolution.
    Native,
    /// Use an exact resolution.
    Exact { width: u32, height: u32 },
}

/// Builder for creating a video stream.
///
/// Use [`Stream::builder()`] to create a new builder.
///
/// # Example
///
/// ```no_run
/// use vdo::{Resolution, Stream, VdoFormat};
///
/// let stream = Stream::builder()
///     .channel(0)
///     .format(VdoFormat::VDO_FORMAT_H264)
///     .resolution(Resolution::Exact { width: 1920, height: 1080 })
///     .framerate(30)
///     .build()
///     .expect("Failed to build stream");
/// ```
#[derive(Clone)]
pub struct StreamBuilder {
    format: VdoFormat,
    buffer_count: u32,
    channel: u32,
    resolution: Resolution,
    framerate: u32,
}

impl Default for StreamBuilder {
    fn default() -> Self {
        Self {
            format: VdoFormat::VDO_FORMAT_H264,
            buffer_count: 3,
            channel: 0,
            resolution: Resolution::Native,
            framerate: 0,
        }
    }
}

impl StreamBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Default: `VdoFormat::VDO_FORMAT_H264`
    ///
    /// See the [platform compatibility table](crate#platform-compatibility) for supported formats.
    pub fn format(mut self, format: VdoFormat) -> Self {
        self.format = format;
        self
    }

    /// Default: 0 (main channel)
    pub fn channel(mut self, channel: u32) -> Self {
        self.channel = channel;
        self
    }

    /// Default: [`Resolution::Native`]
    pub fn resolution(mut self, resolution: Resolution) -> Self {
        self.resolution = resolution;
        self
    }

    /// If 0, the camera's default framerate is used.
    pub fn framerate(mut self, framerate: u32) -> Self {
        self.framerate = framerate;
        self
    }

    /// Default: 3. For YUV/RGB formats, controls frame buffer count.
    /// For compressed formats (H.264, H.265, JPEG), typically ignored.
    pub fn buffers(mut self, count: u32) -> Self {
        self.buffer_count = count;
        self
    }

    /// Builds the stream.
    ///
    /// Returns an error if the stream could not be created (e.g., invalid format
    /// for the platform, or camera not available).
    pub fn build(self) -> std::result::Result<Stream, Error> {
        let mut map = Map::try_new()?;
        map.set_u32(c"channel", self.channel);
        map.set_u32(c"format", self.format.0 as u32);
        if let Resolution::Exact { width, height } = self.resolution {
            map.set_u32(c"width", width);
            map.set_u32(c"height", height);
        }
        if self.framerate > 0 {
            map.set_u32(c"framerate", self.framerate);
        }
        map.set_u32(c"buffer.count", self.buffer_count);
        // Always use INFINITE strategy; EXPLICIT is not exposed because it
        // requires unsafe application-managed buffer allocation.
        map.set_u32(
            c"buffer.strategy",
            VdoBufferStrategy::VDO_BUFFER_STRATEGY_INFINITE.0,
        );

        let (stream_raw, maybe_error) =
            unsafe { try_func!(vdo_sys::vdo_stream_new, map.as_ptr(), None) };

        if stream_raw.is_null() {
            return Err(maybe_error.unwrap_or(Error::MissingVdoError));
        }

        debug_assert!(
            maybe_error.is_none(),
            "vdo_stream_new returned a stream pointer AND an error"
        );

        Ok(Stream {
            raw: stream_raw,
            started: false,
        })
    }
}

/// A video stream from a camera channel.
///
/// Use [`Stream::builder()`] to create a stream, then call [`Stream::start()`]
/// to begin capturing frames. Starting consumes the `Stream` and returns a
/// [`RunningStream`].
///
/// # Example
///
/// ```no_run
/// use vdo::{Resolution, Stream, VdoFormat};
///
/// let stream = Stream::builder()
///     .format(VdoFormat::VDO_FORMAT_JPEG)
///     .resolution(Resolution::Exact { width: 640, height: 480 })
///     .build()?;
///
/// let running = stream.start()?;
/// for _ in 0..5 {
///     let buffer = running.next_buffer()?;
///     println!("Got frame: {} bytes", buffer.size());
/// }
/// drop(running);
/// # Ok::<(), vdo::Error>(())
/// ```
#[derive(Debug)]
pub struct Stream {
    raw: *mut VdoStream,
    started: bool,
}

// SAFETY: We hold exclusive ownership of the raw pointer and the VDO SDK
// does not require streams to be pinned to a specific thread.
unsafe impl Send for Stream {}

impl Stream {
    pub fn builder() -> StreamBuilder {
        StreamBuilder::new()
    }

    /// Equivalent to `Stream::builder().build()` (H.264 format, native resolution).
    pub fn new() -> std::result::Result<Self, Error> {
        StreamBuilder::new().build()
    }

    /// Returns stream information (actual resolution, format, etc.) as a map.
    pub fn info(&self) -> std::result::Result<Map, Error> {
        let (map_raw, maybe_error) = unsafe { try_func!(vdo_sys::vdo_stream_get_info, self.raw) };
        if map_raw.is_null() {
            return Err(maybe_error.unwrap_or(Error::MissingVdoError));
        }
        // SAFETY: map_raw is non-null and freshly returned by VDO with ownership transferred.
        Ok(unsafe { Map::from_raw(map_raw) })
    }

    /// Returns stream settings as a map.
    pub fn settings(&self) -> std::result::Result<Map, Error> {
        let (map_raw, maybe_error) =
            unsafe { try_func!(vdo_sys::vdo_stream_get_settings, self.raw) };
        if map_raw.is_null() {
            return Err(maybe_error.unwrap_or(Error::MissingVdoError));
        }
        Ok(unsafe { Map::from_raw(map_raw) })
    }

    /// Starts the stream, consuming `self` and returning a [`RunningStream`].
    ///
    /// On failure, the stream is consumed and cannot be reused; create a new
    /// stream via [`Stream::builder()`] to retry.
    pub fn start(mut self) -> std::result::Result<RunningStream, Error> {
        let (success, maybe_error) = unsafe { try_func!(vdo_sys::vdo_stream_start, self.raw) };
        if success == glib_sys::GFALSE {
            return Err(maybe_error.unwrap_or(Error::MissingVdoError));
        }
        self.started = true;
        Ok(RunningStream { stream: self })
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        if self.started {
            unsafe { vdo_sys::vdo_stream_stop(self.raw) };
        }
        // Release our GObject reference to avoid leaking.
        unsafe { g_object_unref(self.raw as *mut GObject) };
    }
}

/// A running video stream that yields frame buffers.
///
/// Created by calling [`Stream::start()`]. Use [`next_buffer()`](RunningStream::next_buffer)
/// to retrieve frame buffers. Drop this value to stop the stream.
pub struct RunningStream {
    stream: Stream,
}

// SAFETY: Owns a Stream (which is Send) and the VDO SDK does not
// require streams to be pinned to a specific thread.
unsafe impl Send for RunningStream {}

impl RunningStream {
    /// Blocks until a new frame is available and returns it.
    pub fn next_buffer(&self) -> std::result::Result<StreamBuffer<'_>, Error> {
        let (buffer_ptr, maybe_error) =
            unsafe { try_func!(vdo_sys::vdo_stream_get_buffer, self.stream.raw) };

        if buffer_ptr.is_null() {
            return Err(maybe_error.unwrap_or(Error::MissingVdoError));
        }

        Ok(StreamBuffer {
            raw: buffer_ptr,
            stream: &self.stream,
        })
    }
}

/// A buffer containing a video frame from a running stream.
///
/// Since `VdoBuffer` and `VdoFrame` are the same type in the C API, all frame
/// metadata (size, timestamp, frame type, etc.) is accessed directly on this type.
///
/// The buffer borrows from the [`RunningStream`] that produced it and is
/// automatically unreferenced when dropped. Use [`unref()`](StreamBuffer::unref)
/// to handle unref errors explicitly.
///
/// # Buffer Validity
///
/// The buffer data pointer and frame metadata remain valid until the buffer is
/// unreferenced (on drop or via [`unref()`](StreamBuffer::unref)).
pub struct StreamBuffer<'a> {
    raw: *mut VdoBuffer,
    stream: &'a Stream,
}

impl StreamBuffer<'_> {
    pub fn capacity(&self) -> usize {
        unsafe { vdo_sys::vdo_buffer_get_capacity(self.raw) }
    }

    /// Returns the frame data as a byte slice of [`capacity()`](StreamBuffer::capacity) bytes.
    ///
    /// Use [`size()`](StreamBuffer::size) to get the actual frame data size.
    pub fn as_slice(&self) -> std::result::Result<&[u8], Error> {
        let data = unsafe { vdo_sys::vdo_buffer_get_data(self.raw) };
        if data.is_null() {
            return Err(Error::NullPointer);
        }
        // SAFETY: VDO buffers are backed by mmap'd or allocated regions that are fully
        // initialized at allocation time. Bytes beyond size() may be stale but are valid.
        let slice = unsafe { std::slice::from_raw_parts(data as *const u8, self.capacity()) };
        Ok(slice)
    }

    /// Returns a copy of exactly [`size()`](StreamBuffer::size) bytes of frame data.
    pub fn data_copy(&self) -> std::result::Result<Vec<u8>, Error> {
        let data = unsafe { vdo_sys::vdo_buffer_get_data(self.raw) };
        if data.is_null() {
            return Err(Error::NullPointer);
        }
        // Clamp size to capacity to avoid reading beyond the mapped region.
        let size = self.size().min(self.capacity());
        let slice = unsafe { std::slice::from_raw_parts(data as *const u8, size) };
        Ok(slice.to_vec())
    }

    pub fn frame_type(&self) -> VdoFrameType {
        unsafe { vdo_sys::vdo_frame_get_frame_type(self.raw) }
    }

    /// Starts at 0 and increments with each frame. Wrap-around point is undefined.
    pub fn sequence_number(&self) -> u32 {
        unsafe { vdo_sys::vdo_frame_get_sequence_nbr(self.raw) }
    }

    /// Timestamp in microseconds since boot.
    pub fn timestamp(&self) -> u64 {
        unsafe { vdo_sys::vdo_frame_get_timestamp(self.raw) }
    }

    pub fn custom_timestamp_us(&self) -> i64 {
        unsafe { vdo_sys::vdo_frame_get_custom_timestamp(self.raw) }
    }

    /// Actual frame data size in bytes (may be less than [`capacity()`](StreamBuffer::capacity)).
    pub fn size(&self) -> usize {
        unsafe { vdo_sys::vdo_frame_get_size(self.raw) }
    }

    /// Returns the header size in bytes, or `None` if the frame has no header.
    pub fn header_size(&self) -> Option<usize> {
        let size = unsafe { vdo_sys::vdo_frame_get_header_size(self.raw) };
        if size < 0 {
            None
        } else {
            Some(size as usize)
        }
    }

    /// Returns the raw file descriptor for the buffer's backing memory.
    ///
    /// # Safety
    ///
    /// The returned fd is owned by VDO and will be closed when this buffer is
    /// unreferenced. The caller must not close or duplicate (`dup`) the fd.
    pub unsafe fn file_descriptor(&self) -> std::result::Result<std::os::fd::RawFd, Error> {
        let fd = vdo_sys::vdo_buffer_get_fd(self.raw);
        if fd < 0 {
            return Err(Error::InvalidFd);
        }
        Ok(fd)
    }

    pub fn is_last_buffer(&self) -> bool {
        unsafe { vdo_sys::vdo_frame_get_is_last_buffer(self.raw) != glib_sys::GFALSE }
    }

    /// Explicitly unreferences this buffer, returning an error if the operation fails.
    ///
    /// Normally buffers are unreferenced on drop. Use this if you need error handling.
    pub fn unref(self) -> std::result::Result<(), Error> {
        // Local copy so self.raw stays valid for Drop if unref fails.
        let mut raw = self.raw;
        let stream_raw = self.stream.raw;

        let (success, maybe_error) =
            unsafe { try_func!(vdo_sys::vdo_stream_buffer_unref, stream_raw, &mut raw) };
        if success == glib_sys::GFALSE {
            return Err(maybe_error.unwrap_or(Error::MissingVdoError));
        }
        mem::forget(self); // buffer already unreferenced, suppress Drop
        Ok(())
    }
}

impl Drop for StreamBuffer<'_> {
    fn drop(&mut self) {
        let (success, maybe_error) = unsafe {
            try_func!(
                vdo_sys::vdo_stream_buffer_unref,
                self.stream.raw,
                &mut self.raw
            )
        };
        if success == glib_sys::GFALSE || maybe_error.is_some() {
            match maybe_error {
                Some(err) => log::error!("Failed to unref buffer: {}", err),
                None => log::error!("Failed to unref buffer (no GError details)"),
            }
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use expect_test::expect;

    #[test]
    fn error_code_names() {
        let err = VdoError {
            code: vdo_sys::VDO_ERROR_NOT_FOUND.0 as i32,
            message: "test".to_string(),
        };
        expect!["VDO_ERROR_NOT_FOUND"].assert_eq(err.code_name());

        let err = VdoError {
            code: vdo_sys::VDO_ERROR_NOT_SUPPORTED.0 as i32,
            message: "test".to_string(),
        };
        expect!["VDO_ERROR_NOT_SUPPORTED"].assert_eq(err.code_name());

        let err = VdoError {
            code: 9999,
            message: "test".to_string(),
        };
        expect!["VDO_ERROR_UNKNOWN"].assert_eq(err.code_name());

        // Negative codes (from non-VDO GError domains) should map to UNKNOWN,
        // not wrap to a matching VDO constant.
        let err = VdoError {
            code: -1,
            message: "test".to_string(),
        };
        expect!["VDO_ERROR_UNKNOWN"].assert_eq(err.code_name());
    }

    #[test]
    fn error_display() {
        let err = VdoError {
            code: vdo_sys::VDO_ERROR_BUSY.0 as i32,
            message: "Resource is busy".to_string(),
        };
        expect!["VDO_ERROR_BUSY (7): Resource is busy"].assert_eq(&format!("{err}"));
    }

    #[test]
    fn stream_builder_defaults() {
        let builder = StreamBuilder::default();
        assert_eq!(builder.format, VdoFormat::VDO_FORMAT_H264);
        assert_eq!(builder.channel, 0);
        assert_eq!(builder.buffer_count, 3);
        assert_eq!(builder.resolution, Resolution::Native);
    }

    #[test]
    fn stream_builder_chaining() {
        let builder = StreamBuilder::new()
            .format(VdoFormat::VDO_FORMAT_JPEG)
            .channel(1)
            .resolution(Resolution::Exact {
                width: 1280,
                height: 720,
            })
            .framerate(30)
            .buffers(5);

        assert_eq!(builder.format, VdoFormat::VDO_FORMAT_JPEG);
        assert_eq!(builder.channel, 1);
        assert_eq!(
            builder.resolution,
            Resolution::Exact {
                width: 1280,
                height: 720
            }
        );
        assert_eq!(builder.framerate, 30);
        assert_eq!(builder.buffer_count, 5);
    }

    #[test]
    fn error_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Error>();
    }

    #[test]
    fn vdo_error_from_null() {
        let err = VdoError::from_gerror(ptr::null_mut());
        assert_eq!(err.code(), 0);
        assert!(err.message().is_empty());
    }

    #[test]
    fn error_from_vdo_error() {
        let vdo_err = VdoError {
            code: 1,
            message: "test".to_string(),
        };
        let err: Error = Error::from(vdo_err);
        match err {
            Error::Vdo(e) => {
                assert_eq!(e.code(), 1);
                assert_eq!(e.message(), "test");
            }
            _ => panic!("Expected Error::Vdo"),
        }
    }

    #[test]
    fn all_error_variants_display() {
        expect!["VDO returned an unexpected null pointer"]
            .assert_eq(&format!("{}", Error::NullPointer));
        expect!["VDO returned an invalid file descriptor"]
            .assert_eq(&format!("{}", Error::InvalidFd));
        expect!["Missing error data from VDO library"]
            .assert_eq(&format!("{}", Error::MissingVdoError));
        let vdo = Error::Vdo(VdoError {
            code: 1,
            message: "test".to_string(),
        });
        expect!["VDO_ERROR_NOT_FOUND (1): test"].assert_eq(&format!("{vdo}"));
    }
}

// These tests require the VDO shared library (libvdo.so) and actual camera hardware.
// Results depend on the specific camera model and firmware.
#[cfg(not(any(target_arch = "x86_64", target_os = "macos")))]
#[cfg(test)]
mod tests {
    use super::*;

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn stream_starts_and_stops() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(Resolution::Exact {
                width: 640,
                height: 480,
            })
            .build()?;

        let running = stream.start()?;
        drop(running);
        Ok(())
    }

    #[test]
    fn stream_new_default() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let stream = Stream::new()?;
        let _info = stream.info()?;
        Ok(())
    }

    #[test]
    fn native_resolution() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let stream = Stream::builder()
            .format(VdoFormat::VDO_FORMAT_YUV)
            .build()?;

        let running = stream.start()?;
        let buffer = running.next_buffer()?;
        assert!(buffer.size() > 0);
        drop(buffer);
        drop(running);
        Ok(())
    }

    #[test]
    fn stream_info_available() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(Resolution::Exact {
                width: 640,
                height: 480,
            })
            .build()?;

        let info = stream.info()?;
        info.dump();
        Ok(())
    }

    #[test]
    fn stream_settings_available() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(Resolution::Exact {
                width: 640,
                height: 480,
            })
            .build()?;

        let settings = stream.settings()?;
        settings.dump();
        Ok(())
    }

    #[test]
    fn capture_yuv_frames() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(Resolution::Exact {
                width: 640,
                height: 480,
            })
            .build()?;

        let running = stream.start()?;

        for i in 0..5 {
            let buffer = running.next_buffer()?;
            let size = buffer.size();
            assert!(size > 0, "Frame {} size should be > 0", i);

            // YUV NV12: width * height * 1.5 bytes
            let expected_min = (640 * 480) as usize;
            assert!(
                size >= expected_min,
                "YUV frame too small: {} < {}",
                size,
                expected_min
            );

            log::info!(
                "YUV frame {}: {} bytes, seq={}, ts={}",
                i,
                size,
                buffer.sequence_number(),
                buffer.timestamp()
            );
        }

        drop(running);
        Ok(())
    }

    #[test]
    fn capture_jpeg_frames() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_JPEG)
            .resolution(Resolution::Exact {
                width: 640,
                height: 480,
            })
            .build()?;

        let running = stream.start()?;

        for i in 0..5 {
            let buffer = running.next_buffer()?;
            assert!(buffer.size() > 0, "Frame {} size should be > 0", i);

            let data = buffer.as_slice()?;
            assert!(data.len() >= 2, "Buffer too small for JPEG");
            assert_eq!(data[0], 0xFF, "Invalid JPEG SOI marker");
            assert_eq!(data[1], 0xD8, "Invalid JPEG SOI marker");

            log::info!("JPEG frame {}: {} bytes", i, buffer.size());
        }

        drop(running);
        Ok(())
    }

    #[test]
    fn capture_h264_frames() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_H264)
            .resolution(Resolution::Exact {
                width: 640,
                height: 480,
            })
            .build()?;

        let running = stream.start()?;

        let mut got_i_frame = false;

        for _ in 0..30 {
            let buffer = running.next_buffer()?;
            assert!(buffer.size() > 0, "H.264 frame size should be > 0");

            match buffer.frame_type() {
                VdoFrameType::VDO_FRAME_TYPE_H264_IDR | VdoFrameType::VDO_FRAME_TYPE_H264_I => {
                    got_i_frame = true;
                }
                _ => {}
            }

            log::info!(
                "H.264 frame: {} bytes, type={:?}, seq={}",
                buffer.size(),
                buffer.frame_type(),
                buffer.sequence_number()
            );
        }

        assert!(got_i_frame, "Should have captured at least one I-frame");

        drop(running);
        Ok(())
    }

    /// Skips gracefully on platforms without H.265 support (e.g., Artpec-6).
    #[test]
    fn capture_h265_frames() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();

        let stream_result = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_H265)
            .resolution(Resolution::Exact {
                width: 640,
                height: 480,
            })
            .build();

        match stream_result {
            Ok(stream) => {
                let running = stream.start()?;

                for _ in 0..10 {
                    let buffer = running.next_buffer()?;
                    assert!(buffer.size() > 0, "H.265 frame size should be > 0");
                    log::info!("H.265 frame: {} bytes", buffer.size());
                }

                drop(running);
            }
            Err(Error::Vdo(e)) if e.code_name() == "VDO_ERROR_NOT_SUPPORTED" => {
                log::info!("H.265 not supported on this platform, skipping");
            }
            Err(e) => return Err(e.into()),
        }

        Ok(())
    }

    #[test]
    fn frame_timestamps_increase() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(Resolution::Exact {
                width: 320,
                height: 240,
            })
            .framerate(15)
            .build()?;

        let running = stream.start()?;
        let mut prev_ts = 0u64;
        let mut prev_seq = 0u32;

        for i in 0..10 {
            let buffer = running.next_buffer()?;
            let ts = buffer.timestamp();
            let seq = buffer.sequence_number();

            if i > 0 {
                assert!(
                    ts > prev_ts,
                    "Timestamp should increase: {} <= {}",
                    ts,
                    prev_ts
                );
                assert!(
                    seq > prev_seq,
                    "Sequence should increase: {} <= {}",
                    seq,
                    prev_seq
                );
            }

            prev_ts = ts;
            prev_seq = seq;
        }

        drop(running);
        Ok(())
    }

    #[test]
    fn buffer_data_accessible() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(Resolution::Exact {
                width: 320,
                height: 240,
            })
            .build()?;

        let running = stream.start()?;

        for _ in 0..3 {
            let buffer = running.next_buffer()?;
            let capacity = buffer.capacity();
            let data = buffer.as_slice()?;

            assert_eq!(data.len(), capacity, "Slice length should match capacity");
            assert!(
                buffer.size() <= capacity,
                "Frame size should be <= capacity"
            );

            std::hint::black_box(data[0]);
            std::hint::black_box(data[buffer.size().saturating_sub(1)]);
        }

        drop(running);
        Ok(())
    }

    #[test]
    fn data_copy_returns_frame_data() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(Resolution::Exact {
                width: 320,
                height: 240,
            })
            .build()?;

        let running = stream.start()?;
        let buffer = running.next_buffer()?;

        let copy = buffer.data_copy()?;
        assert_eq!(copy.len(), buffer.size());

        // Verify copy matches the original slice
        let slice = buffer.as_slice()?;
        assert_eq!(&copy[..], &slice[..copy.len()]);

        drop(buffer);
        drop(running);
        Ok(())
    }

    #[test]
    fn file_descriptor_is_valid() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(Resolution::Exact {
                width: 320,
                height: 240,
            })
            .build()?;

        let running = stream.start()?;
        let buffer = running.next_buffer()?;

        // SAFETY: We only read the fd value for the assertion; we do not close or dup it.
        let fd = unsafe { buffer.file_descriptor()? };
        assert!(fd >= 0, "File descriptor should be non-negative");

        drop(buffer);
        drop(running);
        Ok(())
    }

    #[test]
    fn explicit_unref() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(Resolution::Exact {
                width: 320,
                height: 240,
            })
            .build()?;

        let running = stream.start()?;
        let buffer = running.next_buffer()?;
        buffer.unref()?;

        drop(running);
        Ok(())
    }

    #[test]
    fn all_buffer_metadata_accessible() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(Resolution::Exact {
                width: 320,
                height: 240,
            })
            .build()?;

        let running = stream.start()?;
        let buffer = running.next_buffer()?;

        // Exercise all metadata accessors
        std::hint::black_box(buffer.size());
        std::hint::black_box(buffer.capacity());
        std::hint::black_box(buffer.frame_type());
        std::hint::black_box(buffer.sequence_number());
        std::hint::black_box(buffer.timestamp());
        std::hint::black_box(buffer.custom_timestamp_us());
        std::hint::black_box(buffer.header_size());
        std::hint::black_box(buffer.is_last_buffer());

        drop(buffer);
        drop(running);
        Ok(())
    }

    #[test]
    fn multiple_streams_sequential() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();

        {
            let stream = Stream::builder()
                .format(VdoFormat::VDO_FORMAT_YUV)
                .resolution(Resolution::Exact {
                    width: 320,
                    height: 240,
                })
                .build()?;

            let running = stream.start()?;
            for _ in 0..3 {
                let _buf = running.next_buffer()?;
            }
            drop(running);
        }

        {
            let stream = Stream::builder()
                .format(VdoFormat::VDO_FORMAT_JPEG)
                .resolution(Resolution::Exact {
                    width: 320,
                    height: 240,
                })
                .build()?;

            let running = stream.start()?;
            for _ in 0..3 {
                let _buf = running.next_buffer()?;
            }
            drop(running);
        }

        Ok(())
    }

    /// Two streams open simultaneously, polled round-robin from one thread.
    /// May fail if the camera doesn't support multiple concurrent streams.
    #[test]
    fn interleaved_streams() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();

        let stream1 = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(Resolution::Exact {
                width: 320,
                height: 240,
            })
            .build()?;

        let stream2 = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_JPEG)
            .resolution(Resolution::Exact {
                width: 320,
                height: 240,
            })
            .build()?;

        let running1 = stream1.start()?;
        let running2 = stream2.start()?;

        for _ in 0..3 {
            let _buf1 = running1.next_buffer()?;
            let _buf2 = running2.next_buffer()?;
        }

        drop(running1);
        drop(running2);

        Ok(())
    }

    #[test]
    fn invalid_channel_returns_error() {
        init_logger();
        let result = Stream::builder()
            .channel(999)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(Resolution::Exact {
                width: 640,
                height: 480,
            })
            .build();

        assert!(result.is_err(), "Invalid channel should return error");
        if let Err(e) = result {
            log::info!("Expected error for invalid channel: {}", e);
        }
    }

    /// Observational test: result is platform-dependent, logged but not asserted.
    #[test]
    fn unsupported_format_logged() {
        init_logger();
        let result = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_BAYER)
            .resolution(Resolution::Exact {
                width: 640,
                height: 480,
            })
            .build();

        match result {
            Ok(_) => log::info!("BAYER format is supported on this platform"),
            Err(e) => log::info!("BAYER format not supported: {}", e),
        }
    }

    /// Observational test: the camera may adjust or reject unusual resolutions.
    #[test]
    fn invalid_resolution_logged() {
        init_logger();
        let result = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(Resolution::Exact {
                width: 12345,
                height: 6789,
            })
            .build();

        match result {
            Ok(stream) => {
                if let Ok(info) = stream.info() {
                    log::info!("Camera accepted unusual resolution, check actual via info");
                    info.dump();
                }
            }
            Err(e) => {
                log::info!("Camera rejected unusual resolution: {}", e);
            }
        }
    }

    /// Tests that dropping a RunningStream without explicit drop doesn't crash.
    #[test]
    fn stream_dropped_without_stop() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();

        {
            let stream = Stream::builder()
                .channel(0)
                .format(VdoFormat::VDO_FORMAT_YUV)
                .resolution(Resolution::Exact {
                    width: 320,
                    height: 240,
                })
                .build()?;

            let running = stream.start()?;
            for _ in 0..2 {
                let _buf = running.next_buffer()?;
            }
            // Intentionally NOT calling drop(running)
        }

        log::info!("Stream dropped without explicit stop - cleanup successful");
        Ok(())
    }

    #[test]
    fn error_message_is_descriptive() {
        init_logger();

        let err = Stream::builder()
            .channel(999)
            .build()
            .expect_err("Channel 999 should fail");

        match err {
            Error::Vdo(e) => {
                assert!(
                    !e.code_name().is_empty(),
                    "Error code name should not be empty"
                );
                assert!(!e.message().is_empty(), "Error message should not be empty");
            }
            other => panic!("Expected Error::Vdo, got: {:?}", other),
        }
    }

    #[test]
    fn rapid_stream_creation_destruction() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();

        for i in 0..5 {
            let stream = Stream::builder()
                .channel(0)
                .format(VdoFormat::VDO_FORMAT_YUV)
                .resolution(Resolution::Exact {
                    width: 320,
                    height: 240,
                })
                .build()?;

            let running = stream.start()?;
            drop(running.next_buffer()?);
            drop(running);

            log::info!("Rapid cycle {} complete", i);
        }

        Ok(())
    }

    // This test only requires libvdo.so, not camera hardware, but is placed here
    // because the VDO library is only available on the device.
    #[test]
    fn map_get_set_operations() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();

        let mut map = Map::try_new()?;

        map.set_u32(c"test_u32", 42);
        assert_eq!(map.get_u32(c"test_u32", 0), 42);
        assert_eq!(map.get_u32(c"missing_key", 99), 99);

        map.set_bool(c"test_bool", true);
        assert!(map.get_bool(c"test_bool", false));
        assert!(!map.get_bool(c"missing_bool", false));

        map.set_string(c"test_str", c"hello");
        let value = map.get_string(c"test_str");
        assert!(value.is_some());
        assert_eq!(value.unwrap().as_c_str().to_str().unwrap(), "hello");
        assert!(map.get_string(c"missing_str").is_none());

        Ok(())
    }
}
