#![allow(non_upper_case_globals)]
#![allow(clippy::redundant_closure_call)]

use std::ptr;

macro_rules! unsafe_check_success {
    ($success:expr) => {
        if !unsafe { $success } {
            core::result::Result::Err(std::io::Error::last_os_error())
        } else {
            core::result::Result::Ok(())
        }
    };
}

pub struct Bbox {
    ptr: *mut bbox_sys::bbox_t,
}

impl Drop for Bbox {
    fn drop(&mut self) {
        unsafe {
            if self.ptr.is_null() {
                return;
            }
            if !bbox_sys::bbox_destroy(self.ptr) {
                let error = std::io::Error::last_os_error();
                panic!(
                    "Could not destroy {}: {error:?}",
                    std::any::type_name::<Self>()
                );
            }
        }
    }
}
impl Bbox {
    pub fn try_view_new(view: u32) -> std::io::Result<Self> {
        unsafe {
            let ptr = bbox_sys::bbox_view_new(view);
            if ptr.is_null() {
                return Err(std::io::Error::last_os_error().into());
            }
            Ok(Self { ptr })
        }
    }

    // TODO: Consider changing this and/or the `Drop` implementation.
    /// Destroy the object.
    ///
    /// This would be done automatically when it is dropped, but at that point any errors will cause
    /// a panic or, possibly, an abort.
    pub fn try_destroy(mut self) -> std::io::Result<()> {
        let result = unsafe_check_success!(bbox_sys::bbox_destroy(self.ptr));
        self.ptr = ptr::null_mut();
        result
    }

    pub fn try_clear(&mut self) -> std::io::Result<()> {
        unsafe_check_success!(bbox_sys::bbox_clear(self.ptr))
    }
    pub fn try_commit(&mut self, when_us: i64) -> std::io::Result<()> {
        unsafe_check_success!(bbox_sys::bbox_commit(self.ptr, when_us))
    }

    pub fn try_style_corners(&mut self) -> std::io::Result<()> {
        unsafe_check_success!(bbox_sys::bbox_style_corners(self.ptr))
    }
    pub fn try_style_outline(&mut self) -> std::io::Result<()> {
        unsafe_check_success!(bbox_sys::bbox_style_outline(self.ptr))
    }
    pub fn try_thickness_thin(&mut self) -> std::io::Result<()> {
        unsafe_check_success!(bbox_sys::bbox_thickness_thin(self.ptr))
    }
    pub fn try_thickness_medium(&mut self) -> std::io::Result<()> {
        unsafe_check_success!(bbox_sys::bbox_thickness_medium(self.ptr))
    }
    pub fn try_thickness_thick(&mut self) -> std::io::Result<()> {
        unsafe_check_success!(bbox_sys::bbox_thickness_thick(self.ptr))
    }
    pub fn try_color(&mut self, color: Color) -> std::io::Result<()> {
        unsafe_check_success!(bbox_sys::bbox_color(self.ptr, color.raw))
    }

    pub fn try_draw_path(&mut self) -> std::io::Result<()> {
        unsafe_check_success!(bbox_sys::bbox_draw_path(self.ptr))
    }

    pub fn try_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) -> std::io::Result<()> {
        unsafe_check_success!(bbox_sys::bbox_line(self.ptr, x1, y1, x2, y2))
    }
    pub fn try_line_to(&mut self, x: f32, y: f32) -> std::io::Result<()> {
        unsafe_check_success!(bbox_sys::bbox_line_to(self.ptr, x, y))
    }
    pub fn try_move_to(&mut self, x: f32, y: f32) -> std::io::Result<()> {
        unsafe_check_success!(bbox_sys::bbox_move_to(self.ptr, x, y))
    }
    pub fn try_rectangle(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) -> std::io::Result<()> {
        unsafe_check_success!(bbox_sys::bbox_rectangle(self.ptr, x1, y1, x2, y2))
    }
    pub fn try_quad(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x3: f32,
        y3: f32,
        x4: f32,
        y4: f32,
    ) -> std::io::Result<()> {
        unsafe_check_success!(bbox_sys::bbox_quad(
            self.ptr, x1, y1, x2, y2, x3, y3, x4, y4
        ))
    }

    pub fn try_video_output(&mut self, enabled: bool) -> std::io::Result<()> {
        unsafe_check_success!(bbox_sys::bbox_video_output(self.ptr, enabled))
    }
}

#[derive(Copy, Clone)]
pub struct Color {
    raw: bbox_sys::bbox_color_t,
}

impl Color {
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        unsafe {
            let raw = bbox_sys::bbox_color_from_rgb(r, g, b);
            Self { raw }
        }
    }
    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        unsafe {
            let raw = bbox_sys::bbox_color_from_rgba(r, g, b, a);
            Self { raw }
        }
    }
}
