//! Property updates for messagebox widgets.

use super::super::Parts;
use super::super::Props;
use super::common::set_label_text;
use crate::UiError;
use core::ptr::NonNull;
use cstr_core::CString;
use rs_lvgl_sys::lv_obj_t;

pub(super) fn apply(
    obj: NonNull<lv_obj_t>,
    new: &Props,
    old: Option<&Props>,
    parts: Parts,
) -> Result<(), UiError> {
    match (new, old) {
        #[cfg(feature = "widget-msgbox")]
        (
            Props::MessageBox {
                title,
                text,
                buttons,
            },
            old,
        ) => {
            let (old_title, old_text, old_buttons) = match old {
                Some(Props::MessageBox {
                    title,
                    text,
                    buttons,
                }) => (
                    Some(title.as_deref()),
                    Some(text.as_str()),
                    Some(buttons.as_slice()),
                ),
                _ => (None, None, None),
            };
            if old_text != Some(text.as_str()) {
                if let Some(label) = parts.label {
                    set_label_text(label, text);
                }
            }
            if let Some(title) = title {
                if old_title.is_none() {
                    let title =
                        CString::new(title.as_str()).map_err(|_| UiError::InvalidProperty)?;
                    // SAFETY: `obj` is a live message box; LVGL copies the text.
                    let label =
                        unsafe { rs_lvgl_sys::lv_msgbox_add_title(obj.as_ptr(), title.as_ptr()) };
                    if label.is_null() {
                        return Err(UiError::Allocation);
                    }
                } else if old_title != Some(Some(title.as_str())) {
                    // SAFETY: `obj` is a live message box that has a title.
                    let label = unsafe { rs_lvgl_sys::lv_msgbox_get_title(obj.as_ptr()) };
                    if let Some(label) = NonNull::new(label) {
                        set_label_text(label, title);
                    }
                }
            }
            match old_buttons {
                None => {
                    for (index, text) in buttons.iter().enumerate() {
                        add_footer_button(obj, index, text)?;
                    }
                }
                Some(old_buttons) => {
                    // `needs_recreate` guarantees the same button count.
                    // SAFETY: `obj` is a live message box.
                    let footer = unsafe { rs_lvgl_sys::lv_msgbox_get_footer(obj.as_ptr()) };
                    for (index, (old, text)) in old_buttons.iter().zip(buttons).enumerate() {
                        if old == text || footer.is_null() {
                            continue;
                        }
                        // SAFETY: the footer holds one button per entry, each
                        // with its label as the first child.
                        let label = unsafe {
                            let button = rs_lvgl_sys::lv_obj_get_child(footer, index as i32);
                            if button.is_null() {
                                continue;
                            }
                            rs_lvgl_sys::lv_obj_get_child(button, 0)
                        };
                        if let Some(label) = NonNull::new(label) {
                            set_label_text(label, text);
                        }
                    }
                }
            }
        }
        _ => unreachable!("property dispatcher selected a different widget family"),
    }
    Ok(())
}

/// Add one message box footer button that reports its index.
#[cfg(feature = "widget-msgbox")]
fn add_footer_button(obj: NonNull<lv_obj_t>, index: usize, text: &str) -> Result<(), UiError> {
    let text = CString::new(text).map_err(|_| UiError::InvalidProperty)?;
    // SAFETY: `obj` is a live message box; LVGL copies the text. The
    // callback carries only the index and needs no cleanup.
    unsafe {
        let button = rs_lvgl_sys::lv_msgbox_add_footer_button(obj.as_ptr(), text.as_ptr());
        if button.is_null() {
            return Err(UiError::Allocation);
        }
        let dsc = rs_lvgl_sys::lv_obj_add_event_cb(
            button,
            Some(footer_button_clicked),
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED,
            (index + 1) as *mut cty::c_void,
        );
        if dsc.is_null() {
            return Err(UiError::Allocation);
        }
    }
    Ok(())
}

/// Forward a footer button click to its message box as a value change whose
/// parameter is the button index plus one.
///
/// # Safety
///
/// Called by LVGL with a live event whose target is a footer button
/// created by [`add_footer_button`]. The callback never unwinds.
#[cfg(feature = "widget-msgbox")]
unsafe extern "C" fn footer_button_clicked(event: *mut rs_lvgl_sys::lv_event_t) {
    // SAFETY: footer buttons are children of the footer, which is a child of
    // the message box; all are alive during the event.
    unsafe {
        let button = rs_lvgl_sys::lv_event_get_current_target_obj(event);
        let footer = rs_lvgl_sys::lv_obj_get_parent(button);
        let msgbox = rs_lvgl_sys::lv_obj_get_parent(footer);
        if msgbox.is_null() {
            return;
        }
        rs_lvgl_sys::lv_obj_send_event(
            msgbox,
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            rs_lvgl_sys::lv_event_get_user_data(event),
        );
    }
}
