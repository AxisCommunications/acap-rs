use gobject_sys::{g_object_unref, GObject};
use std::ffi::CString;
use vdo_sys::*;

type Result<T> = std::result::Result<T, Error>;
pub enum Error {
    NullPointer,
    CStringAllocation,
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
    buffer_strategy: u32,
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
    fn default() -> Self {
        StreamBuilder {
            format: VdoFormat::VDO_FORMAT_H264,
            buffer_access: 0,
            buffer_count: 0,
            buffer_strategy: 0,
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

    pub fn build(self) -> Result<Stream> {
        let map = Map::new()?;
        map.set_u32("channel", self.channel)?;
        map.set_u32("format", self.format as u32)?;
        map.set_u32("width", self.width)?;
        map.set_u32("height", self.height)?;
        Ok(Stream {})
    }
}

pub struct Stream {}

impl Stream {
    fn builder() -> StreamBuilder {
        StreamBuilder::new()
    }
}
