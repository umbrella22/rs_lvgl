use crate::{widgets::Retained, UiError};
use alloc::{boxed::Box, rc::Rc};
use core::ptr::NonNull;
/// Encoded GIF bytes retained for native timer decoding. Clones share bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Gif {
    pub data: Rc<[u8]>,
    pub paused: bool,
}
impl Gif {
    pub fn new(data: impl Into<Rc<[u8]>>) -> Self {
        Self {
            data: data.into(),
            paused: false,
        }
    }
    pub(crate) fn validate(&self) -> Result<(), UiError> {
        if self.data.len() < 13
            || self.data.len() > i32::MAX as usize
            || !matches!(&self.data[..6], b"GIF87a" | b"GIF89a")
        {
            return Err(UiError::InvalidProperty);
        }
        let width = u16::from_le_bytes([self.data[6], self.data[7]]);
        let height = u16::from_le_bytes([self.data[8], self.data[9]]);
        crate::graphics::DrawBuffer::stride(width, height)?;
        if u32::from(width) > rs_lvgl_sys::LV_GIF_MAX_WIDTH
            || u32::from(height) > rs_lvgl_sys::LV_GIF_MAX_HEIGHT
        {
            return Err(UiError::InvalidProperty);
        }
        Ok(())
    }
    pub(crate) fn apply(
        &self,
        obj: NonNull<rs_lvgl_sys::lv_obj_t>,
        old: Option<&Self>,
        retained: &mut Retained,
    ) -> Result<(), UiError> {
        // SAFETY: descriptor and encoded source allocations are retained until
        // source replacement or native deletion, including while paused.
        unsafe {
            if old.is_none_or(|old| old.data != self.data) {
                let data = self.data.clone();
                let mut descriptor = Box::<rs_lvgl_sys::lv_image_dsc_t>::default();
                descriptor
                    .header
                    .set_magic(rs_lvgl_sys::LV_IMAGE_HEADER_MAGIC);
                descriptor
                    .header
                    .set_cf(rs_lvgl_sys::lv_color_format_t_LV_COLOR_FORMAT_RAW);
                descriptor.data_size = data.len() as u32;
                descriptor.data = data.as_ptr();
                rs_lvgl_sys::lv_gif_set_src(
                    obj.as_ptr(),
                    (&*descriptor as *const rs_lvgl_sys::lv_image_dsc_t).cast(),
                );
                *retained = Retained::Gif {
                    _data: data,
                    _descriptor: descriptor,
                };
                if !rs_lvgl_sys::lv_gif_is_loaded(obj.as_ptr()) {
                    return Err(UiError::InvalidProperty);
                }
            }
            if self.paused {
                rs_lvgl_sys::lv_gif_pause(obj.as_ptr());
            } else {
                rs_lvgl_sys::lv_gif_resume(obj.as_ptr());
            }
        }
        Ok(())
    }
}
