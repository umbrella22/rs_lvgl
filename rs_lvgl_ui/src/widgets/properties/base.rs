//! Property updates for base widgets.

use super::super::Parts;
use super::super::Props;
use super::common::set_label_text;
use crate::UiError;
use core::ptr::NonNull;
use rs_lvgl_sys::lv_obj_t;

pub(super) fn apply(
    obj: NonNull<lv_obj_t>,
    new: &Props,
    old: Option<&Props>,
    parts: Parts,
) -> Result<(), UiError> {
    match (new, old) {
        (
            Props::Label {
                text,
                text_overflow,
            },
            old,
        ) => {
            let (old_text, old_mode) = match old {
                Some(Props::Label {
                    text,
                    text_overflow,
                }) => (Some(text.as_ref()), Some(*text_overflow)),
                _ => (None, None),
            };
            if old_text != Some(text.as_ref()) {
                set_label_text(obj, text);
            }
            if old_mode != Some(*text_overflow) {
                // SAFETY: `obj` is a live label object.
                unsafe {
                    rs_lvgl_sys::lv_label_set_long_mode(obj.as_ptr(), text_overflow.to_raw())
                };
            }
        }
        (Props::Button { text }, old) => {
            let changed = match old {
                Some(Props::Button { text: old }) => old != text,
                _ => true,
            };
            if changed {
                if let Some(label) = parts.label {
                    set_label_text(label, text);
                }
            }
        }
        _ => unreachable!("property dispatcher selected a different widget family"),
    }
    Ok(())
}
