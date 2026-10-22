use crate::{widgets::Retained, Node, UiError, WidgetKind};
use alloc::rc::Rc;
use core::ptr::NonNull;
/// JSON Lottie animation rendered by the selected vendored ThorVG provider.
/// Changing source or dimensions recreates the widget; unchanged updates keep
/// its native playback position. No file loader or external provider is used.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Lottie {
    pub json: Rc<str>,
    pub width: u16,
    pub height: u16,
}
impl Lottie {
    pub fn new(json: impl Into<Rc<str>>, width: u16, height: u16) -> Self {
        Self {
            json: json.into(),
            width,
            height,
        }
    }
    pub(crate) fn validate(&self) -> Result<(), UiError> {
        crate::graphics::DrawBuffer::stride(self.width, self.height)?;
        if self.width > i16::MAX as u16
            || self.height > i16::MAX as u16
            || self.json.is_empty()
            || self.json.len() > u32::MAX as usize
            || self.json.as_bytes().contains(&0)
        {
            return Err(UiError::InvalidProperty);
        }
        Ok(())
    }
    pub(crate) fn apply(
        &self,
        obj: NonNull<rs_lvgl_sys::lv_obj_t>,
        retained: &mut Retained,
    ) -> Result<(), UiError> {
        let buffer = crate::graphics::DrawBuffer::new(self.width, self.height)?;
        let json =
            cstr_core::CString::new(self.json.as_ref()).map_err(|_| UiError::InvalidProperty)?;
        // SAFETY: live Lottie; the aligned target is retained. The checked shim
        // uses ThorVG's copying loader and rejects invalid clock metadata.
        unsafe {
            rs_lvgl_sys::lv_lottie_set_draw_buf(obj.as_ptr(), buffer.0.as_ptr());
            *retained = Retained::Lottie(buffer);
            if !rs_lvgl_sys::lv_rs_lottie_set_source(
                obj.as_ptr(),
                json.as_ptr(),
                self.json.len() as u32,
            ) {
                return Err(UiError::InvalidProperty);
            }
        }
        Ok(())
    }
}
impl<M> Node<M> {
    pub fn lottie(animation: Lottie) -> Self {
        let style = crate::Style::default()
            .width(crate::Size::Px(animation.width as i16))
            .height(crate::Size::Px(animation.height as i16));
        Self::new(WidgetKind::Lottie { animation }).style(style)
    }
}
