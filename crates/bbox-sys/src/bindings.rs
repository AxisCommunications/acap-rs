/* automatically generated by rust-bindgen 0.69.4 */

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct __BindgenBitfieldUnit<Storage> {
    storage: Storage,
}
impl<Storage> __BindgenBitfieldUnit<Storage> {
    #[inline]
    pub const fn new(storage: Storage) -> Self {
        Self { storage }
    }
}
impl<Storage> __BindgenBitfieldUnit<Storage>
where
    Storage: AsRef<[u8]> + AsMut<[u8]>,
{
    #[inline]
    pub fn get_bit(&self, index: usize) -> bool {
        debug_assert!(index / 8 < self.storage.as_ref().len());
        let byte_index = index / 8;
        let byte = self.storage.as_ref()[byte_index];
        let bit_index = if cfg!(target_endian = "big") {
            7 - (index % 8)
        } else {
            index % 8
        };
        let mask = 1 << bit_index;
        byte & mask == mask
    }
    #[inline]
    pub fn set_bit(&mut self, index: usize, val: bool) {
        debug_assert!(index / 8 < self.storage.as_ref().len());
        let byte_index = index / 8;
        let byte = &mut self.storage.as_mut()[byte_index];
        let bit_index = if cfg!(target_endian = "big") {
            7 - (index % 8)
        } else {
            index % 8
        };
        let mask = 1 << bit_index;
        if val {
            *byte |= mask;
        } else {
            *byte &= !mask;
        }
    }
    #[inline]
    pub fn get(&self, bit_offset: usize, bit_width: u8) -> u64 {
        debug_assert!(bit_width <= 64);
        debug_assert!(bit_offset / 8 < self.storage.as_ref().len());
        debug_assert!((bit_offset + (bit_width as usize)) / 8 <= self.storage.as_ref().len());
        let mut val = 0;
        for i in 0..(bit_width as usize) {
            if self.get_bit(i + bit_offset) {
                let index = if cfg!(target_endian = "big") {
                    bit_width as usize - 1 - i
                } else {
                    i
                };
                val |= 1 << index;
            }
        }
        val
    }
    #[inline]
    pub fn set(&mut self, bit_offset: usize, bit_width: u8, val: u64) {
        debug_assert!(bit_width <= 64);
        debug_assert!(bit_offset / 8 < self.storage.as_ref().len());
        debug_assert!((bit_offset + (bit_width as usize)) / 8 <= self.storage.as_ref().len());
        for i in 0..(bit_width as usize) {
            let mask = 1 << i;
            let val_bit_is_set = val & mask == mask;
            let index = if cfg!(target_endian = "big") {
                bit_width as usize - 1 - i
            } else {
                i
            };
            self.set_bit(index + bit_offset, val_bit_is_set);
        }
    }
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct bbox_t {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct bbox_color_t {
    pub _bitfield_align_1: [u32; 0],
    pub _bitfield_1: __BindgenBitfieldUnit<[u8; 4usize]>,
}
impl bbox_color_t {
    #[inline]
    pub fn voldemort(&self) -> u32 {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(0usize, 32u8) as u32) }
    }
    #[inline]
    pub fn set_voldemort(&mut self, val: u32) {
        unsafe {
            let val: u32 = ::std::mem::transmute(val);
            self._bitfield_1.set(0usize, 32u8, val as u64)
        }
    }
    #[inline]
    pub fn new_bitfield_1(voldemort: u32) -> __BindgenBitfieldUnit<[u8; 4usize]> {
        let mut __bindgen_bitfield_unit: __BindgenBitfieldUnit<[u8; 4usize]> = Default::default();
        __bindgen_bitfield_unit.set(0usize, 32u8, {
            let voldemort: u32 = unsafe { ::std::mem::transmute(voldemort) };
            voldemort as u64
        });
        __bindgen_bitfield_unit
    }
}
pub type bbox_channel_t = u32;
extern "C" {
    pub fn bbox_new(n_channels: usize, ...) -> *mut bbox_t;
}
extern "C" {
    pub fn bbox_view_new(view: bbox_channel_t) -> *mut bbox_t;
}
extern "C" {
    pub fn bbox_destroy(self_: *mut bbox_t) -> bool;
}
extern "C" {
    pub fn bbox_video_output(self_: *mut bbox_t, enabled: bool) -> bool;
}
extern "C" {
    pub fn bbox_color_from_rgb(r: u8, g: u8, b: u8) -> bbox_color_t;
}
extern "C" {
    pub fn bbox_color_from_rgba(r: u8, g: u8, b: u8, a: u8) -> bbox_color_t;
}
extern "C" {
    pub fn bbox_clear(self_: *mut bbox_t) -> bool;
}
extern "C" {
    pub fn bbox_color(self_: *mut bbox_t, color: bbox_color_t) -> bool;
}
extern "C" {
    pub fn bbox_style_outline(self_: *mut bbox_t) -> bool;
}
extern "C" {
    pub fn bbox_style_corners(self_: *mut bbox_t) -> bool;
}
extern "C" {
    pub fn bbox_thickness_thin(self_: *mut bbox_t) -> bool;
}
extern "C" {
    pub fn bbox_thickness_medium(self_: *mut bbox_t) -> bool;
}
extern "C" {
    pub fn bbox_thickness_thick(self_: *mut bbox_t) -> bool;
}
extern "C" {
    pub fn bbox_rectangle(self_: *mut bbox_t, x1: f32, y1: f32, x2: f32, y2: f32) -> bool;
}
extern "C" {
    pub fn bbox_line(self_: *mut bbox_t, x1: f32, y1: f32, x2: f32, y2: f32) -> bool;
}
extern "C" {
    pub fn bbox_quad(
        self_: *mut bbox_t,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x3: f32,
        y3: f32,
        x4: f32,
        y4: f32,
    ) -> bool;
}
extern "C" {
    pub fn bbox_move_to(self_: *mut bbox_t, x: f32, y: f32) -> bool;
}
extern "C" {
    pub fn bbox_line_to(self_: *mut bbox_t, x: f32, y: f32) -> bool;
}
extern "C" {
    pub fn bbox_draw_path(self_: *mut bbox_t) -> bool;
}
extern "C" {
    pub fn bbox_commit(self_: *mut bbox_t, when_us: i64) -> bool;
}
