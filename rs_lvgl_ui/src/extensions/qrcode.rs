use crate::{Color, UiError};
use alloc::vec::Vec;
use core::ptr::NonNull;

/// Binary QR payload. Embedded zero bytes are preserved. The native encoder
/// reports data that cannot fit the requested size as InvalidProperty.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QrCode {
    pub data: Vec<u8>,
    pub size: u16,
    pub dark: Color,
    pub light: Color,
    pub quiet_zone: bool,
}
impl QrCode {
    pub fn new(data: impl Into<Vec<u8>>, size: u16) -> Self {
        Self {
            data: data.into(),
            size,
            dark: Color::rgb(0, 0, 0),
            light: Color::rgb(255, 255, 255),
            quiet_zone: true,
        }
    }
    pub(crate) fn validate(&self) -> Result<(), UiError> {
        if self.size == 0 || self.data.len() > u32::MAX as usize {
            return Err(UiError::InvalidProperty);
        }
        Ok(())
    }
    pub(crate) fn apply(&self, obj: NonNull<rs_lvgl_sys::lv_obj_t>) -> Result<(), UiError> {
        // SAFETY: a live QR widget; LVGL copies the length-delimited payload.
        unsafe {
            rs_lvgl_sys::lv_qrcode_set_update_mode(
                obj.as_ptr(),
                rs_lvgl_sys::lv_qrcode_update_mode_t_LV_QRCODE_UPDATE_MODE_DEFERRED,
            );
            rs_lvgl_sys::lv_qrcode_set_size(obj.as_ptr(), self.size.into());
            rs_lvgl_sys::lv_qrcode_set_dark_color(obj.as_ptr(), self.dark.to_raw());
            rs_lvgl_sys::lv_qrcode_set_light_color(obj.as_ptr(), self.light.to_raw());
            rs_lvgl_sys::lv_qrcode_set_quiet_zone(obj.as_ptr(), self.quiet_zone);
            if rs_lvgl_sys::lv_qrcode_update(
                obj.as_ptr(),
                self.data.as_ptr().cast(),
                self.data.len() as u32,
            ) != rs_lvgl_sys::lv_result_t_LV_RESULT_OK
            {
                return Err(UiError::InvalidProperty);
            }
        }
        Ok(())
    }
}
