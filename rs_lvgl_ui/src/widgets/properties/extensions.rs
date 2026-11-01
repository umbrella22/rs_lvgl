//! Property updates for extensions widgets.

use super::super::Parts;
use super::super::Props;
use super::super::Retained;
use crate::UiError;
use core::ptr::NonNull;
use rs_lvgl_sys::lv_obj_t;

pub(super) fn apply(
    obj: NonNull<lv_obj_t>,
    new: &Props,
    old: Option<&Props>,
    parts: Parts,
    retained: &mut Retained,
) -> Result<(), UiError> {
    match (new, old) {
        #[cfg(feature = "widget-ime-pinyin")]
        (Props::PinyinKeyboard { font, .. }, _) => {
            // SAFETY: the hidden IME and its candidate panel are live inside
            // this private container; LVGL propagates its font to candidates.
            unsafe {
                rs_lvgl_sys::lv_obj_set_style_text_font(
                    parts.ime.expect("IME part").as_ptr(),
                    font.0,
                    0,
                );
            }
        }
        #[cfg(feature = "widget-qrcode")]
        (Props::QrCode { code }, old) => {
            if !matches!(old, Some(Props::QrCode { code: previous }) if previous == code) {
                code.apply(obj)?;
            }
        }
        #[cfg(feature = "widget-barcode")]
        (Props::Barcode { code }, old) => {
            if !matches!(old, Some(Props::Barcode { code: previous }) if previous == code) {
                code.apply(obj)?;
            }
        }
        #[cfg(feature = "widget-gif")]
        (Props::Gif { animation }, old) => {
            let old = match old {
                Some(Props::Gif { animation }) => Some(animation),
                _ => None,
            };
            if old != Some(animation) {
                animation.apply(obj, old, retained)?;
            }
        }
        #[cfg(feature = "widget-lottie")]
        (Props::Lottie { animation }, old) => {
            if !matches!(old, Some(Props::Lottie { animation: previous }) if previous == animation)
            {
                animation.apply(obj, retained)?;
            }
        }
        #[cfg(feature = "widget-canvas")]
        (Props::Canvas { frame }, old) => {
            if matches!(old, Some(Props::Canvas { frame: previous }) if previous == frame) {
                return Ok(());
            }
            let resized = !matches!(old, Some(Props::Canvas { frame: previous }) if previous.width == frame.width && previous.height == frame.height);
            if resized {
                let buffer = crate::graphics::DrawBuffer::new(frame.width, frame.height)?;
                // SAFETY: the new stable buffer is installed before releasing
                // the old buffer retained by this live canvas.
                unsafe {
                    rs_lvgl_sys::lv_canvas_set_draw_buf(obj.as_ptr(), buffer.0.as_ptr());
                }
                *retained = Retained::Canvas(buffer);
            }
            crate::canvas::draw(obj, frame);
        }
        _ => unreachable!("property dispatcher selected a different widget family"),
    }
    Ok(())
}
