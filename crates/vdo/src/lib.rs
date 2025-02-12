//! Stream buffer strategy seems to be round robin. Allocate N buffers with
//! `vod_stream_buffer_alloc`, enqueue the buffers with,
//! `vdo_stream_buffer_alloc`, and VDO will round robin the buffers and place
//! frame data in them.
//!

use glib::translate::from_glib_full;
use glib_sys::{gboolean, gpointer, GError, GTRUE};
use gobject_sys::{g_object_unref, GObject};
use log::{debug, error, info};
use std::ffi::{CStr, CString};
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::mem;
use std::sync::{mpsc, Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::{ptr, thread};
use vdo_sys::*;
pub use vdo_sys::{VdoBufferStrategy, VdoFormat};

macro_rules! try_func {
    ($func:ident $(,)?) => {{
        let mut error: *mut GError = ptr::null_mut();
        let success = $func(&mut error);
        if error.is_null() {
            (success, None)
        } else {
            (success, Some(Error::VDOError(VDOError::from_gerror(error))))
        }
    }};
    ($func:ident, $($arg:expr),+ $(,)?) => {{
        let mut error: *mut GError = ptr::null_mut();
        let success = $func($( $arg ),+, &mut error);
        if error.is_null() {
            (success, None)
        } else {
            (success, Some(Error::VDOError(VDOError::from_gerror(error))))
        }

    }}
}

#[derive(Default)]
pub struct VDOError {
    code: i32,
    message: String,
}

impl Display for VDOError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "VDOError {{ code: {}, phrase: {}, message: {} }}",
            self.code,
            self.get_code_message(),
            self.message
        )
    }
}

impl VDOError {
    fn get_code_message(&self) -> &str {
        let code_u = u32::try_from(self.code).unwrap_or(0);
        if code_u == VDO_ERROR_NOT_FOUND as u32 {
            "VDO_ERROR_NOT_FOUND"
        } else if code_u == VDO_ERROR_EXISTS as u32 {
            "VDO_ERROR_EXISTS"
        } else if code_u == VDO_ERROR_INVALID_ARGUMENT as u32 {
            "VDO_ERROR_INVALID_ARGUMENT"
        } else if code_u == VDO_ERROR_PERMISSION_DENIED as u32 {
            "VDO_ERROR_PERMISSION_DENIED"
        } else if code_u == VDO_ERROR_NOT_SUPPORTED as u32 {
            "VDO_ERROR_NOT_SUPPORTED"
        } else if code_u == VDO_ERROR_CLOSED as u32 {
            "VDO_ERROR_CLOSED"
        } else if code_u == VDO_ERROR_BUSY as u32 {
            "VDO_ERROR_BUSY"
        } else if code_u == VDO_ERROR_IO as u32 {
            "VDO_ERROR_IO"
        } else if code_u == VDO_ERROR_HAL as u32 {
            "VDO_ERROR_HAL"
        } else if code_u == VDO_ERROR_DBUS as u32 {
            "VDO_ERROR_DBUS"
        } else if code_u == VDO_ERROR_OOM as u32 {
            "VDO_ERROR_OOM"
        } else if code_u == VDO_ERROR_IDLE as u32 {
            "VDO_ERROR_IDLE"
        } else if code_u == VDO_ERROR_NO_DATA as u32 {
            "VDO_ERROR_NO_DATA"
        } else if code_u == VDO_ERROR_NO_BUFFER_SPACE as u32 {
            "VDO_ERROR_NO_BUFFER_SPACE"
        } else if code_u == VDO_ERROR_BUFFER_FAILURE as u32 {
            "VDO_ERROR_BUFFER_FAILURE"
        } else if code_u == VDO_ERROR_INTERFACE_DOWN as u32 {
            "VDO_ERROR_INTERFACE_DOWN"
        } else if code_u == VDO_ERROR_FAILED as u32 {
            "VDO_ERROR_FAILED"
        } else if code_u == VDO_ERROR_FATAL as u32 {
            "VDO_ERROR_FATAL"
        } else if code_u == VDO_ERROR_NOT_CONTROLLED as u32 {
            "VDO_ERROR_NOT_CONTROLLED"
        } else if code_u == VDO_ERROR_NO_EVENT as u32 {
            "VDO_ERROR_NO_EVENT"
        } else {
            "VDO_ERROR_FAILED"
        }
    }

    fn from_gerror(gerror: *mut GError) -> Self {
        if !gerror.is_null() {
            let g_error = unsafe { *gerror };
            if !g_error.message.is_null() {
                let msg = unsafe { CStr::from_ptr(g_error.message) };
                VDOError {
                    code: g_error.code,
                    message: String::from(msg.to_str().unwrap_or("Invalid message")),
                }
            } else {
                VDOError {
                    code: g_error.code,
                    message: String::from("Invalid message"),
                }
            }
        } else {
            VDOError::default()
        }
    }
}

impl std::error::Error for VDOError {}

impl Debug for VDOError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

type Result<T> = std::result::Result<T, Error>;
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    VDOError(#[from] VDOError),
    #[error("libvdo returned an unexpected null pointer")]
    NullPointer,
    #[error("could not allocate memory for CString")]
    CStringAllocation,
    #[error("missing error data from libvdo")]
    MissingVDOError,
    #[error("poisoned pointer to stream")]
    PoisonedStream,
    #[error("no buffers are allocated for the stream")]
    NoBuffersAllocated,
}

pub struct Map {
    raw: *mut VdoMap,
}

impl Map {
    /// Create a new larodMap object
    pub fn new() -> Result<Self> {
        let map = unsafe { vdo_map_new() };
        if !map.is_null() {
            Ok(Self { raw: map })
        } else {
            Err(Error::NullPointer)
        }
    }

    pub fn set_u32(&self, key: &str, value: u32) -> Result<()> {
        let Ok(key_cstr) = CString::new(key) else {
            return Err(Error::CStringAllocation);
        };
        unsafe {
            vdo_map_set_uint32(self.raw, key_cstr.as_ptr(), value);
        }
        Ok(())
    }

    pub fn dump(&self) {
        unsafe {
            vdo_map_dump(self.raw);
        }
    }
}

impl Drop for Map {
    // using g_object_unref is sourced from the vdo-larod examples
    // https://github.com/AxisCommunications/acap-native-sdk-examples/blob/36800ed4c28dd96a2b659db3cb2c8a937c61d6d0/vdo-larod/app/imgprovider.c#L355
    fn drop(&mut self) {
        unsafe { g_object_unref(self.raw as *mut GObject) }
    }
}

pub struct StreamBuilder {
    format: VdoFormat,
    buffer_access: u32,
    buffer_count: u32,
    buffer_strategy: VdoBufferStrategy,
    input: u32,
    channel: u32,
    width: u32,
    height: u32,
    framerate: u32,
    compression: u32,
    rotation: u32,
    horizontal_flip: bool,
    monochrome: bool,
    dynamic_gop: bool,
    dynamic_bitrate: bool,
    dynamic_framerate: bool,
    dynamic_compression: bool,
    qp_i: u32,
    qp_p: u32,
    bitrate: u32,
    rc_mode: VdoRateControlMode,
    rc_prio: VdoRateControlPriority,
    gop_length: u32,
    overlays: Option<String>,
}

impl Default for StreamBuilder {
    /// VdoBufferStrategy::VDO_BUFFER_STRATEGY_EXPLICIT only works for VdoFormat::VDO_FORMAT_YUV and RGB
    /// VdoBufferStrategy::VDO_BUFFER_STRATEGY_INFINITE works for all VdoFormat's.
    fn default() -> Self {
        StreamBuilder {
            format: VdoFormat::VDO_FORMAT_H264,
            buffer_access: 0,
            buffer_count: 0,
            buffer_strategy: VdoBufferStrategy::VDO_BUFFER_STRATEGY_INFINITE,
            input: 0,
            channel: 0,
            width: 0,
            height: 0,
            framerate: 0,
            compression: 0,
            rotation: 0,
            horizontal_flip: false,
            monochrome: false,
            dynamic_gop: false,
            dynamic_bitrate: false,
            dynamic_framerate: false,
            dynamic_compression: false,
            qp_i: 0,
            qp_p: 0,
            bitrate: 0,
            rc_mode: VdoRateControlMode::VDO_RATE_CONTROL_MODE_NONE,
            rc_prio: VdoRateControlPriority::VDO_RATE_CONTROL_PRIORITY_NONE,
            gop_length: 0,
            overlays: None,
        }
    }
}

impl StreamBuilder {
    pub fn new() -> Self {
        StreamBuilder::default()
    }

    /// Set the number of buffers to use for the stream.
    /// For VdoFormat::VDO_FORMAT_YUV and RGB, the default number of buffers is 3.
    /// For VdoFormat::VDO_FORMAT_JPEG and h26x this number is ignored.
    pub fn buffers(mut self, num_buffers: u32) -> Self {
        self.buffer_count = num_buffers;
        self
    }

    pub fn channel(mut self, chan: u32) -> Self {
        self.channel = chan;
        self
    }
    pub fn format(mut self, format: VdoFormat) -> Self {
        self.format = format;
        self
    }

    /// Set the resolution for the stream.
    pub fn resolution(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn framerate(mut self, framerate: u32) -> Self {
        self.framerate = framerate;
        self
    }

    pub fn build(self) -> Result<Stream> {
        let map = Map::new()?;
        map.set_u32("channel", self.channel)?;
        map.set_u32("format", self.format as u32)?;
        map.set_u32("width", self.width)?;
        map.set_u32("height", self.height)?;
        map.set_u32("framerate", self.framerate)?;
        map.set_u32("buffer.count", self.buffer_count)?;
        map.set_u32("buffer.strategy", self.buffer_strategy as u32)?;
        let (stream_raw, maybe_error) = unsafe { try_func!(vdo_stream_new, map.raw, None) };
        if !stream_raw.is_null() {
            debug_assert!(
                maybe_error.is_none(),
                "vdo_stream_new returned an stream pointer AND returned an error!"
            );
            Ok(Stream {
                raw: StreamWrapper(stream_raw),
                buffers: Vec::new(),
            })
        } else {
            Err(maybe_error.unwrap_or(Error::MissingVDOError))
        }
    }
}

pub struct Frame<'a> {
    raw: *mut VdoFrame,
    phantom: PhantomData<&'a Buffer>,
}

impl<'a> Frame<'a> {
    pub fn size(&self) -> usize {
        unsafe { vdo_frame_get_size(self.raw) }
    }

    pub fn get_type(&self) -> VdoFrameType {
        unsafe { vdo_frame_get_frame_type(self.raw) }
    }
}

pub struct Buffer {
    raw: *mut VdoBuffer,
}

pub struct StreamBuffer<'a> {
    buffer: Buffer,
    stream: &'a Stream,
}

impl<'a> StreamBuffer<'a> {
    pub fn capacity(&self) -> usize {
        unsafe { vdo_buffer_get_capacity(self.buffer.raw) }
    }
    pub fn as_slice(&self) -> Result<&[u8]> {
        let buffer_data = unsafe { vdo_buffer_get_data(self.buffer.raw) };
        if !buffer_data.is_null() {
            let slice =
                unsafe { std::slice::from_raw_parts(buffer_data as *mut u8, self.capacity()) } as _;
            Ok(slice)
        } else {
            Err(Error::NullPointer)
        }
    }

    pub fn as_mut_slice(&self) -> Result<&mut [u8]> {
        let buffer_data = unsafe { vdo_buffer_get_data(self.buffer.raw) };
        if !buffer_data.is_null() {
            let slice =
                unsafe { std::slice::from_raw_parts_mut(buffer_data as *mut u8, self.capacity()) }
                    as _;
            Ok(slice)
        } else {
            Err(Error::NullPointer)
        }
    }

    pub fn frame(&self) -> Result<Frame> {
        let frame = unsafe { vdo_buffer_get_frame(self.buffer.raw) };
        Ok(Frame {
            raw: frame,
            phantom: PhantomData,
        })
    }
}

impl<'a> Drop for StreamBuffer<'a> {
    fn drop(&mut self) {
        let (success, maybe_error) = unsafe {
            try_func!(
                vdo_stream_buffer_unref,
                self.stream.raw.0,
                &mut self.buffer.raw
            )
        };
    }
}

// unsafe impl Send for Buffer {}

unsafe impl Send for StreamWrapper {}

struct StreamWrapper(*mut VdoStream);
pub struct Stream {
    raw: StreamWrapper,
    buffers: Vec<*mut VdoBuffer>,
}

pub struct StreamIterator<'a> {
    stream: &'a Stream,
}

impl<'a> Iterator for StreamIterator<'a> {
    type Item = StreamBuffer<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let (buffer_ptr, maybe_error) =
            unsafe { try_func!(vdo_stream_get_buffer, self.stream.raw.0) };
        if !buffer_ptr.is_null() {
            debug_assert!(
                maybe_error.is_none(),
                "vdo_stream_get_buffer returned an stream pointer AND returned an error!"
            );
            debug!("fetched buffer from vdo stream");
            Some(StreamBuffer {
                buffer: Buffer { raw: buffer_ptr },
                stream: self.stream,
            })
        } else {
            None
        }
    }
}

pub struct RunningStream<'a> {
    stream: &'a mut Stream,
}

impl<'a> RunningStream<'a> {
    pub fn iter(&mut self) -> StreamIterator {
        StreamIterator {
            stream: self.stream,
        }
    }
    pub fn stop(&mut self) -> Result<()> {
        unsafe { vdo_stream_stop(self.stream.raw.0) };
        Ok(())
    }
}

impl<'a> IntoIterator for &'a RunningStream<'a> {
    type Item = StreamBuffer<'a>;
    type IntoIter = StreamIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        StreamIterator {
            stream: self.stream,
        }
    }
}

impl Stream {
    pub fn builder() -> StreamBuilder {
        StreamBuilder::new()
    }

    pub fn new() -> Result<Self> {
        StreamBuilder::new().build()
    }

    pub fn info(&self) -> Result<Map> {
        let (map_raw, maybe_error) = unsafe { try_func!(vdo_stream_get_info, self.raw.0) };
        if !map_raw.is_null() {
            debug_assert!(
                maybe_error.is_none(),
                "vdo_stream_get_info returned a pointer AND returned an error!"
            );
            Ok(Map { raw: map_raw })
        } else {
            Err(maybe_error.unwrap_or(Error::MissingVDOError))
        }
    }

    pub fn settings(&self) -> Result<Map> {
        let (map_raw, maybe_error) = unsafe { try_func!(vdo_stream_get_settings, self.raw.0) };
        if !map_raw.is_null() {
            debug_assert!(
                maybe_error.is_none(),
                "vdo_stream_get_settings returned a pointer AND returned an error!"
            );
            Ok(Map { raw: map_raw })
        } else {
            Err(maybe_error.unwrap_or(Error::MissingVDOError))
        }
    }

    /// Request the Larod service to start fetching frames and passing back buffers.
    pub fn start(&mut self) -> Result<RunningStream> {
        let (success_start, maybe_error) = unsafe { try_func!(vdo_stream_start, self.raw.0) };
        if success_start == GTRUE {
            debug_assert!(
                maybe_error.is_none(),
                "vdo_stream_new returned an stream pointer AND returned an error!"
            );
            Ok(RunningStream { stream: self })
        } else {
            Err(maybe_error.unwrap_or(Error::MissingVDOError))
        }
    }

    // Do we want to spawn a fetcher thread?
    // pub fn start_with_channel(&mut self) -> Result<()> {
    //     let Ok(raw_stream_ptr) = self.raw.lock() else {
    //         return Err(Error::PoisonedStream);
    //     };
    //     for buffer in self.buffers.iter() {
    //         let (success_enqueue, maybe_error) =
    //             unsafe { try_func!(vdo_stream_buffer_enqueue, raw_stream_ptr.0, buffer.raw) };
    //         if success_enqueue == GTRUE {
    //             debug!("enqueued buffer to stream");
    //             debug_assert!(
    //                 maybe_error.is_none(),
    //                 "vdo_stream_buffer_enqueue indicated success AND returned an error!"
    //             );
    //         } else {
    //             return Err(maybe_error.unwrap_or(Error::MissingVDOError));
    //         }
    //     }
    //     let (success_start, maybe_error) = unsafe { try_func!(vdo_stream_start, raw_stream_ptr.0) };
    //     drop(raw_stream_ptr);
    //     if success_start == GTRUE {
    //         debug_assert!(
    //             maybe_error.is_none(),
    //             "vdo_stream_new returned an stream pointer AND returned an error!"
    //         );
    //     } else {
    //         return Err(maybe_error.unwrap_or(Error::MissingVDOError));
    //     }
    //     let (sender, receiver) = mpsc::channel();
    //     let stream_c = self.raw.clone();
    //     self.rx_channel = Some(receiver);
    //     self.fetcher_thread = Some(thread::spawn(move || {
    //         debug!("starting frame fetcher thread");
    //         let Ok(raw_stream_ptr) = stream_c.lock() else {
    //             return Err(StreamError::PoisonedStream);
    //         };
    //         let (buffer_ptr, maybe_error) =
    //             unsafe { try_func!(vdo_stream_get_buffer, raw_stream_ptr.0) };
    //         if !buffer_ptr.is_null() {
    //             debug_assert!(
    //                 maybe_error.is_none(),
    //                 "vdo_stream_get_buffer returned an stream pointer AND returned an error!"
    //             );
    //             debug!("fetched buffer from vdo stream");
    //             sender.send(Buffer { raw: buffer_ptr });
    //         } else {
    //             error!("error while fetching buffer: {}", maybe_error.unwrap());
    //         }
    //         Ok(())
    //     }));
    //     Ok(())
    // }
}

impl Drop for Stream {
    fn drop(&mut self) {
        unsafe {
            vdo_stream_stop(self.raw.0);
        }
        for mut buffer in mem::take(&mut self.buffers).into_iter() {
            let (success, maybe_error) =
                unsafe { try_func!(vdo_stream_buffer_unref, self.raw.0, &mut buffer) };
        }
    }
}

#[cfg(all(test, target_arch = "aarch64", feature = "device-tests"))]
mod tests {
    use super::*;
    use anyhow::Context;

    #[test]
    fn stream_starts_without_explicit_buffers() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init();
        let mut stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_PLANAR_RGB)
            .resolution(1920, 1080)
            .build()
            .context("Unable to create stream")?;
        let mut r = stream.start().context("Unable to start stream")?;
        r.stop()?;
        Ok(())
    }

    #[test]
    fn stream_starts_with_explicit_buffers() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init();
        let mut stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_PLANAR_RGB)
            .resolution(1920, 1080)
            .buffers(5)
            .build()
            .context("Unable to create stream")?;
        let mut r = stream.start().context("Unable to start stream")?;
        for _ in 0..10 {
            let buff = r.iter().next().context("failed to fetch frame")?;
            let size = buff
                .frame()
                .context("error fetching frame for buffer")?
                .size();
            info!("frame size: {}", size);
            assert!(size > 0);
        }
        r.stop()?;
        Ok(())
    }

    #[test]
    fn stream_starts_with_rgb() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init();
        let mut stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_PLANAR_RGB)
            .resolution(1920, 1080)
            .build()
            .context("Unable to create stream")?;
        let mut r = stream.start().context("starting stream returned error")?;
        for _ in 0..10 {
            let buff = r.iter().next().context("failed to fetch frame")?;
            let size = buff
                .frame()
                .context("error fetching frame for buffer")?
                .size();
            info!("frame size: {}", size);
            assert!(size > 0);
        }
        r.stop().context("Unable to stop stream")?;
        Ok(())
    }

    #[test]
    fn stream_starts_with_jpeg() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init();
        let mut stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_JPEG)
            .resolution(1920, 1080)
            .build()
            .context("Unable to create stream")?;
        let mut r = stream.start().context("starting stream returned error")?;
        for _ in 0..10 {
            let buff = r.iter().next().context("failed to fetch frame")?;
            let size = buff
                .frame()
                .context("error fetching frame for buffer")?
                .size();
            info!("frame size: {}", size);
            assert!(size > 0);
        }
        r.stop().context("Unable to stop stream")?;
        Ok(())
    }

    #[test]
    fn stream_starts_with_yuv() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init();
        let mut stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_YUV)
            .resolution(1920, 1080)
            .build()
            .context("Unable to create stream")?;
        let mut r = stream.start().context("starting stream returned error")?;
        for _ in 0..10 {
            let buff = r.iter().next().context("failed to fetch frame")?;
            let size = buff
                .frame()
                .context("error fetching frame for buffer")?
                .size();
            info!("frame size: {}", size);
            assert!(size > 0);
        }
        r.stop().context("Unable to stop stream")?;
        Ok(())
    }

    #[test]
    fn stream_starts_with_h264() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init();
        let mut stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_H264)
            .resolution(1920, 1080)
            .build()
            .context("Unable to create stream")?;
        let mut r = stream.start().context("starting stream returned error")?;
        for _ in 0..10 {
            let buff = r.iter().next().context("failed to fetch frame")?;
            let size = buff
                .frame()
                .context("error fetching frame for buffer")?
                .size();
            info!("frame size: {}", size);
            assert!(size > 0);
        }
        r.stop().context("Unable to stop stream")?;
        Ok(())
    }

    #[test]
    fn stream_starts_with_h265() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init();
        let mut stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_H265)
            .resolution(1920, 1080)
            .build()
            .context("Unable to create stream")?;
        let mut r = stream.start().context("starting stream returned error")?;
        for _ in 0..10 {
            let buff = r.iter().next().context("failed to fetch frame")?;
            let size = buff
                .frame()
                .context("error fetching frame for buffer")?
                .size();
            info!("frame size: {}", size);
            assert!(size > 0);
        }
        r.stop().context("Unable to stop stream")?;
        Ok(())
    }

    #[test]
    fn stream_fetches_frames_infinitely() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init();
        let mut stream = Stream::builder()
            .channel(0)
            .format(VdoFormat::VDO_FORMAT_PLANAR_RGB)
            .resolution(1920, 1080)
            .buffers(5)
            .build()
            .context("Unable to create stream")?;
        let mut r = stream.start().context("starting stream returned error")?;

        for _ in 0..10 {
            let buff = r.iter().next().context("failed to fetch frame")?;
            let size = buff
                .frame()
                .context("error fetching frame for buffer")?
                .size();
            info!("frame size: {}", size);
            assert!(size > 0);
        }

        r.stop().context("Unable to stop stream")?;
        Ok(())
    }
}
