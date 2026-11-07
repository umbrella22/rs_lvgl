//! Property updates for text widgets.

use super::super::{Props, Retained};
#[cfg(feature = "widget-textarea")]
use super::common::textarea_text_is;
use super::suppress;
use crate::events::HandlerSlot;
use crate::UiError;
use alloc::vec::Vec;
use core::ptr::NonNull;
use cstr_core::CString;
use rs_lvgl_sys::lv_obj_t;

pub(super) fn apply<M: Clone + 'static>(
    obj: NonNull<lv_obj_t>,
    new: &Props,
    old: Option<&Props>,
    slot: Option<&HandlerSlot<M>>,
    retained: &mut Retained,
) -> Result<(), UiError> {
    match (new, old) {
        #[cfg(feature = "widget-textarea")]
        (
            Props::Textarea {
                text,
                placeholder,
                one_line,
                password,
                options,
            },
            old,
        ) => {
            let defaults = crate::TextareaInputOptions::default();
            let options = options.as_deref().unwrap_or(&defaults);
            let (old_placeholder, old_one_line, old_password) = match old {
                Some(Props::Textarea {
                    placeholder,
                    one_line,
                    password,
                    ..
                }) => (Some(placeholder.as_str()), Some(*one_line), Some(*password)),
                _ => (None, None, None),
            };
            let previous = match retained {
                Retained::TextInput { accepted } => accepted.as_ref().map(|s| s.as_bytes()),
                _ => None,
            };
            let filter_changed = previous != options.accepted.as_ref().map(|s| s.as_bytes());
            // Allocate the next backing before native writes. Heap storage remains
            // stable when its owner moves into the mounted retention slot.
            let replacement = if filter_changed {
                options
                    .accepted
                    .as_ref()
                    .map(|s| CString::new(s.as_str()))
                    .transpose()
                    .map_err(|_| UiError::InvalidProperty)?
            } else {
                None
            };
            let accepted_ptr = if filter_changed {
                replacement.as_ref().map(|s| s.as_ptr())
            } else {
                match retained {
                    Retained::TextInput { accepted } => accepted.as_ref().map(|s| s.as_ptr()),
                    _ => None,
                }
            }
            .unwrap_or(core::ptr::null());
            // SAFETY: the object is a live textarea. The static accepted pointer
            // refers to either the old mounted backing or the replacement above.
            // Both stay live through every synchronous setter and native event.
            unsafe {
                suppress(slot, || {
                    let raw = obj.as_ptr();
                    if old_one_line != Some(*one_line) {
                        rs_lvgl_sys::lv_textarea_set_one_line(raw, *one_line);
                    }
                    if old_password != Some(*password) {
                        rs_lvgl_sys::lv_textarea_set_password_mode(raw, *password);
                    }
                    if old_placeholder != Some(placeholder.as_str()) {
                        let value =
                            CString::new(placeholder.as_str()).expect("validated placeholder");
                        rs_lvgl_sys::lv_textarea_set_placeholder_text(raw, value.as_ptr());
                    }
                    if !textarea_text_is(obj, text) {
                        // Native set_text applies editing filters. Disable both
                        // while restoring canonical model text, including passwords.
                        rs_lvgl_sys::lv_textarea_set_accepted_chars_static(raw, core::ptr::null());
                        rs_lvgl_sys::lv_textarea_set_max_length(raw, 0);
                        let value = CString::new(text.as_str()).expect("validated text");
                        rs_lvgl_sys::lv_textarea_set_text(raw, value.as_ptr());
                    }
                    rs_lvgl_sys::lv_textarea_set_accepted_chars_static(raw, accepted_ptr);
                    rs_lvgl_sys::lv_textarea_set_max_length(
                        raw,
                        options.maximum.map_or(0, |n| n.get()),
                    );
                    rs_lvgl_sys::lv_textarea_set_cursor_click_pos(raw, options.click);
                    if let Some(position) = options.cursor {
                        rs_lvgl_sys::lv_textarea_set_cursor_pos(raw, position as i32);
                    }
                });
            }
            if filter_changed {
                // The native object now points at the replacement. Release only
                // the old backing; later errors keep this mounted backing alive
                // until the affected native tree has been deleted.
                *retained = Retained::TextInput {
                    accepted: replacement,
                };
            }
        }
        #[cfg(feature = "widget-span")]
        (Props::Spans { spans }, old) => {
            let old_spans = match old {
                Some(Props::Spans { spans }) => Some(spans.as_slice()),
                _ => None,
            };
            if old_spans == Some(spans.as_slice()) {
                return Ok(());
            }
            let texts = spans
                .iter()
                .map(|span| CString::new(span.text.as_str()).map_err(|_| UiError::InvalidProperty))
                .collect::<Result<Vec<_>, _>>()?;
            let reuse = old_spans.filter(|old| old.len() == spans.len());
            // SAFETY: `obj` is a live span group. Span handles returned by
            // LVGL stay valid until they are deleted from the group; texts
            // are copied.
            unsafe {
                if reuse.is_none() {
                    loop {
                        let span = rs_lvgl_sys::lv_spangroup_get_child(obj.as_ptr(), 0);
                        if span.is_null() {
                            break;
                        }
                        rs_lvgl_sys::lv_spangroup_delete_span(obj.as_ptr(), span);
                    }
                    for _ in spans {
                        if rs_lvgl_sys::lv_spangroup_add_span(obj.as_ptr()).is_null() {
                            return Err(UiError::Allocation);
                        }
                    }
                }
                for (index, (span, text)) in spans.iter().zip(&texts).enumerate() {
                    let previous = reuse.map(|old| &old[index]);
                    if previous == Some(span) {
                        continue;
                    }
                    let native = rs_lvgl_sys::lv_spangroup_get_child(obj.as_ptr(), index as i32);
                    if native.is_null() {
                        return Err(UiError::Allocation);
                    }
                    if previous.map(|old| old.text.as_str()) != Some(span.text.as_str()) {
                        rs_lvgl_sys::lv_span_set_text(native, text.as_ptr());
                    }
                    let style = rs_lvgl_sys::lv_span_get_style(native);
                    match span.color {
                        Some(color) => rs_lvgl_sys::lv_style_set_text_color(style, color.to_raw()),
                        None => {
                            rs_lvgl_sys::lv_style_remove_prop(
                                style,
                                rs_lvgl_sys::_lv_style_id_t_LV_STYLE_TEXT_COLOR
                                    as rs_lvgl_sys::lv_style_prop_t,
                            );
                        }
                    }
                    match span.font {
                        Some(font) => rs_lvgl_sys::lv_style_set_text_font(style, font.0),
                        None => {
                            rs_lvgl_sys::lv_style_remove_prop(
                                style,
                                rs_lvgl_sys::_lv_style_id_t_LV_STYLE_TEXT_FONT
                                    as rs_lvgl_sys::lv_style_prop_t,
                            );
                        }
                    }
                }
                rs_lvgl_sys::lv_spangroup_refresh(obj.as_ptr());
            }
        }
        #[cfg(feature = "widget-arclabel")]
        (
            Props::ArcLabel {
                text,
                radius,
                start_angle,
                angle_size,
            },
            old,
        ) => {
            let (old_text, old_radius, old_start, old_size) = match old {
                Some(Props::ArcLabel {
                    text,
                    radius,
                    start_angle,
                    angle_size,
                }) => (
                    Some(text.as_str()),
                    Some(*radius),
                    Some(*start_angle),
                    Some(*angle_size),
                ),
                _ => (None, None, None, None),
            };
            // SAFETY: `obj` is a live arc label; LVGL copies the text.
            unsafe {
                if old_text != Some(text.as_str()) {
                    let text = CString::new(text.as_str()).map_err(|_| UiError::InvalidProperty)?;
                    rs_lvgl_sys::lv_arclabel_set_text(obj.as_ptr(), text.as_ptr());
                }
                if old_radius != Some(*radius) {
                    rs_lvgl_sys::lv_arclabel_set_radius(obj.as_ptr(), *radius);
                }
                if old_start != Some(*start_angle) {
                    rs_lvgl_sys::lv_arclabel_set_angle_start(obj.as_ptr(), *start_angle as _);
                }
                if old_size != Some(*angle_size) {
                    rs_lvgl_sys::lv_arclabel_set_angle_size(obj.as_ptr(), *angle_size as _);
                }
            }
        }
        _ => unreachable!("property dispatcher selected a different widget family"),
    }
    Ok(())
}
