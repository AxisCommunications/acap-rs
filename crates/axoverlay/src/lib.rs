//! Bindings for the [Overlay API](https://developer.axis.com/acap/api/src/api/axoverlay/html/index.html).

use std::{
    ffi::{c_float, c_int},
    mem::MaybeUninit,
    ptr,
    sync::Mutex,
};

use axoverlay_sys::{
    axoverlay_anchor_point_AXOVERLAY_ANCHOR_CENTER,
    axoverlay_anchor_point_AXOVERLAY_ANCHOR_TOP_LEFT,
    axoverlay_backend_type_AXOVERLAY_CAIRO_IMAGE_BACKEND, axoverlay_cleanup,
    axoverlay_colorspace_AXOVERLAY_COLORSPACE_1BIT_PALETTE,
    axoverlay_colorspace_AXOVERLAY_COLORSPACE_4BIT_PALETTE,
    axoverlay_colorspace_AXOVERLAY_COLORSPACE_ARGB32, axoverlay_create_overlay,
    axoverlay_destroy_overlay, axoverlay_get_max_resolution_height,
    axoverlay_get_max_resolution_width, axoverlay_init, axoverlay_init_axoverlay_settings,
    axoverlay_init_overlay_data, axoverlay_is_backend_supported, axoverlay_overlay_data,
    axoverlay_palette_color, axoverlay_position_type,
    axoverlay_position_type_AXOVERLAY_BOTTOM_LEFT, axoverlay_position_type_AXOVERLAY_BOTTOM_RIGHT,
    axoverlay_position_type_AXOVERLAY_CUSTOM_NORMALIZED,
    axoverlay_position_type_AXOVERLAY_CUSTOM_SOURCE, axoverlay_position_type_AXOVERLAY_TOP_LEFT,
    axoverlay_position_type_AXOVERLAY_TOP_RIGHT, axoverlay_redraw, axoverlay_set_palette_color,
    axoverlay_settings, axoverlay_stream_data, axoverlay_stream_type,
    axoverlay_stream_type_AXOVERLAY_STREAM_AV1, axoverlay_stream_type_AXOVERLAY_STREAM_H264,
    axoverlay_stream_type_AXOVERLAY_STREAM_H265, axoverlay_stream_type_AXOVERLAY_STREAM_JPEG,
    axoverlay_stream_type_AXOVERLAY_STREAM_RGB, axoverlay_stream_type_AXOVERLAY_STREAM_VOUT,
    axoverlay_stream_type_AXOVERLAY_STREAM_YCBCR,
};
pub use glib::Error;
use glib::{translate::from_glib_full, MainLoop};
use glib_sys::{gpointer, GError, GFALSE, GTRUE};
use log::error;

static ADJUSTMENT_CALLBACK: Mutex<Option<AdjustmentFunction>> = Mutex::new(None);
static RENDER_CALLBACK: Mutex<Option<RenderFunction>> = Mutex::new(None);

type Result<T> = core::result::Result<T, Error>;

unsafe fn try_into_unit(_: (), error: *mut GError) -> Result<()> {
    if error.is_null() {
        Ok(())
    } else {
        Err(from_glib_full(error))
    }
}

macro_rules! try_func {
    ($func:ident, $($arg:expr),* $(,)?) => {{
        let mut error: *mut GError = ptr::null_mut();
        let success = $func($( $arg, )* &mut error);
        try_into_unit(success, error)
    }}
}

macro_rules! try_func_retval {
    ($func:ident, $($arg:expr),+ $(,)?) => {{
        let mut error: *mut GError = ptr::null_mut();
        let retval = $func($( $arg ),+, &mut error);
        if error.is_null() {
            Ok(retval)
        } else {
            Err(from_glib_full(error))
        }
    }}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnchorPoint {
    TopLeft,
    Center,
}

impl AnchorPoint {
    fn as_int(self) -> u32 {
        match self {
            AnchorPoint::TopLeft => axoverlay_anchor_point_AXOVERLAY_ANCHOR_TOP_LEFT,
            AnchorPoint::Center => axoverlay_anchor_point_AXOVERLAY_ANCHOR_CENTER,
        }
    }
}

// TODO: Implement support for other backends in `render_callback_trampoline`
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Backend {
    CairoImage,
    // OpenGLES,
    // Open,
}

impl Backend {
    fn as_int(self) -> u32 {
        match self {
            Backend::CairoImage => axoverlay_backend_type_AXOVERLAY_CAIRO_IMAGE_BACKEND,
            // Backend::OpenGLES => axoverlay_backend_type_AXOVERLAY_OPENGLES_BACKEND,
            // Backend::Open => axoverlay_backend_type_AXOVERLAY_OPEN_BACKEND,
        }
    }

    pub fn is_supported(&self) -> bool {
        // TODO: Safety
        unsafe { axoverlay_is_backend_supported(self.as_int()) == GTRUE }
    }
}

pub struct Camera(i32);

impl Camera {
    pub fn max_height(&self) -> Result<i32> {
        // TODO: Safety
        unsafe { try_func_retval!(axoverlay_get_max_resolution_height, self.0) }
    }

    pub fn max_width(&self) -> Result<i32> {
        // TODO: Safety
        unsafe { try_func_retval!(axoverlay_get_max_resolution_width, self.0) }
    }
}

impl std::fmt::Display for Camera {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub struct Color(axoverlay_palette_color);

impl Color {
    pub fn set_palette(&mut self, index: usize) -> Result<()> {
        // TODO: Safety
        unsafe { try_func!(axoverlay_set_palette_color, index as c_int, &mut self.0) }
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    ARGB32,
    FourBitPalette,
    OneBitPalette,
}

impl ColorSpace {
    fn as_int(self) -> u32 {
        match self {
            ColorSpace::ARGB32 => axoverlay_colorspace_AXOVERLAY_COLORSPACE_ARGB32,
            ColorSpace::FourBitPalette => axoverlay_colorspace_AXOVERLAY_COLORSPACE_4BIT_PALETTE,
            ColorSpace::OneBitPalette => axoverlay_colorspace_AXOVERLAY_COLORSPACE_1BIT_PALETTE,
        }
    }
}

pub struct OverlayBuilder(axoverlay_overlay_data);

impl OverlayBuilder {
    pub fn height(&mut self, height: i32) -> &mut Self {
        self.0.height = height;
        self
    }

    pub fn width(&mut self, width: i32) -> &mut Self {
        self.0.width = width;
        self
    }

    pub fn x(&mut self, x: f32) -> &mut Self {
        self.0.x = x;
        self
    }

    pub fn y(&mut self, y: f32) -> &mut Self {
        self.0.y = y;
        self
    }

    pub fn anchor_point(&mut self, anchor_point: AnchorPoint) -> &mut Self {
        self.0.anchor_point = anchor_point.as_int();
        self
    }

    pub fn colorspace(&mut self, colorspace: ColorSpace) -> &mut Self {
        self.0.colorspace = colorspace.as_int();
        self
    }

    pub fn position_type(&mut self, pos_type: PosType) -> &mut Self {
        self.0.postype = pos_type.as_int();
        self
    }

    pub fn scale_to_stream(&mut self, scale_to_stream: bool) -> &mut Self {
        self.0.scale_to_stream = match scale_to_stream {
            true => GTRUE,
            false => GFALSE,
        };
        self
    }

    pub fn create_overlay(&mut self) -> Result<Overlay> {
        // TODO: Implement user data
        // TODO: Safety
        match unsafe { try_func_retval!(axoverlay_create_overlay, &mut self.0, ptr::null_mut()) } {
            Ok(id) => Ok(Overlay(id)),
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Overlay(c_int);

impl Overlay {
    pub fn id(&self) -> i32 {
        self.0
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        // TODO: Safety
        match unsafe { try_func!(axoverlay_destroy_overlay, self.0) } {
            Ok(()) => {}
            Err(e) => {
                error!("Failed to destroy overlay: {}", e);
            }
        }
    }
}

pub struct OverlayInfo {
    pub x: f32,
    pub y: f32,
    pub width: i32,
    pub height: i32,
}

type AdjustmentFunction = fn(
    id: i32,
    stream: &StreamData,
    position_type: &PosType,
    overlay_x: &mut c_float,
    overlay_y: &mut c_float,
    overlay_width: &mut c_int,
    overlay_height: &mut c_int,
);

extern "C" fn adjustment_callback_trampoline(
    id: c_int,
    stream: *mut axoverlay_stream_data,
    position_type: *mut axoverlay_position_type,
    overlay_x: *mut c_float,
    overlay_y: *mut c_float,
    overlay_width: *mut c_int,
    overlay_height: *mut c_int,
    _user_data: gpointer,
) {
    if let Some(adjustment_function) = ADJUSTMENT_CALLBACK.lock().unwrap().as_ref() {
        let stream_data = StreamData(stream);
        let position_type = unsafe { PosType::from_int(*position_type) };
        let overlay_x = unsafe { overlay_x.as_mut().unwrap() };
        let overlay_y = unsafe { overlay_y.as_mut().unwrap() };
        let overlay_width = unsafe { overlay_width.as_mut().unwrap() };
        let overlay_height = unsafe { overlay_height.as_mut().unwrap() };

        adjustment_function(
            id,
            &stream_data,
            &position_type,
            overlay_x,
            overlay_y,
            overlay_width,
            overlay_height,
        );
    }
}

type RenderFunction = fn(
    rendering_context: &cairo::Context,
    id: i32,
    stream: &StreamData,
    position_type: PosType,
    info: OverlayInfo,
);

extern "C" fn render_callback_trampoline(
    rendering_context: gpointer,
    id: c_int,
    stream: *mut axoverlay_stream_data,
    position_type: axoverlay_position_type,
    overlay_x: c_float,
    overlay_y: c_float,
    overlay_width: c_int,
    overlay_height: c_int,
    _user_data: gpointer,
) {
    if let Some(render_callback) = RENDER_CALLBACK.lock().unwrap().as_ref() {
        let stream_data = StreamData(stream);
        let position_type = PosType::from_int(position_type);
        let rendering_context = unsafe {
            cairo::Context::from_raw_borrow(rendering_context as *mut cairo::ffi::cairo_t)
        };
        render_callback(
            &rendering_context,
            id,
            &stream_data,
            position_type,
            OverlayInfo {
                x: overlay_x,
                y: overlay_y,
                width: overlay_width,
                height: overlay_height,
            },
        );
    }
}

#[derive(Debug)]
pub struct Settings(axoverlay_settings);

impl Settings {
    pub fn backend(&mut self, backend: Backend) -> &mut Self {
        self.0.backend = backend.as_int();
        self
    }

    pub fn adjustment_callback(&mut self, f: AdjustmentFunction) -> &mut Self {
        if ADJUSTMENT_CALLBACK.lock().unwrap().replace(f).is_some() {
            panic!("Adjustment function already set");
        }
        self.0.adjustment_callback = Some(adjustment_callback_trampoline);
        self
    }

    pub fn render_callback(&mut self, f: RenderFunction) -> &mut Self {
        if RENDER_CALLBACK.lock().unwrap().replace(f).is_some() {
            panic!("Render function already set");
        }
        self.0.render_callback = Some(render_callback_trampoline);
        self
    }

    pub fn init(&mut self, _main_loop: &MainLoop) -> Result<Api> {
        // TODO: Safety
        match unsafe { try_func!(axoverlay_init, &mut self.0) } {
            Ok(()) => Ok(Api {}),
            Err(e) => Err(e),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        let mut inner = MaybeUninit::<axoverlay_settings>::uninit();
        // TODO: Safety
        unsafe {
            axoverlay_init_axoverlay_settings(inner.as_mut_ptr());
            Self(inner.assume_init())
        }
    }
}

pub struct Api {}

impl Api {
    pub fn camera(&self, id: i32) -> Camera {
        Camera(id)
    }

    pub fn color(&self, red: u8, green: u8, blue: u8, alpha: u8, pixelate: bool) -> Color {
        Color(axoverlay_palette_color {
            red,
            green,
            blue,
            alpha,
            pixelate: match pixelate {
                true => GTRUE,
                false => GFALSE,
            },
        })
    }

    pub fn overlay_builder(&self) -> OverlayBuilder {
        let mut inner = MaybeUninit::<axoverlay_overlay_data>::uninit();
        // TODO: Safety
        unsafe {
            axoverlay_init_overlay_data(inner.as_mut_ptr());
            OverlayBuilder(inner.assume_init())
        }
    }
}

/// Must not be called before `API` has been initialized
pub fn redraw() -> Result<()> {
    // TODO: Safety
    unsafe { try_func!(axoverlay_redraw,) }
}

impl Drop for Api {
    fn drop(&mut self) {
        ADJUSTMENT_CALLBACK.lock().unwrap().take();
        // TODO: Safety
        unsafe { axoverlay_cleanup() }
    }
}

pub enum PosType {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    CustomNormalized,
    CustomSource,
}

impl PosType {
    fn from_int(value: axoverlay_position_type) -> Self {
        match value {
            i if i == axoverlay_position_type_AXOVERLAY_TOP_LEFT => Self::TopLeft,
            i if i == axoverlay_position_type_AXOVERLAY_TOP_RIGHT => Self::TopRight,
            i if i == axoverlay_position_type_AXOVERLAY_BOTTOM_LEFT => Self::BottomLeft,
            i if i == axoverlay_position_type_AXOVERLAY_BOTTOM_RIGHT => Self::BottomRight,
            i if i == axoverlay_position_type_AXOVERLAY_CUSTOM_NORMALIZED => Self::CustomNormalized,
            i if i == axoverlay_position_type_AXOVERLAY_CUSTOM_SOURCE => Self::CustomSource,
            _ => unreachable!(),
        }
    }

    fn as_int(&self) -> axoverlay_position_type {
        match self {
            Self::TopLeft => axoverlay_position_type_AXOVERLAY_TOP_LEFT,
            Self::TopRight => axoverlay_position_type_AXOVERLAY_TOP_RIGHT,
            Self::BottomLeft => axoverlay_position_type_AXOVERLAY_BOTTOM_LEFT,
            Self::BottomRight => axoverlay_position_type_AXOVERLAY_BOTTOM_RIGHT,
            Self::CustomNormalized => axoverlay_position_type_AXOVERLAY_CUSTOM_NORMALIZED,
            Self::CustomSource => axoverlay_position_type_AXOVERLAY_CUSTOM_SOURCE,
        }
    }
}

pub struct StreamData(*mut axoverlay_stream_data);

impl StreamData {
    pub fn id(&self) -> i32 {
        // TODO: Safety
        unsafe { (*self.0).id }
    }

    pub fn camera(&self) -> Camera {
        // TODO: Safety
        Camera(unsafe { (*self.0).camera })
    }

    pub fn width(&self) -> i32 {
        // TODO: Safety
        unsafe { (*self.0).width }
    }

    pub fn height(&self) -> i32 {
        // TODO: Safety
        unsafe { (*self.0).height }
    }

    pub fn rotation(&self) -> i32 {
        // TODO: Safety
        unsafe { (*self.0).rotation }
    }

    pub fn is_mirrored(&self) -> bool {
        // TODO: Safety
        unsafe { (*self.0).is_mirrored == GTRUE }
    }

    pub fn type_(&self) -> StreamType {
        // TODO: Safety
        StreamType::from_int(unsafe { (*self.0).type_ })
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StreamType {
    JPEG,
    H264,
    H265,
    YCbCr,
    VOut,
    Other,
    RGB,
    AV1,
}

impl StreamType {
    fn from_int(value: axoverlay_stream_type) -> Self {
        match value {
            i if i == axoverlay_stream_type_AXOVERLAY_STREAM_JPEG => Self::JPEG,
            i if i == axoverlay_stream_type_AXOVERLAY_STREAM_H264 => Self::H264,
            i if i == axoverlay_stream_type_AXOVERLAY_STREAM_H265 => Self::H265,
            i if i == axoverlay_stream_type_AXOVERLAY_STREAM_YCBCR => Self::YCbCr,
            i if i == axoverlay_stream_type_AXOVERLAY_STREAM_VOUT => Self::VOut,
            i if i == axoverlay_stream_type_AXOVERLAY_STREAM_RGB => Self::RGB,
            i if i == axoverlay_stream_type_AXOVERLAY_STREAM_AV1 => Self::AV1,
            _ => Self::Other,
        }
    }
}
