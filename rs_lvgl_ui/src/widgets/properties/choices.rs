//! Property updates for choices widgets.

use super::super::Props;
use super::super::Retained;
#[cfg(any(feature = "widget-switch", feature = "widget-checkbox"))]
use super::common::{is_checked, write_checked};
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
        #[cfg(feature = "widget-switch")]
        (Props::Switch { checked }, _) => {
            if is_checked(obj) != *checked {
                write_checked(obj, *checked, slot);
            }
        }
        #[cfg(feature = "widget-checkbox")]
        (Props::Checkbox { text, checked }, old) => {
            let text_changed = match old {
                Some(Props::Checkbox { text: old_text, .. }) => old_text != text,
                _ => true,
            };
            let checked_changed = is_checked(obj) != *checked;
            // SAFETY: `obj` is a live checkbox object.
            unsafe {
                if text_changed {
                    if let Ok(text) = CString::new(text.as_str()) {
                        rs_lvgl_sys::lv_checkbox_set_text(obj.as_ptr(), text.as_ptr());
                    }
                }
                if checked_changed {
                    write_checked(obj, *checked, slot);
                }
            }
        }
        #[cfg(feature = "widget-dropdown")]
        (Props::Dropdown { options, selected }, old) => {
            let options_changed = match old {
                Some(Props::Dropdown {
                    options: old_options,
                    ..
                }) => old_options != options,
                _ => true,
            };
            // SAFETY: `obj` is a live dropdown object.
            unsafe {
                if options_changed {
                    let joined = options.join("\n");
                    if let Ok(options) = CString::new(joined) {
                        suppress(slot, || {
                            rs_lvgl_sys::lv_dropdown_set_options(obj.as_ptr(), options.as_ptr());
                            rs_lvgl_sys::lv_dropdown_set_selected(obj.as_ptr(), *selected);
                        });
                    }
                } else if rs_lvgl_sys::lv_dropdown_get_selected(obj.as_ptr()) != *selected {
                    suppress(slot, || {
                        rs_lvgl_sys::lv_dropdown_set_selected(obj.as_ptr(), *selected);
                    });
                }
            }
        }
        #[cfg(feature = "widget-roller")]
        (
            Props::Roller {
                options,
                selected,
                visible_rows,
                mode,
            },
            old,
        ) => {
            let (options_changed, rows_changed) = match old {
                Some(Props::Roller {
                    options: old_options,
                    visible_rows: old_rows,
                    mode: old_mode,
                    ..
                }) => (
                    old_options != options || old_mode != mode,
                    old_rows != visible_rows,
                ),
                _ => (true, true),
            };
            if options_changed {
                let joined =
                    CString::new(options.join("\n")).map_err(|_| UiError::InvalidProperty)?;
                // SAFETY: `obj` is a live roller; LVGL copies the options.
                suppress(slot, || unsafe {
                    rs_lvgl_sys::lv_roller_set_options(
                        obj.as_ptr(),
                        joined.as_ptr(),
                        mode.to_raw(),
                    );
                });
            }
            // SAFETY: `obj` is a live roller.
            unsafe {
                if rows_changed {
                    rs_lvgl_sys::lv_roller_set_visible_row_count(obj.as_ptr(), *visible_rows);
                }
                if rs_lvgl_sys::lv_roller_get_selected(obj.as_ptr()) != *selected {
                    suppress(slot, || {
                        rs_lvgl_sys::lv_roller_set_selected(obj.as_ptr(), *selected, false);
                    });
                }
            }
        }
        #[cfg(feature = "widget-buttonmatrix")]
        (Props::ButtonMatrix { rows }, old) => {
            let changed = match old {
                Some(Props::ButtonMatrix { rows: old_rows }) => old_rows != rows,
                _ => true,
            } || !matches!(retained, Retained::Map { .. });
            if changed {
                const ROW_BREAK: &[u8] = b"\n\0";
                const END: &[u8] = b"\0";
                let mut texts = Vec::new();
                for text in rows.iter().flatten() {
                    texts.push(CString::new(text.as_str()).map_err(|_| UiError::InvalidProperty)?);
                }
                let mut pointers = Vec::with_capacity(texts.len() + rows.len());
                let mut next = texts.iter();
                for (index, row) in rows.iter().enumerate() {
                    if index > 0 {
                        pointers.push(ROW_BREAK.as_ptr().cast());
                    }
                    for _ in row {
                        if let Some(text) = next.next() {
                            pointers.push(text.as_ptr());
                        }
                    }
                }
                pointers.push(END.as_ptr().cast());
                // SAFETY: `obj` is a live button matrix. LVGL keeps the map
                // pointer and the strings; both heap buffers move into
                // `retained` without changing address, and the old map is
                // dropped only after LVGL switched to the new one.
                unsafe { rs_lvgl_sys::lv_buttonmatrix_set_map(obj.as_ptr(), pointers.as_ptr()) };
                *retained = Retained::Map {
                    _texts: texts,
                    _pointers: pointers,
                };
            }
        }
        _ => unreachable!("property dispatcher selected a different widget family"),
    }
    Ok(())
}
