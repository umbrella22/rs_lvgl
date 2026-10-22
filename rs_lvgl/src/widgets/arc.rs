use crate::lv_core::obj::NativeObject;
use crate::widgets::Arc;

#[repr(u32)]
pub enum ArcMode {
    Normal = rs_lvgl_sys::lv_arc_mode_t_LV_ARC_MODE_NORMAL,
    Symmetrical = rs_lvgl_sys::lv_arc_mode_t_LV_ARC_MODE_SYMMETRICAL,
    Reverse = rs_lvgl_sys::lv_arc_mode_t_LV_ARC_MODE_REVERSE,
}

impl From<ArcMode> for rs_lvgl_sys::lv_arc_mode_t {
    fn from(value: ArcMode) -> Self {
        value as rs_lvgl_sys::lv_arc_mode_t
    }
}

impl Arc<'_> {
    pub fn set_start_angle(&mut self, start: rs_lvgl_sys::lv_value_precise_t) {
        unsafe { rs_lvgl_sys::lv_arc_set_start_angle(self.core.raw().as_ptr(), start) }
    }

    pub fn set_end_angle(&mut self, end: rs_lvgl_sys::lv_value_precise_t) {
        unsafe { rs_lvgl_sys::lv_arc_set_end_angle(self.core.raw().as_ptr(), end) }
    }

    pub fn set_angles(
        &mut self,
        start: rs_lvgl_sys::lv_value_precise_t,
        end: rs_lvgl_sys::lv_value_precise_t,
    ) {
        unsafe { rs_lvgl_sys::lv_arc_set_angles(self.core.raw().as_ptr(), start, end) }
    }

    pub fn set_bg_start_angle(&mut self, start: rs_lvgl_sys::lv_value_precise_t) {
        unsafe { rs_lvgl_sys::lv_arc_set_bg_start_angle(self.core.raw().as_ptr(), start) }
    }

    pub fn set_bg_end_angle(&mut self, end: rs_lvgl_sys::lv_value_precise_t) {
        unsafe { rs_lvgl_sys::lv_arc_set_bg_end_angle(self.core.raw().as_ptr(), end) }
    }

    pub fn set_bg_angles(
        &mut self,
        start: rs_lvgl_sys::lv_value_precise_t,
        end: rs_lvgl_sys::lv_value_precise_t,
    ) {
        unsafe { rs_lvgl_sys::lv_arc_set_bg_angles(self.core.raw().as_ptr(), start, end) }
    }

    pub fn set_rotation(&mut self, rotation: i32) {
        unsafe { rs_lvgl_sys::lv_arc_set_rotation(self.core.raw().as_ptr(), rotation) }
    }

    pub fn set_mode(&mut self, mode: ArcMode) {
        unsafe { rs_lvgl_sys::lv_arc_set_mode(self.core.raw().as_ptr(), mode.into()) }
    }

    pub fn set_value(&mut self, value: i32) {
        unsafe { rs_lvgl_sys::lv_arc_set_value(self.core.raw().as_ptr(), value) }
    }

    pub fn set_range(&mut self, min: i32, max: i32) {
        unsafe { rs_lvgl_sys::lv_arc_set_range(self.core.raw().as_ptr(), min, max) }
    }

    pub fn set_change_rate(&mut self, rate: u32) {
        unsafe { rs_lvgl_sys::lv_arc_set_change_rate(self.core.raw().as_ptr(), rate) }
    }

    pub fn set_knob_offset(&mut self, offset: i32) {
        unsafe { rs_lvgl_sys::lv_arc_set_knob_offset(self.core.raw().as_ptr(), offset) }
    }

    pub fn get_angle_start(&self) -> rs_lvgl_sys::lv_value_precise_t {
        unsafe { rs_lvgl_sys::lv_arc_get_angle_start(self.core.raw().as_ptr()) }
    }

    pub fn get_angle_end(&self) -> rs_lvgl_sys::lv_value_precise_t {
        unsafe { rs_lvgl_sys::lv_arc_get_angle_end(self.core.raw().as_ptr()) }
    }

    pub fn get_bg_angle_start(&self) -> rs_lvgl_sys::lv_value_precise_t {
        unsafe { rs_lvgl_sys::lv_arc_get_bg_angle_start(self.core.raw().as_ptr()) }
    }

    pub fn get_bg_angle_end(&self) -> rs_lvgl_sys::lv_value_precise_t {
        unsafe { rs_lvgl_sys::lv_arc_get_bg_angle_end(self.core.raw().as_ptr()) }
    }

    pub fn get_value(&self) -> i32 {
        unsafe { rs_lvgl_sys::lv_arc_get_value(self.core.raw().as_ptr()) }
    }

    pub fn get_min_value(&self) -> i32 {
        unsafe { rs_lvgl_sys::lv_arc_get_min_value(self.core.raw().as_ptr()) }
    }

    pub fn get_max_value(&self) -> i32 {
        unsafe { rs_lvgl_sys::lv_arc_get_max_value(self.core.raw().as_ptr()) }
    }

    pub fn get_mode(&self) -> ArcMode {
        match unsafe { rs_lvgl_sys::lv_arc_get_mode(self.core.raw().as_ptr()) } {
            rs_lvgl_sys::lv_arc_mode_t_LV_ARC_MODE_NORMAL => ArcMode::Normal,
            rs_lvgl_sys::lv_arc_mode_t_LV_ARC_MODE_SYMMETRICAL => ArcMode::Symmetrical,
            rs_lvgl_sys::lv_arc_mode_t_LV_ARC_MODE_REVERSE => ArcMode::Reverse,
            _ => unreachable!("unknown lv_arc_mode_t value"),
        }
    }

    pub fn get_rotation(&self) -> i32 {
        unsafe { rs_lvgl_sys::lv_arc_get_rotation(self.core.raw().as_ptr()) }
    }

    pub fn get_knob_offset(&self) -> i32 {
        unsafe { rs_lvgl_sys::lv_arc_get_knob_offset(self.core.raw().as_ptr()) }
    }

    pub fn get_change_rate(&self) -> u32 {
        unsafe { rs_lvgl_sys::lv_arc_get_change_rate(self.core.raw().as_ptr()) }
    }
}
