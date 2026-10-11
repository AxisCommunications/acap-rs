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
//! # SDK Compatibility
//!
//! By default, this crate only uses API that every supported AXIS OS version has. Enable the
//! `axis-os-12` feature to also use API that needs AXIS OS 12 or newer:
//!
//! ```toml
//! vdo = { version = "...", features = ["axis-os-12"] }
//! ```
//!
//! Building with the feature requires SDK 12 or newer.
//!
//! # Example
//!
//! ```no_run
//! use vdo::{Resolution, StreamBuilder, VdoFormat};
//!
//! let stream = StreamBuilder::new()
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
use std::{
    fmt::{Debug, Display},
    ptr,
};

use glib_sys::GError;
use gobject_sys::{g_object_unref, GObject};
pub use map::{CStringPtr, Map};
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
    #[error("Missing error data from VDO library")]
    MissingVdoError,
}

/// Error from the VDO library.
pub struct VdoError {
    domain: u32,
    code: i32,
    message: String,
}

impl VdoError {
    fn from_gerror(gerror: *mut GError) -> Self {
        if gerror.is_null() {
            return VdoError {
                domain: 0,
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
            domain: g_error.domain,
            code: g_error.code,
            message,
        }
    }

    /// Whether this is `VDO_ERROR_NO_DATA` from the VDO error domain.
    fn is_no_data(&self) -> bool {
        // SAFETY: vdo_error_quark only registers/returns a GQuark.
        self.domain == unsafe { vdo_sys::vdo_error_quark() }
            && self.code == vdo_sys::VDO_ERROR_NO_DATA.0 as i32
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
            #[cfg(feature = "axis-os-12")]
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
/// Use [`StreamBuilder::new()`] to create a new builder.
///
/// # Example
///
/// ```no_run
/// use vdo::{Resolution, StreamBuilder, VdoFormat};
///
/// let stream = StreamBuilder::new()
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
    gop_length: u32,
}

impl Default for StreamBuilder {
    fn default() -> Self {
        Self {
            format: VdoFormat::VDO_FORMAT_H264,
            buffer_count: 3,
            channel: 0,
            resolution: Resolution::Native,
            framerate: 0,
            gop_length: 0,
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

    /// GOP length — the number of frames between key frames. If 0 (default),
    /// the camera's configured GOP is used, which on cameras with dynamic-GOP
    /// / Zipstream can make key frames (and the SPS/PPS parameter sets VDO
    /// emits ahead of each key frame) very sparse. Set a small fixed value to
    /// force key frames to recur at a predictable interval. Passed to
    /// `vdo_stream_new` as the `gop_length` setting.
    pub fn gop_length(mut self, gop_length: u32) -> Self {
        self.gop_length = gop_length;
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
        let mut map = Map::new();
        map.set_u32(c"channel", self.channel);
        map.set_u32(c"format", self.format.0 as u32);
        if let Resolution::Exact { width, height } = self.resolution {
            map.set_u32(c"width", width);
            map.set_u32(c"height", height);
        }
        if self.framerate > 0 {
            map.set_u32(c"framerate", self.framerate);
        }
        if self.gop_length > 0 {
            map.set_u32(c"gop_length", self.gop_length);
        }
        map.set_u32(c"buffer.count", self.buffer_count);
        // Always use INFINITE strategy; EXPLICIT is not exposed because it
        // requires unsafe application-managed buffer allocation.
        map.set_u32(
            c"buffer.strategy",
            VdoBufferStrategy::VDO_BUFFER_STRATEGY_INFINITE.0,
        );

        Stream::create(&map)
    }
}

/// An existing VDO stream, from [`list_streams()`].
#[derive(Debug, Clone)]
pub struct StreamInfo {
    /// The stream's id, as [`Stream::id()`] reports it.
    pub id: u32,
    /// The stream's settings (`vdo_stream_get_settings`).
    pub settings: Map,
}

/// Every stream VDO currently has, from any client, with its settings.
/// Streams whose settings can't be read are left out.
pub fn list_streams() -> std::result::Result<Vec<StreamInfo>, Error> {
    let mut error: *mut GError = ptr::null_mut();
    let list = unsafe { vdo_sys::vdo_stream_get_all(&mut error) };
    if !error.is_null() {
        return Err(Error::Vdo(VdoError::from_gerror(error)));
    }
    let mut streams = Vec::new();
    let mut node = list;
    while !node.is_null() {
        // SAFETY: node is a live element of the list VDO returned; its data
        // is a VdoStream we own a reference to and release below.
        let raw = unsafe { (*node).data } as *mut VdoStream;
        if !raw.is_null() {
            let id = unsafe { vdo_sys::vdo_stream_get_id(raw) };
            let (map_raw, maybe_error) =
                unsafe { try_func!(vdo_sys::vdo_stream_get_settings, raw) };
            if !map_raw.is_null() {
                // SAFETY: non-null and owned by us, per vdo_stream_get_settings.
                let settings = unsafe { Map::from_raw(map_raw) };
                streams.push(StreamInfo { id, settings });
            } else {
                log::debug!("skipping VDO stream {id}: settings unreadable: {maybe_error:?}");
            }
            unsafe { g_object_unref(raw as *mut GObject) };
        }
        node = unsafe { (*node).next };
    }
    unsafe { glib_sys::g_list_free(list) };
    Ok(streams)
}

/// A video stream from a camera channel.
///
/// Use [`StreamBuilder`] to create a stream, then call [`Stream::start()`]
/// to begin capturing frames. Starting consumes the `Stream` and returns a
/// [`RunningStream`].
///
/// # Example
///
/// ```no_run
/// use vdo::{Resolution, StreamBuilder, VdoFormat};
///
/// let stream = StreamBuilder::new()
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
    /// Equivalent to `StreamBuilder::new().build()` (H.264 format, native resolution).
    pub fn new() -> std::result::Result<Self, Error> {
        StreamBuilder::new().build()
    }

    /// Creates a stream from exactly `settings`, except that the buffer
    /// strategy is always INFINITE (the only one this crate supports, as in
    /// [`StreamBuilder::build()`]).
    ///
    /// Use this to join an encode that is already running: when `settings`
    /// is an exact copy of a stream whose `sharable` setting is true (from
    /// [`list_streams()`], with the `id`, `identity` and `intent` entries
    /// removed), VDO gives the new stream the *same* encode instead of
    /// starting another, and [`Stream::id()`] returns the existing stream's
    /// id. The copy must be exact, buffer settings included; any difference
    /// starts a new encode. Because the buffer strategy is forced to
    /// INFINITE, a sharable stream using another strategy can't be joined. A joined stream may be non-blocking
    /// (`socket.blocking` false), which [`RunningStream::next_buffer()`]
    /// handles.
    ///
    /// ```no_run
    /// let existing = vdo::list_streams()?
    ///     .into_iter()
    ///     .find(|s| s.settings.get_bool(c"sharable", false))
    ///     .expect("a sharable stream");
    /// let mut settings = existing.settings.clone();
    /// for key in [c"id", c"identity", c"intent"] {
    ///     settings.remove(key);
    /// }
    /// let stream = vdo::Stream::from_settings(&settings)?;
    /// assert_eq!(stream.id(), existing.id);
    /// # Ok::<(), vdo::Error>(())
    /// ```
    pub fn from_settings(settings: &Map) -> std::result::Result<Self, Error> {
        let mut map = settings.clone();
        map.set_u32(
            c"buffer.strategy",
            VdoBufferStrategy::VDO_BUFFER_STRATEGY_INFINITE.0,
        );
        Self::create(&map)
    }

    fn create(map: &Map) -> std::result::Result<Self, Error> {
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

    /// The stream's VDO id. For a stream that joined an existing encode (see
    /// [`Stream::from_settings()`]) this is the existing stream's id.
    pub fn id(&self) -> u32 {
        unsafe { vdo_sys::vdo_stream_get_id(self.raw) }
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
    /// stream via [`StreamBuilder`] to retry.
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
    ///
    /// A stream that joined an existing encode can be non-blocking, where
    /// VDO reports `VDO_ERROR_NO_DATA` until a frame is ready; this waits on
    /// the stream's file descriptor and retries, so it blocks either way. It
    /// returns that error if the descriptor is hung up or in error.
    pub fn next_buffer(&self) -> std::result::Result<StreamBuffer<'_>, Error> {
        let buffer_ptr = loop {
            let (buffer_ptr, maybe_error) =
                unsafe { try_func!(vdo_sys::vdo_stream_get_buffer, self.stream.raw) };
            if !buffer_ptr.is_null() {
                break buffer_ptr;
            }
            match maybe_error {
                Some(Error::Vdo(e)) if e.is_no_data() => {
                    if !self.wait_readable()? {
                        return Err(Error::Vdo(e));
                    }
                }
                other => return Err(other.unwrap_or(Error::MissingVdoError)),
            }
        };

        Ok(StreamBuffer {
            raw: buffer_ptr,
            stream: &self.stream,
        })
    }

    /// The stream's VDO id; see [`Stream::id()`].
    pub fn id(&self) -> u32 {
        self.stream.id()
    }

    /// Waits up to a second for the stream's file descriptor to become
    /// readable. Returns `Ok(false)` if the descriptor is hung up, in error
    /// or invalid, or the wait fails other than by being interrupted, so the
    /// caller stops retrying.
    fn wait_readable(&self) -> std::result::Result<bool, Error> {
        let (fd, maybe_error) = unsafe { try_func!(vdo_sys::vdo_stream_get_fd, self.stream.raw) };
        if fd < 0 {
            return Err(maybe_error.unwrap_or(Error::MissingVdoError));
        }
        let mut pfd = glib_sys::GPollFD {
            fd,
            events: glib_sys::G_IO_IN as u16,
            revents: 0,
        };
        // SAFETY: pfd is a valid GPollFD for the duration of the call. A
        // bounded wait, so a quiet stream returns to vdo_stream_get_buffer
        // regularly rather than parking here.
        let ready = unsafe { glib_sys::g_poll(&mut pfd, 1, 1000) };
        if ready < 0 {
            // A signal interrupting the wait is not a broken stream.
            return Ok(std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted);
        }
        let broken = (glib_sys::G_IO_HUP | glib_sys::G_IO_ERR | glib_sys::G_IO_NVAL) as u16;
        Ok(pfd.revents & broken == 0)
    }

    /// The running stream's information (actual resolution, framerate,
    /// `peers`, statistics, …); see [`Stream::info()`].
    pub fn info(&self) -> std::result::Result<Map, Error> {
        self.stream.info()
    }

    /// The running stream's settings; see [`Stream::settings()`].
    pub fn settings(&self) -> std::result::Result<Map, Error> {
        self.stream.settings()
    }
}

/// A buffer containing a video frame from a running stream.
///
/// Since `VdoBuffer` and `VdoFrame` are the same type in the C API, all frame
/// metadata (size, timestamp, frame type, etc.) is accessed directly on this type.
///
/// The buffer borrows from the [`RunningStream`] that produced it and is
/// automatically unreferenced when dropped.
///
/// # Buffer Validity
///
/// The buffer data pointer and frame metadata remain valid until the buffer is
/// unreferenced on drop.
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

    /// Returns a copy of the frame data, excluding the header if one is present.
    ///
    /// Use [`as_slice()`](StreamBuffer::as_slice) for a raw view of the whole buffer.
    pub fn data_copy(&self) -> std::result::Result<Vec<u8>, Error> {
        let data = unsafe { vdo_sys::vdo_buffer_get_data(self.raw) };
        if data.is_null() {
            return Err(Error::NullPointer);
        }
        let offset = self.header_size().unwrap_or(0);
        let size = self.size();
        assert!(offset <= size, "expect header to fit within frame size");
        assert!(
            size <= self.capacity(),
            "expect frame size to fit within buffer capacity"
        );
        // SAFETY: offset..size lies within the mapped region of capacity bytes,
        // which is fully initialized at allocation time.
        let slice =
            unsafe { std::slice::from_raw_parts((data as *const u8).add(offset), size - offset) };
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

    pub fn is_last_buffer(&self) -> bool {
        unsafe { vdo_sys::vdo_frame_get_is_last_buffer(self.raw) != glib_sys::GFALSE }
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
    use expect_test::expect;

    use super::*;

    // `VDO_ERROR_NO_VIDEO` is 21 in SDK 12. SDK 1.15 does not declare it, but AXIS OS 12 may
    // still return it to apps built against SDK 1.15.
    const NO_VIDEO: i32 = 21;

    #[cfg(feature = "axis-os-12")]
    #[test]
    fn no_video_error_code_has_a_name() {
        let err = VdoError {
            domain: 0,
            code: NO_VIDEO,
            message: "test".to_string(),
        };
        expect!["VDO_ERROR_NO_VIDEO"].assert_eq(err.code_name());
    }

    #[cfg(not(feature = "axis-os-12"))]
    #[test]
    fn no_video_error_code_is_unknown() {
        let err = VdoError {
            domain: 0,
            code: NO_VIDEO,
            message: "test".to_string(),
        };
        expect!["VDO_ERROR_UNKNOWN"].assert_eq(err.code_name());
    }

    #[test]
    fn error_code_names() {
        let err = VdoError {
            domain: 0,
            code: vdo_sys::VDO_ERROR_NOT_FOUND.0 as i32,
            message: "test".to_string(),
        };
        expect!["VDO_ERROR_NOT_FOUND"].assert_eq(err.code_name());

        let err = VdoError {
            domain: 0,
            code: vdo_sys::VDO_ERROR_NOT_SUPPORTED.0 as i32,
            message: "test".to_string(),
        };
        expect!["VDO_ERROR_NOT_SUPPORTED"].assert_eq(err.code_name());

        let err = VdoError {
            domain: 0,
            code: 9999,
            message: "test".to_string(),
        };
        expect!["VDO_ERROR_UNKNOWN"].assert_eq(err.code_name());

        // Negative codes (from non-VDO GError domains) should map to UNKNOWN,
        // not wrap to a matching VDO constant.
        let err = VdoError {
            domain: 0,
            code: -1,
            message: "test".to_string(),
        };
        expect!["VDO_ERROR_UNKNOWN"].assert_eq(err.code_name());
    }

    #[test]
    fn no_data_requires_the_vdo_error_domain() {
        let foreign = VdoError {
            domain: 0,
            code: vdo_sys::VDO_ERROR_NO_DATA.0 as i32,
            message: "test".to_string(),
        };
        assert!(!foreign.is_no_data());
        let vdo = VdoError {
            domain: unsafe { vdo_sys::vdo_error_quark() },
            ..foreign
        };
        assert!(vdo.is_no_data());
    }

    #[test]
    fn error_display() {
        let err = VdoError {
            domain: 0,
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
            domain: 0,
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
        expect!["Missing error data from VDO library"]
            .assert_eq(&format!("{}", Error::MissingVdoError));
        let vdo = Error::Vdo(VdoError {
            domain: 0,
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
        let stream = StreamBuilder::new()
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
        let stream = StreamBuilder::new()
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
        let stream = StreamBuilder::new()
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
        let stream = StreamBuilder::new()
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
        let stream = StreamBuilder::new()
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
        let stream = StreamBuilder::new()
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
        let stream = StreamBuilder::new()
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

        let stream_result = StreamBuilder::new()
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
        let stream = StreamBuilder::new()
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
        let stream = StreamBuilder::new()
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
        let stream = StreamBuilder::new()
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
        let offset = buffer.header_size().unwrap_or(0);
        assert_eq!(copy.len(), buffer.size() - offset);

        // Verify copy matches the original slice, past the header if any
        let slice = buffer.as_slice()?;
        assert_eq!(&copy[..], &slice[offset..offset + copy.len()]);

        drop(buffer);
        drop(running);
        Ok(())
    }

    #[test]
    fn all_buffer_metadata_accessible() -> std::result::Result<(), Box<dyn std::error::Error>> {
        init_logger();
        let stream = StreamBuilder::new()
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
            let stream = StreamBuilder::new()
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
            let stream = StreamBuilder::new()
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

        let stream1 = StreamBuilder::new()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(Resolution::Exact {
                width: 320,
                height: 240,
            })
            .build()?;

        let stream2 = StreamBuilder::new()
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
        let result = StreamBuilder::new()
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
        let result = StreamBuilder::new()
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
        let result = StreamBuilder::new()
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
            let stream = StreamBuilder::new()
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

        let err = StreamBuilder::new()
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
            let stream = StreamBuilder::new()
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

        let mut map = Map::new();

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
