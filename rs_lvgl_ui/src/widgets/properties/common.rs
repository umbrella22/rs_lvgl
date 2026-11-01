//! Shared native text and checked-state operations.

use super::suppress;
use crate::events::HandlerSlot;
use core::ptr::NonNull;
use cstr_core::CString;
use rs_lvgl_sys::lv_obj_t;

pub(super) fn set_label_text(obj: NonNull<lv_obj_t>, text: &str) {
    let Ok(text) = CString::new(text) else {
        return;
    };
    // SAFETY: `obj` is a live label and `text` is a valid NUL-terminated
    // string. LVGL copies the text into its own allocation.
    #[cfg(feature = "update-metrics")]
    crate::metrics::label_text_set();
    unsafe { rs_lvgl_sys::lv_label_set_text(obj.as_ptr(), text.as_ptr()) };
}

#[cfg(any(feature = "widget-switch", feature = "widget-checkbox"))]
pub(super) fn is_checked(obj: NonNull<lv_obj_t>) -> bool {
    // SAFETY: `obj` is a live object; the state query only reads it.
    unsafe { rs_lvgl_sys::lv_obj_has_state(obj.as_ptr(), rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED) }
}

/// Whether the native textarea already shows `text`.
#[cfg(feature = "widget-textarea")]
pub(super) fn textarea_text_is(obj: NonNull<lv_obj_t>, text: &str) -> bool {
    // SAFETY: `obj` is a live textarea. The returned buffer is owned by LVGL
    // and is only read before this function returns.
    unsafe {
        let current = rs_lvgl_sys::lv_textarea_get_text(obj.as_ptr());
        if current.is_null() {
            return text.is_empty();
        }
        cstr_core::CStr::from_ptr(current).to_bytes() == text.as_bytes()
    }
}

#[cfg(any(feature = "widget-switch", feature = "widget-checkbox"))]
pub(super) fn write_checked<M: Clone + 'static>(
    obj: NonNull<lv_obj_t>,
    checked: bool,
    slot: Option<&HandlerSlot<M>>,
) {
    // SAFETY: the property dispatcher supplies a live switch or checkbox.
    suppress(slot, || unsafe {
        if checked {
            rs_lvgl_sys::lv_obj_add_state(obj.as_ptr(), rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED);
        } else {
            rs_lvgl_sys::lv_obj_remove_state(
                obj.as_ptr(),
                rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED,
            );
        }
    });
}
