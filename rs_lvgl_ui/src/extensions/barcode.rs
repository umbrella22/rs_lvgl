use crate::{Color, Node, UiError, WidgetKind};
use alloc::string::String;
use core::ptr::NonNull;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum BarcodeEncoding {
    /// Preserves spaces; Code 128 ASCII data.
    Raw = rs_lvgl_sys::lv_barcode_encoding_t_LV_BARCODE_ENCODING_CODE128_RAW,
    /// Native GS1 convention removes spaces and interprets `[FNC1]`.
    Gs1 = rs_lvgl_sys::lv_barcode_encoding_t_LV_BARCODE_ENCODING_CODE128_GS1,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Barcode {
    pub data: String,
    pub height: u16,
    pub scale: u16,
    pub encoding: BarcodeEncoding,
    pub dark: Color,
    pub light: Color,
}
impl Barcode {
    pub fn new(data: impl Into<String>, height: u16) -> Self {
        Self {
            data: data.into(),
            height,
            scale: 1,
            encoding: BarcodeEncoding::Raw,
            dark: Color::rgb(0, 0, 0),
            light: Color::rgb(255, 255, 255),
        }
    }
    pub(crate) fn validate(&self) -> Result<(), UiError> {
        if self.height > i16::MAX as u16
            || self.height == 0
            || self.scale == 0
            || self.data.is_empty()
            || !self.data.is_ascii()
            || self.data.as_bytes().contains(&0)
        {
            return Err(UiError::InvalidProperty);
        }
        let text =
            cstr_core::CString::new(self.data.as_str()).map_err(|_| UiError::InvalidProperty)?;
        // SAFETY: NUL-terminated data is borrowed only for encoding; the shim
        // checks exact encoded width against the native 16-bit image header.
        if unsafe {
            rs_lvgl_sys::lv_rs_barcode_width(text.as_ptr(), self.encoding as _, self.scale)
        } == 0
        {
            return Err(UiError::InvalidProperty);
        }
        Ok(())
    }
    pub(crate) fn apply(&self, obj: NonNull<rs_lvgl_sys::lv_obj_t>) -> Result<(), UiError> {
        let text =
            cstr_core::CString::new(self.data.as_str()).map_err(|_| UiError::InvalidProperty)?;
        // SAFETY: live barcode, validated dimensions; native code copies text.
        unsafe {
            rs_lvgl_sys::lv_barcode_set_update_mode(
                obj.as_ptr(),
                rs_lvgl_sys::lv_barcode_update_mode_t_LV_BARCODE_UPDATE_MODE_DEFERRED,
            );
            rs_lvgl_sys::lv_obj_set_height(obj.as_ptr(), self.height.into());
            rs_lvgl_sys::lv_barcode_set_encoding(obj.as_ptr(), self.encoding as _);
            rs_lvgl_sys::lv_barcode_set_scale(obj.as_ptr(), self.scale);
            rs_lvgl_sys::lv_barcode_set_dark_color(obj.as_ptr(), self.dark.to_raw());
            rs_lvgl_sys::lv_barcode_set_light_color(obj.as_ptr(), self.light.to_raw());
            if rs_lvgl_sys::lv_barcode_update(obj.as_ptr(), text.as_ptr())
                != rs_lvgl_sys::lv_result_t_LV_RESULT_OK
                || rs_lvgl_sys::lv_barcode_render(obj.as_ptr())
                    != rs_lvgl_sys::lv_result_t_LV_RESULT_OK
            {
                return Err(UiError::InvalidProperty);
            }
        }
        Ok(())
    }
}
impl<M> Node<M> {
    pub fn barcode(code: Barcode) -> Self {
        let style = crate::Style::default().height(crate::Size::Px(code.height as i16));
        Self::new(WidgetKind::Barcode { code }).style(style)
    }
}
