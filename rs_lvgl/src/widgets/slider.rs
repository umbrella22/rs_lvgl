use crate::lv_core::obj::NativeObject;
use crate::widgets::Slider;
use crate::AnimationState;

#[repr(u32)]
pub enum SliderMode {
    Normal = rs_lvgl_sys::lv_slider_mode_t_LV_SLIDER_MODE_NORMAL,
    Symmetrical = rs_lvgl_sys::lv_slider_mode_t_LV_SLIDER_MODE_SYMMETRICAL,
    Range = rs_lvgl_sys::lv_slider_mode_t_LV_SLIDER_MODE_RANGE,
}

impl From<SliderMode> for rs_lvgl_sys::lv_slider_mode_t {
    fn from(value: SliderMode) -> Self {
        value as rs_lvgl_sys::lv_slider_mode_t
    }
}

#[repr(u32)]
pub enum SliderOrientation {
    Auto = rs_lvgl_sys::lv_slider_orientation_t_LV_SLIDER_ORIENTATION_AUTO,
    Horizontal = rs_lvgl_sys::lv_slider_orientation_t_LV_SLIDER_ORIENTATION_HORIZONTAL,
    Vertical = rs_lvgl_sys::lv_slider_orientation_t_LV_SLIDER_ORIENTATION_VERTICAL,
}

impl From<SliderOrientation> for rs_lvgl_sys::lv_slider_orientation_t {
    fn from(value: SliderOrientation) -> Self {
        value as rs_lvgl_sys::lv_slider_orientation_t
    }
}

impl Slider<'_> {
    pub fn set_range(&self, min: i32, max: i32) {
        unsafe { rs_lvgl_sys::lv_slider_set_range(self.core.raw().as_ptr(), min, max) }
    }

    pub fn set_value(&self, value: i32, anim: AnimationState) {
        unsafe { rs_lvgl_sys::lv_slider_set_value(self.core.raw().as_ptr(), value, anim.into()) }
    }

    pub fn set_mode(&self, mode: SliderMode) {
        unsafe { rs_lvgl_sys::lv_slider_set_mode(self.core.raw().as_ptr(), mode.into()) }
    }

    pub fn set_orientation(&self, orientation: SliderOrientation) {
        unsafe {
            rs_lvgl_sys::lv_slider_set_orientation(self.core.raw().as_ptr(), orientation.into())
        }
    }

    pub fn get_value(&self) -> i32 {
        unsafe { rs_lvgl_sys::lv_slider_get_value(self.core.raw().as_ptr()) }
    }

    pub fn get_left_value(&self) -> i32 {
        unsafe { rs_lvgl_sys::lv_slider_get_left_value(self.core.raw().as_ptr()) }
    }

    pub fn get_min_value(&self) -> i32 {
        unsafe { rs_lvgl_sys::lv_slider_get_min_value(self.core.raw().as_ptr()) }
    }

    pub fn get_max_value(&self) -> i32 {
        unsafe { rs_lvgl_sys::lv_slider_get_max_value(self.core.raw().as_ptr()) }
    }

    pub fn get_mode(&self) -> SliderMode {
        match unsafe { rs_lvgl_sys::lv_slider_get_mode(self.core.raw().as_ptr()) } {
            rs_lvgl_sys::lv_slider_mode_t_LV_SLIDER_MODE_NORMAL => SliderMode::Normal,
            rs_lvgl_sys::lv_slider_mode_t_LV_SLIDER_MODE_SYMMETRICAL => SliderMode::Symmetrical,
            rs_lvgl_sys::lv_slider_mode_t_LV_SLIDER_MODE_RANGE => SliderMode::Range,
            _ => unreachable!("unknown lv_slider_mode_t value"),
        }
    }

    pub fn get_orientation(&self) -> SliderOrientation {
        match unsafe { rs_lvgl_sys::lv_slider_get_orientation(self.core.raw().as_ptr()) } {
            rs_lvgl_sys::lv_slider_orientation_t_LV_SLIDER_ORIENTATION_AUTO => {
                SliderOrientation::Auto
            }
            rs_lvgl_sys::lv_slider_orientation_t_LV_SLIDER_ORIENTATION_HORIZONTAL => {
                SliderOrientation::Horizontal
            }
            rs_lvgl_sys::lv_slider_orientation_t_LV_SLIDER_ORIENTATION_VERTICAL => {
                SliderOrientation::Vertical
            }
            _ => unreachable!("unknown lv_slider_orientation_t value"),
        }
    }
}
