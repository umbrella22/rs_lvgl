use crate::support::AnimationState;
use crate::widgets::Bar;
use crate::NativeObject;

#[repr(u32)]
pub enum BarMode {
    Normal = rs_lvgl_sys::lv_bar_mode_t_LV_BAR_MODE_NORMAL,
    Symmetrical = rs_lvgl_sys::lv_bar_mode_t_LV_BAR_MODE_SYMMETRICAL,
    Range = rs_lvgl_sys::lv_bar_mode_t_LV_BAR_MODE_RANGE,
}

impl From<BarMode> for rs_lvgl_sys::lv_bar_mode_t {
    fn from(value: BarMode) -> Self {
        value as rs_lvgl_sys::lv_bar_mode_t
    }
}

#[repr(u32)]
pub enum BarOrientation {
    Auto = rs_lvgl_sys::lv_bar_orientation_t_LV_BAR_ORIENTATION_AUTO,
    Horizontal = rs_lvgl_sys::lv_bar_orientation_t_LV_BAR_ORIENTATION_HORIZONTAL,
    Vertical = rs_lvgl_sys::lv_bar_orientation_t_LV_BAR_ORIENTATION_VERTICAL,
}

impl From<BarOrientation> for rs_lvgl_sys::lv_bar_orientation_t {
    fn from(value: BarOrientation) -> Self {
        value as rs_lvgl_sys::lv_bar_orientation_t
    }
}

impl Bar<'_> {
    pub fn set_range(&mut self, min: i32, max: i32) {
        unsafe {
            rs_lvgl_sys::lv_bar_set_range(self.core.raw().as_mut(), min, max);
        }
    }

    pub fn set_value(&mut self, value: i32, anim: AnimationState) {
        unsafe {
            rs_lvgl_sys::lv_bar_set_value(self.core.raw().as_mut(), value, anim.into());
        }
    }

    pub fn set_mode(&mut self, mode: BarMode) {
        unsafe { rs_lvgl_sys::lv_bar_set_mode(self.core.raw().as_mut(), mode.into()) }
    }

    pub fn set_orientation(&mut self, orientation: BarOrientation) {
        unsafe { rs_lvgl_sys::lv_bar_set_orientation(self.core.raw().as_mut(), orientation.into()) }
    }

    pub fn get_value(&self) -> i32 {
        unsafe { rs_lvgl_sys::lv_bar_get_value(self.core.raw().as_ptr()) }
    }

    pub fn get_min_value(&self) -> i32 {
        unsafe { rs_lvgl_sys::lv_bar_get_min_value(self.core.raw().as_ptr()) }
    }

    pub fn get_max_value(&self) -> i32 {
        unsafe { rs_lvgl_sys::lv_bar_get_max_value(self.core.raw().as_ptr()) }
    }

    pub fn get_mode(&self) -> BarMode {
        match unsafe { rs_lvgl_sys::lv_bar_get_mode(self.core.raw().as_ptr()) } {
            rs_lvgl_sys::lv_bar_mode_t_LV_BAR_MODE_NORMAL => BarMode::Normal,
            rs_lvgl_sys::lv_bar_mode_t_LV_BAR_MODE_SYMMETRICAL => BarMode::Symmetrical,
            rs_lvgl_sys::lv_bar_mode_t_LV_BAR_MODE_RANGE => BarMode::Range,
            _ => unreachable!("unknown lv_bar_mode_t value"),
        }
    }

    pub fn get_orientation(&self) -> BarOrientation {
        match unsafe { rs_lvgl_sys::lv_bar_get_orientation(self.core.raw().as_ptr()) } {
            rs_lvgl_sys::lv_bar_orientation_t_LV_BAR_ORIENTATION_AUTO => BarOrientation::Auto,
            rs_lvgl_sys::lv_bar_orientation_t_LV_BAR_ORIENTATION_HORIZONTAL => {
                BarOrientation::Horizontal
            }
            rs_lvgl_sys::lv_bar_orientation_t_LV_BAR_ORIENTATION_VERTICAL => {
                BarOrientation::Vertical
            }
            _ => unreachable!("unknown lv_bar_orientation_t value"),
        }
    }
}
/*
/// The different parts, of a bar object.
pub enum BarPart {
    /// The background of the bar.
    Background,
    /// The indicator of the bar.
    /// This is what moves/changes, depending on the bar's value.
    Indicator,
}

impl From<BarPart> for u8 {
    fn from(component: BarPart) -> Self {
        match component {
            BarPart::Background => rs_lvgl_sys::LV_BAR_PART_BG as u8,
            BarPart::Indicator => rs_lvgl_sys::LV_BAR_PART_INDIC as u8,
        }
    }
}
*/
