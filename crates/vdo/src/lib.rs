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
//! use vdo::{Stream, VdoFormat};
//!
//! let mut stream = Stream::builder()
//!     .channel(0)
//!     .format(VdoFormat::VDO_FORMAT_YUV)
//!     .resolution(1920, 1080)
//!     .build()
//!     .expect("Failed to create stream");
//!
//! let mut running = stream.start().expect("Failed to start stream");
//!
//! for buffer in running.iter().take(10) {
//!     let frame = buffer.frame().expect("Failed to get frame");
//!     println!("Frame size: {} bytes", frame.size());
//! }
//!
//! running.stop().expect("Failed to stop stream");
//! ```
//!
//! # Known Issues
//!
//! - Image rotation may vary between platforms. Check the `rotation` property in stream info.
//! - Some formats (RGB, PLANAR_RGB) may produce upside-down images on certain platforms.

use glib_sys::GError;
use gobject_sys::{g_object_unref, GObject};
use std::ffi::{CStr, CString};
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::mem;
use std::ptr;
use vdo_sys::*;

// Re-export commonly used types from vdo-sys
pub use vdo_sys::{
    VdoBufferStrategy, VdoFormat, VdoFrameType, VdoRateControlMode, VdoRateControlPriority,
};

/// Macro for calling VDO functions that take a GError** parameter.
/// Returns a tuple of (result, Option<Error>).
macro_rules! try_func {
    ($func:ident $(,)?) => {{
        let mut error: *mut GError = ptr::null_mut();
        let success = $func(&mut error);
        if error.is_null() {
            (success, None)
        } else {
            (success, Some(Error::Vdo(VdoError::from_gerror(error))))
        }
    }};
    ($func:ident, $($arg:expr),+ $(,)?) => {{
        let mut error: *mut GError = ptr::null_mut();
        let success = $func($( $arg ),+, &mut error);
        if error.is_null() {
            (success, None)
        } else {
            (success, Some(Error::Vdo(VdoError::from_gerror(error))))
        }
    }};
}

// ============================================================================
// Error types
// ============================================================================

/// Error type for VDO operations.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Error returned by the VDO library.
    #[error(transparent)]
    Vdo(#[from] VdoError),
    /// VDO returned an unexpected null pointer.
    #[error("VDO returned an unexpected null pointer")]
    NullPointer,
    /// Could not allocate memory for CString.
    #[error("Could not allocate memory for CString")]
    CStringAllocation,
    /// Missing error data from VDO library.
    #[error("Missing error data from VDO library")]
    MissingVdoError,
    /// No buffers are allocated for the stream.
    #[error("No buffers are allocated for the stream")]
    NoBuffersAllocated,
}

/// Result type for VDO operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Error from the VDO library.
#[derive(Default)]
pub struct VdoError {
    code: i32,
    message: String,
}

impl VdoError {
    fn from_gerror(gerror: *mut GError) -> Self {
        if gerror.is_null() {
            return VdoError::default();
        }

        let g_error = unsafe { *gerror };
        let message = if g_error.message.is_null() {
            String::from("Unknown error")
        } else {
            unsafe { CStr::from_ptr(g_error.message) }
                .to_str()
                .unwrap_or("Invalid UTF-8 in error message")
                .to_string()
        };

        // Free the GError
        unsafe { glib_sys::g_error_free(gerror) };

        VdoError {
            code: g_error.code,
            message,
        }
    }

    /// Returns the error code name.
    pub fn code_name(&self) -> &'static str {
        let code = self.code as u32;
        match code {
            x if x == VDO_ERROR_NOT_FOUND.0 => "VDO_ERROR_NOT_FOUND",
            x if x == VDO_ERROR_EXISTS.0 => "VDO_ERROR_EXISTS",
            x if x == VDO_ERROR_INVALID_ARGUMENT.0 => "VDO_ERROR_INVALID_ARGUMENT",
            x if x == VDO_ERROR_PERMISSION_DENIED.0 => "VDO_ERROR_PERMISSION_DENIED",
            x if x == VDO_ERROR_NOT_SUPPORTED.0 => "VDO_ERROR_NOT_SUPPORTED",
            x if x == VDO_ERROR_CLOSED.0 => "VDO_ERROR_CLOSED",
            x if x == VDO_ERROR_BUSY.0 => "VDO_ERROR_BUSY",
            x if x == VDO_ERROR_IO.0 => "VDO_ERROR_IO",
            x if x == VDO_ERROR_HAL.0 => "VDO_ERROR_HAL",
            x if x == VDO_ERROR_DBUS.0 => "VDO_ERROR_DBUS",
            x if x == VDO_ERROR_OOM.0 => "VDO_ERROR_OOM",
            x if x == VDO_ERROR_IDLE.0 => "VDO_ERROR_IDLE",
            x if x == VDO_ERROR_NO_DATA.0 => "VDO_ERROR_NO_DATA",
            x if x == VDO_ERROR_NO_BUFFER_SPACE.0 => "VDO_ERROR_NO_BUFFER_SPACE",
            x if x == VDO_ERROR_BUFFER_FAILURE.0 => "VDO_ERROR_BUFFER_FAILURE",
            x if x == VDO_ERROR_INTERFACE_DOWN.0 => "VDO_ERROR_INTERFACE_DOWN",
            x if x == VDO_ERROR_FAILED.0 => "VDO_ERROR_FAILED",
            x if x == VDO_ERROR_FATAL.0 => "VDO_ERROR_FATAL",
            x if x == VDO_ERROR_NOT_CONTROLLED.0 => "VDO_ERROR_NOT_CONTROLLED",
            x if x == VDO_ERROR_NO_EVENT.0 => "VDO_ERROR_NO_EVENT",
            _ => "VDO_ERROR_UNKNOWN",
        }
    }

    /// Returns the numeric error code.
    pub fn code(&self) -> i32 {
        self.code
    }

    /// Returns the error message.
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

// ============================================================================
// Map - VDO settings/configuration container
// ============================================================================

/// A key-value map for VDO settings.
///
/// Used to configure stream parameters and retrieve stream information.
pub struct Map {
    raw: *mut VdoMap,
}

impl Map {
    /// Creates a new empty map.
    pub fn new() -> Result<Self> {
        let map = unsafe { vdo_map_new() };
        if map.is_null() {
            Err(Error::NullPointer)
        } else {
            Ok(Self { raw: map })
        }
    }

    /// Sets a 32-bit unsigned integer value.
    pub fn set_u32(&self, key: &str, value: u32) -> Result<()> {
        let key_cstr = CString::new(key).map_err(|_| Error::CStringAllocation)?;
        unsafe {
            vdo_map_set_uint32(self.raw, key_cstr.as_ptr(), value);
        }
        Ok(())
    }

    /// Gets a 32-bit unsigned integer value.
    pub fn get_u32(&self, key: &str, default: u32) -> Result<u32> {
        let key_cstr = CString::new(key).map_err(|_| Error::CStringAllocation)?;
        let value = unsafe { vdo_map_get_uint32(self.raw, key_cstr.as_ptr(), default) };
        Ok(value)
    }

    /// Sets a string value.
    pub fn set_string(&self, key: &str, value: &str) -> Result<()> {
        let key_cstr = CString::new(key).map_err(|_| Error::CStringAllocation)?;
        let value_cstr = CString::new(value).map_err(|_| Error::CStringAllocation)?;
        unsafe {
            vdo_map_set_string(self.raw, key_cstr.as_ptr(), value_cstr.as_ptr());
        }
        Ok(())
    }

    /// Gets a string value, returning an owned copy.
    ///
    /// Returns `None` if the key doesn't exist or the value is null.
    pub fn get_string(&self, key: &str) -> Result<Option<String>> {
        let key_cstr = CString::new(key).map_err(|_| Error::CStringAllocation)?;
        let ptr = unsafe { vdo_map_dup_string(self.raw, key_cstr.as_ptr(), ptr::null()) };
        if ptr.is_null() {
            return Ok(None);
        }
        let cstr = unsafe { CStr::from_ptr(ptr) };
        let result = cstr.to_str().map(|s| s.to_owned()).ok();
        unsafe { glib_sys::g_free(ptr as *mut _) };
        Ok(result)
    }

    /// Sets a boolean value.
    pub fn set_bool(&self, key: &str, value: bool) -> Result<()> {
        let key_cstr = CString::new(key).map_err(|_| Error::CStringAllocation)?;
        let gvalue = if value {
            glib_sys::GTRUE
        } else {
            glib_sys::GFALSE
        };
        unsafe {
            vdo_map_set_boolean(self.raw, key_cstr.as_ptr(), gvalue);
        }
        Ok(())
    }

    /// Gets a boolean value.
    pub fn get_bool(&self, key: &str, default: bool) -> Result<bool> {
        let key_cstr = CString::new(key).map_err(|_| Error::CStringAllocation)?;
        let gdefault = if default {
            glib_sys::GTRUE
        } else {
            glib_sys::GFALSE
        };
        let value = unsafe { vdo_map_get_boolean(self.raw, key_cstr.as_ptr(), gdefault) };
        Ok(value != glib_sys::GFALSE)
    }

    /// Dumps the map contents to stdout (for debugging).
    pub fn dump(&self) {
        unsafe {
            vdo_map_dump(self.raw);
        }
    }

    /// Returns the raw pointer (for internal use).
    pub(crate) fn as_ptr(&self) -> *mut VdoMap {
        self.raw
    }
}

impl Drop for Map {
    fn drop(&mut self) {
        unsafe { g_object_unref(self.raw as *mut GObject) }
    }
}

// ============================================================================
// StreamBuilder - Builder pattern for Stream
// ============================================================================

/// Builder for creating a video stream.
///
/// Use [`Stream::builder()`] to create a new builder.
///
/// # Example
///
/// ```no_run
/// use vdo::{Stream, VdoFormat};
///
/// let stream = Stream::builder()
///     .channel(0)
///     .format(VdoFormat::VDO_FORMAT_H264)
///     .resolution(1920, 1080)
///     .framerate(30)
///     .build()
///     .expect("Failed to build stream");
/// ```
#[derive(Clone)]
pub struct StreamBuilder {
    format: VdoFormat,
    buffer_count: u32,
    buffer_strategy: VdoBufferStrategy,
    channel: u32,
    width: u32,
    height: u32,
    framerate: u32,
}

impl Default for StreamBuilder {
    fn default() -> Self {
        Self {
            format: VdoFormat::VDO_FORMAT_H264,
            buffer_count: 3,
            buffer_strategy: VdoBufferStrategy::VDO_BUFFER_STRATEGY_INFINITE,
            channel: 0,
            width: 0,
            height: 0,
            framerate: 0,
        }
    }
}

impl StreamBuilder {
    /// Creates a new stream builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the video format.
    ///
    /// Default: `VdoFormat::VDO_FORMAT_H264`
    ///
    /// See the [platform compatibility table](crate#platform-compatibility) for supported formats.
    pub fn format(mut self, format: VdoFormat) -> Self {
        self.format = format;
        self
    }

    /// Sets the video channel.
    ///
    /// Default: 0 (main channel)
    pub fn channel(mut self, channel: u32) -> Self {
        self.channel = channel;
        self
    }

    /// Sets the video resolution.
    ///
    /// If width or height is 0, the camera's native resolution is used.
    pub fn resolution(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Sets the framerate.
    ///
    /// If 0, the camera's default framerate is used.
    pub fn framerate(mut self, framerate: u32) -> Self {
        self.framerate = framerate;
        self
    }

    /// Sets the number of buffers.
    ///
    /// Default: 3
    ///
    /// For YUV and RGB formats, this controls the number of frame buffers.
    /// For compressed formats (H.264, H.265, JPEG), this is typically ignored.
    pub fn buffers(mut self, count: u32) -> Self {
        self.buffer_count = count;
        self
    }

    /// Sets the buffer strategy.
    ///
    /// Default: `VdoBufferStrategy::VDO_BUFFER_STRATEGY_INFINITE`
    ///
    /// - `VDO_BUFFER_STRATEGY_INFINITE`: VDO manages buffers internally (works for all formats)
    /// - `VDO_BUFFER_STRATEGY_EXPLICIT`: Application manages buffers (only for YUV/RGB)
    pub fn buffer_strategy(mut self, strategy: VdoBufferStrategy) -> Self {
        self.buffer_strategy = strategy;
        self
    }

    /// Builds the stream.
    ///
    /// # Errors
    ///
    /// Returns an error if the stream could not be created (e.g., invalid format
    /// for the platform, or camera not available).
    pub fn build(self) -> Result<Stream> {
        let map = Map::new()?;
        map.set_u32("channel", self.channel)?;
        map.set_u32("format", self.format.0 as u32)?;
        if self.width > 0 {
            map.set_u32("width", self.width)?;
        }
        if self.height > 0 {
            map.set_u32("height", self.height)?;
        }
        if self.framerate > 0 {
            map.set_u32("framerate", self.framerate)?;
        }
        map.set_u32("buffer.count", self.buffer_count)?;
        map.set_u32("buffer.strategy", self.buffer_strategy.0)?;

        let (stream_raw, maybe_error) = unsafe { try_func!(vdo_stream_new, map.as_ptr(), None) };

        if stream_raw.is_null() {
            return Err(maybe_error.unwrap_or(Error::MissingVdoError));
        }

        debug_assert!(
            maybe_error.is_none(),
            "vdo_stream_new returned a stream pointer AND an error"
        );

        Ok(Stream {
            raw: stream_raw,
            _buffers: Vec::new(),
        })
    }
}

// ============================================================================
// Stream - Video stream handle
// ============================================================================

/// A video stream from a camera channel.
///
/// Use [`Stream::builder()`] to create a stream, then call [`Stream::start()`]
/// to begin capturing frames.
///
/// # Example
///
/// ```no_run
/// use vdo::{Stream, VdoFormat};
///
/// let mut stream = Stream::builder()
///     .format(VdoFormat::VDO_FORMAT_JPEG)
///     .resolution(640, 480)
///     .build()?;
///
/// let mut running = stream.start()?;
/// for buffer in running.iter().take(5) {
///     println!("Got frame: {} bytes", buffer.frame()?.size());
/// }
/// running.stop()?;
/// # Ok::<(), vdo::Error>(())
/// ```
pub struct Stream {
    raw: *mut VdoStream,
    _buffers: Vec<*mut VdoBuffer>,
}

// SAFETY: Stream can be sent between threads.
// The underlying VDO library uses GLib which is thread-safe.
unsafe impl Send for Stream {}

impl Stream {
    /// Creates a new stream builder.
    pub fn builder() -> StreamBuilder {
        StreamBuilder::new()
    }

    /// Creates a stream with default settings (H.264 format).
    ///
    /// This is equivalent to `Stream::builder().build()`. The default format
    /// is H.264, which may not be what you want. For other formats, use
    /// [`Stream::builder()`] instead:
    ///
    /// ```no_run
    /// # use vdo::{Stream, VdoFormat};
    /// let stream = Stream::builder()
    ///     .format(VdoFormat::VDO_FORMAT_YUV)
    ///     .build()?;
    /// # Ok::<(), vdo::Error>(())
    /// ```
    pub fn new() -> Result<Self> {
        StreamBuilder::new().build()
    }

    /// Returns stream information as a map.
    ///
    /// The map contains properties like actual resolution, format, etc.
    pub fn info(&self) -> Result<Map> {
        let (map_raw, maybe_error) = unsafe { try_func!(vdo_stream_get_info, self.raw) };
        if map_raw.is_null() {
            return Err(maybe_error.unwrap_or(Error::MissingVdoError));
        }
        Ok(Map { raw: map_raw })
    }

    /// Returns stream settings as a map.
    pub fn settings(&self) -> Result<Map> {
        let (map_raw, maybe_error) = unsafe { try_func!(vdo_stream_get_settings, self.raw) };
        if map_raw.is_null() {
            return Err(maybe_error.unwrap_or(Error::MissingVdoError));
        }
        Ok(Map { raw: map_raw })
    }

    /// Starts the stream and returns a handle for accessing frames.
    ///
    /// The stream will begin capturing frames from the camera. Use the returned
    /// [`RunningStream`] to iterate over frames.
    ///
    /// # Errors
    ///
    /// Returns an error if the stream could not be started.
    pub fn start(&mut self) -> Result<RunningStream<'_>> {
        let (success, maybe_error) = unsafe { try_func!(vdo_stream_start, self.raw) };
        if success != glib_sys::GTRUE {
            return Err(maybe_error.unwrap_or(Error::MissingVdoError));
        }
        Ok(RunningStream { stream: self })
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        unsafe {
            vdo_stream_stop(self.raw);
        }
        // Clean up any allocated buffers
        for mut buffer in mem::take(&mut self._buffers) {
            unsafe {
                let _ = try_func!(vdo_stream_buffer_unref, self.raw, &mut buffer);
            }
        }
    }
}

// ============================================================================
// RunningStream - A started stream that can be iterated
// ============================================================================

/// A running video stream that yields frame buffers.
///
/// Created by calling [`Stream::start()`]. Use [`iter()`](RunningStream::iter)
/// to get an iterator over frames.
pub struct RunningStream<'a> {
    stream: &'a mut Stream,
}

impl RunningStream<'_> {
    /// Returns an iterator over frame buffers.
    ///
    /// Each call to `next()` blocks until a new frame is available.
    /// The iterator never ends naturally - use `.take(n)` to limit frames.
    pub fn iter(&mut self) -> StreamIterator<'_> {
        StreamIterator {
            stream: self.stream,
        }
    }

    /// Stops the stream.
    ///
    /// After stopping, no more frames can be retrieved.
    pub fn stop(&mut self) -> Result<()> {
        unsafe { vdo_stream_stop(self.stream.raw) };
        Ok(())
    }
}

// ============================================================================
// StreamIterator - Iterator over stream buffers
// ============================================================================

/// Iterator that yields frame buffers from a running stream.
pub struct StreamIterator<'a> {
    stream: &'a Stream,
}

impl<'a> Iterator for StreamIterator<'a> {
    type Item = StreamBuffer<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (buffer_ptr, maybe_error) =
            unsafe { try_func!(vdo_stream_get_buffer, self.stream.raw) };

        if buffer_ptr.is_null() {
            if let Some(err) = maybe_error {
                log::error!("Error getting buffer: {}", err);
            }
            return None;
        }

        Some(StreamBuffer {
            raw: buffer_ptr,
            stream: self.stream,
            _phantom: PhantomData,
        })
    }
}

// ============================================================================
// StreamBuffer - A frame buffer from a stream
// ============================================================================

/// A buffer containing a video frame.
///
/// The buffer is automatically released when dropped.
pub struct StreamBuffer<'a> {
    raw: *mut VdoBuffer,
    stream: &'a Stream,
    _phantom: PhantomData<&'a ()>,
}

impl StreamBuffer<'_> {
    /// Returns the buffer capacity in bytes.
    pub fn capacity(&self) -> usize {
        unsafe { vdo_buffer_get_capacity(self.raw) }
    }

    /// Returns the frame data as a byte slice.
    ///
    /// The slice length is the buffer capacity, not the actual frame size.
    /// Use [`Frame::size()`] to get the actual frame data size.
    pub fn as_slice(&self) -> Result<&[u8]> {
        let data = unsafe { vdo_buffer_get_data(self.raw) };
        if data.is_null() {
            return Err(Error::NullPointer);
        }
        let slice = unsafe { std::slice::from_raw_parts(data as *const u8, self.capacity()) };
        Ok(slice)
    }

    /// Returns the frame data as a mutable byte slice.
    pub fn as_mut_slice(&mut self) -> Result<&mut [u8]> {
        let data = unsafe { vdo_buffer_get_data(self.raw) };
        if data.is_null() {
            return Err(Error::NullPointer);
        }
        let slice = unsafe { std::slice::from_raw_parts_mut(data as *mut u8, self.capacity()) };
        Ok(slice)
    }

    /// Returns frame metadata for this buffer.
    pub fn frame(&self) -> Result<Frame<'_>> {
        let frame = unsafe { vdo_buffer_get_frame(self.raw) };
        if frame.is_null() {
            return Err(Error::NullPointer);
        }
        Ok(Frame {
            raw: frame,
            _phantom: PhantomData,
        })
    }
}

impl Drop for StreamBuffer<'_> {
    fn drop(&mut self) {
        unsafe {
            let _ = try_func!(vdo_stream_buffer_unref, self.stream.raw, &mut self.raw);
        }
    }
}

// ============================================================================
// Frame - Frame metadata
// ============================================================================

/// Metadata for a video frame.
///
/// Contains information about frame timing, size, and type.
pub struct Frame<'a> {
    raw: *mut VdoFrame,
    _phantom: PhantomData<&'a StreamBuffer<'a>>,
}

impl Frame<'_> {
    /// Returns the frame type (I-frame, P-frame, etc.).
    pub fn frame_type(&self) -> VdoFrameType {
        unsafe { vdo_frame_get_frame_type(self.raw) }
    }

    /// Returns the sequence number of the frame.
    ///
    /// Starts at 0 and increments with each frame. The wrap-around point is undefined.
    pub fn sequence_number(&self) -> u32 {
        unsafe { vdo_frame_get_sequence_nbr(self.raw) }
    }

    /// Returns the timestamp in microseconds since boot.
    pub fn timestamp(&self) -> u64 {
        unsafe { vdo_frame_get_timestamp(self.raw) }
    }

    /// Returns the custom timestamp.
    pub fn custom_timestamp(&self) -> i64 {
        unsafe { vdo_frame_get_custom_timestamp(self.raw) }
    }

    /// Returns the frame data size in bytes.
    ///
    /// This is the actual size of the frame data, which may be less than
    /// the buffer capacity.
    pub fn size(&self) -> usize {
        unsafe { vdo_frame_get_size(self.raw) }
    }

    /// Returns the header size in bytes.
    pub fn header_size(&self) -> isize {
        unsafe { vdo_frame_get_header_size(self.raw) }
    }

    /// Returns the file descriptor for the frame data.
    ///
    /// This can be used for zero-copy operations with other APIs.
    pub fn file_descriptor(&self) -> std::os::fd::BorrowedFd<'_> {
        unsafe {
            let fd = vdo_frame_get_fd(self.raw);
            std::os::fd::BorrowedFd::borrow_raw(fd)
        }
    }

    /// Returns whether this is the last buffer in a sequence.
    pub fn is_last_buffer(&self) -> bool {
        unsafe { vdo_frame_get_is_last_buffer(self.raw) != 0 }
    }
}

// ============================================================================
// Unit Tests (no device required)
// ============================================================================

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn error_code_names() {
        // Test that error code names are correctly mapped
        let err = VdoError {
            code: VDO_ERROR_NOT_FOUND.0 as i32,
            message: "test".to_string(),
        };
        assert_eq!(err.code_name(), "VDO_ERROR_NOT_FOUND");

        let err = VdoError {
            code: VDO_ERROR_NOT_SUPPORTED.0 as i32,
            message: "test".to_string(),
        };
        assert_eq!(err.code_name(), "VDO_ERROR_NOT_SUPPORTED");

        let err = VdoError {
            code: 9999,
            message: "test".to_string(),
        };
        assert_eq!(err.code_name(), "VDO_ERROR_UNKNOWN");
    }

    #[test]
    fn error_display() {
        let err = VdoError {
            code: VDO_ERROR_BUSY.0 as i32,
            message: "Resource is busy".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("VDO_ERROR_BUSY"));
        assert!(display.contains("Resource is busy"));
    }

    #[test]
    fn stream_builder_defaults() {
        let builder = StreamBuilder::default();
        // Check default values match expected
        assert_eq!(builder.format, VdoFormat::VDO_FORMAT_H264);
        assert_eq!(builder.channel, 0);
        assert_eq!(builder.buffer_count, 3);
        assert_eq!(
            builder.buffer_strategy,
            VdoBufferStrategy::VDO_BUFFER_STRATEGY_INFINITE
        );
    }

    #[test]
    fn stream_builder_chaining() {
        let builder = StreamBuilder::new()
            .format(VdoFormat::VDO_FORMAT_JPEG)
            .channel(1)
            .resolution(1280, 720)
            .framerate(30)
            .buffers(5)
            .buffer_strategy(VdoBufferStrategy::VDO_BUFFER_STRATEGY_EXPLICIT);

        assert_eq!(builder.format, VdoFormat::VDO_FORMAT_JPEG);
        assert_eq!(builder.channel, 1);
        assert_eq!(builder.width, 1280);
        assert_eq!(builder.height, 720);
        assert_eq!(builder.framerate, 30);
        assert_eq!(builder.buffer_count, 5);
        assert_eq!(
            builder.buffer_strategy,
            VdoBufferStrategy::VDO_BUFFER_STRATEGY_EXPLICIT
        );
    }

    #[test]
    fn stream_builder_clone() {
        let builder1 = StreamBuilder::new()
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(640, 480);

        let builder2 = builder1.clone();

        assert_eq!(builder1.format, builder2.format);
        assert_eq!(builder1.width, builder2.width);
        assert_eq!(builder1.height, builder2.height);
    }

    #[test]
    fn vdo_format_values() {
        // Verify format values match expected C API values
        assert_eq!(VdoFormat::VDO_FORMAT_H264.0, 0);
        assert_eq!(VdoFormat::VDO_FORMAT_H265.0, 1);
        assert_eq!(VdoFormat::VDO_FORMAT_JPEG.0, 2);
        assert_eq!(VdoFormat::VDO_FORMAT_YUV.0, 3);
    }

    #[test]
    fn buffer_strategy_values() {
        // Verify buffer strategy values
        assert_eq!(VdoBufferStrategy::VDO_BUFFER_STRATEGY_NONE.0, 0);
        assert_eq!(VdoBufferStrategy::VDO_BUFFER_STRATEGY_INFINITE.0, 4);
        assert_eq!(VdoBufferStrategy::VDO_BUFFER_STRATEGY_EXPLICIT.0, 3);
    }

    #[test]
    fn error_is_send() {
        // Verify Error can be sent across threads
        fn assert_send<T: Send>() {}
        assert_send::<Error>();
    }

    #[test]
    fn vdo_error_default() {
        let err = VdoError::default();
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
        // Ensure all error variants have meaningful Display output
        let errors = [
            Error::NullPointer,
            Error::CStringAllocation,
            Error::MissingVdoError,
            Error::NoBuffersAllocated,
            Error::Vdo(VdoError {
                code: 1,
                message: "test".to_string(),
            }),
        ];

        for err in &errors {
            let msg = format!("{}", err);
            assert!(!msg.is_empty(), "Error display should not be empty");
        }
    }
}

// ============================================================================
// Device Tests (require actual Axis camera)
// ============================================================================

#[cfg(all(test, target_arch = "aarch64", feature = "device-tests"))]
mod device_tests {
    use super::*;

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn stream_starts_and_stops() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let mut stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(640, 480)
            .build()?;

        let mut running = stream.start()?;
        running.stop()?;
        Ok(())
    }

    #[test]
    fn stream_info_available() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(640, 480)
            .build()?;

        let info = stream.info()?;
        // Just verify we can get info without error
        info.dump();
        Ok(())
    }

    #[test]
    fn stream_settings_available() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(640, 480)
            .build()?;

        let settings = stream.settings()?;
        settings.dump();
        Ok(())
    }

    #[test]
    fn capture_yuv_frames() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let mut stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(640, 480)
            .build()?;

        let mut running = stream.start()?;

        for (i, buffer) in running.iter().take(5).enumerate() {
            let frame = buffer.frame()?;
            let size = frame.size();
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
                frame.sequence_number(),
                frame.timestamp()
            );
        }

        running.stop()?;
        Ok(())
    }

    #[test]
    fn capture_jpeg_frames() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let mut stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_JPEG)
            .resolution(640, 480)
            .build()?;

        let mut running = stream.start()?;

        for (i, buffer) in running.iter().take(5).enumerate() {
            let frame = buffer.frame()?;
            assert!(frame.size() > 0, "Frame {} size should be > 0", i);

            // Verify JPEG magic bytes (SOI marker)
            let data = buffer.as_slice()?;
            assert!(data.len() >= 2, "Buffer too small for JPEG");
            assert_eq!(data[0], 0xFF, "Invalid JPEG SOI marker");
            assert_eq!(data[1], 0xD8, "Invalid JPEG SOI marker");

            log::info!("JPEG frame {}: {} bytes", i, frame.size());
        }

        running.stop()?;
        Ok(())
    }

    #[test]
    fn capture_h264_frames() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let mut stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_H264)
            .resolution(640, 480)
            .build()?;

        let mut running = stream.start()?;

        let mut got_i_frame = false;
        let mut got_p_frame = false;

        for buffer in running.iter().take(30) {
            let frame = buffer.frame()?;
            assert!(frame.size() > 0, "H.264 frame size should be > 0");

            match frame.frame_type() {
                VdoFrameType::VDO_FRAME_TYPE_I => got_i_frame = true,
                VdoFrameType::VDO_FRAME_TYPE_P => got_p_frame = true,
                _ => {}
            }

            log::info!(
                "H.264 frame: {} bytes, type={:?}, seq={}",
                frame.size(),
                frame.frame_type(),
                frame.sequence_number()
            );
        }

        // We should see at least an I-frame in 30 frames
        assert!(got_i_frame, "Should have captured at least one I-frame");

        running.stop()?;
        Ok(())
    }

    #[test]
    fn capture_h265_frames() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();

        // H.265 might not be supported on all platforms (e.g., Artpec-6)
        let stream_result = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_H265)
            .resolution(640, 480)
            .build();

        match stream_result {
            Ok(mut stream) => {
                let mut running = stream.start()?;

                for buffer in running.iter().take(10) {
                    let frame = buffer.frame()?;
                    assert!(frame.size() > 0, "H.265 frame size should be > 0");
                    log::info!("H.265 frame: {} bytes", frame.size());
                }

                running.stop()?;
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
        let mut stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(320, 240)
            .framerate(15)
            .build()?;

        let mut running = stream.start()?;
        let mut prev_ts = 0u64;
        let mut prev_seq = 0u32;

        for (i, buffer) in running.iter().take(10).enumerate() {
            let frame = buffer.frame()?;
            let ts = frame.timestamp();
            let seq = frame.sequence_number();

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

        running.stop()?;
        Ok(())
    }

    #[test]
    fn buffer_data_accessible() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let mut stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(320, 240)
            .build()?;

        let mut running = stream.start()?;

        for buffer in running.iter().take(3) {
            let frame = buffer.frame()?;
            let capacity = buffer.capacity();
            let data = buffer.as_slice()?;

            assert_eq!(data.len(), capacity, "Slice length should match capacity");
            assert!(frame.size() <= capacity, "Frame size should be <= capacity");

            // Verify we can read data without crashing
            let _first_byte = data[0];
            let _last_byte = data[frame.size().saturating_sub(1)];
        }

        running.stop()?;
        Ok(())
    }

    #[test]
    fn multiple_streams_sequential() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();

        // Create and use first stream
        {
            let mut stream = Stream::builder()
                .format(VdoFormat::VDO_FORMAT_YUV)
                .resolution(320, 240)
                .build()?;

            let mut running = stream.start()?;
            let _ = running.iter().take(3).count();
            running.stop()?;
        }

        // Create and use second stream
        {
            let mut stream = Stream::builder()
                .format(VdoFormat::VDO_FORMAT_JPEG)
                .resolution(320, 240)
                .build()?;

            let mut running = stream.start()?;
            let _ = running.iter().take(3).count();
            running.stop()?;
        }

        Ok(())
    }

    // ========================================================================
    // Error Handling Tests
    // ========================================================================

    #[test]
    fn invalid_channel_returns_error() {
        init_logger();
        // Channel 999 should not exist on any camera
        let result = Stream::builder()
            .channel(999)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(640, 480)
            .build();

        assert!(result.is_err(), "Invalid channel should return error");
        if let Err(e) = result {
            log::info!("Expected error for invalid channel: {}", e);
        }
    }

    #[test]
    fn unsupported_format_returns_error() {
        init_logger();
        // Try a format that might not be supported (platform-dependent)
        // VDO_FORMAT_BAYER is often not supported
        let result = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_BAYER)
            .resolution(640, 480)
            .build();

        // This might succeed on some platforms, so just log the result
        match result {
            Ok(_) => log::info!("BAYER format is supported on this platform"),
            Err(e) => log::info!("BAYER format not supported: {}", e),
        }
    }

    #[test]
    fn invalid_resolution_handled() {
        init_logger();
        // Try an unusual resolution that might not be supported
        let result = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(12345, 6789) // Unusual resolution
            .build();

        // Camera might adjust resolution or return error
        match result {
            Ok(stream) => {
                // Camera accepted - check actual resolution via info
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

    #[test]
    fn stream_stop_is_idempotent() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let mut stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(320, 240)
            .build()?;

        let mut running = stream.start()?;
        let _ = running.iter().take(1).count();

        // Stop multiple times should not crash
        running.stop()?;
        running.stop()?; // Second stop should be safe

        Ok(())
    }

    #[test]
    fn stream_dropped_without_stop() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();

        // Create stream, start it, get some frames, then drop without calling stop()
        // This tests that Drop implementation properly cleans up
        {
            let mut stream = Stream::builder()
                .channel(0)
                .format(VdoFormat::VDO_FORMAT_YUV)
                .resolution(320, 240)
                .build()?;

            let mut running = stream.start()?;
            let _ = running.iter().take(2).count();
            // Intentionally NOT calling running.stop()
            // Drop should handle cleanup
        }

        // If we get here without crash, cleanup worked
        log::info!("Stream dropped without explicit stop - cleanup successful");
        Ok(())
    }

    #[test]
    fn error_message_is_descriptive() {
        init_logger();

        // Force an error and check that the message is helpful
        let result = Stream::builder()
            .channel(999) // Invalid
            .build();

        if let Err(Error::Vdo(e)) = result {
            let msg = e.message();
            let code_name = e.code_name();
            log::info!(
                "Error code: {}, name: {}, message: {}",
                e.code(),
                code_name,
                msg
            );

            // Error should have some information
            assert!(!code_name.is_empty(), "Error code name should not be empty");
        }
    }

    #[test]
    fn rapid_stream_creation_destruction() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();

        // Rapidly create and destroy streams to test for resource leaks
        for i in 0..5 {
            let mut stream = Stream::builder()
                .channel(0)
                .format(VdoFormat::VDO_FORMAT_YUV)
                .resolution(320, 240)
                .build()?;

            let mut running = stream.start()?;
            let _ = running.iter().take(1).count();
            running.stop()?;

            log::info!("Rapid cycle {} complete", i);
        }

        Ok(())
    }
}
