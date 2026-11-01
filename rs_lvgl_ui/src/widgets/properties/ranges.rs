//! Property updates for ranges widgets.

use super::super::Props;
use super::suppress;
use crate::events::HandlerSlot;
use crate::UiError;
use core::ptr::NonNull;
use rs_lvgl_sys::lv_obj_t;

pub(super) fn apply<M: Clone + 'static>(
    obj: NonNull<lv_obj_t>,
    new: &Props,
    old: Option<&Props>,
    slot: Option<&HandlerSlot<M>>,
) -> Result<(), UiError> {
    match (new, old) {
        #[cfg(feature = "widget-slider")]
        (Props::Slider { min, max, value }, old) => {
            let range_changed = match old {
                Some(Props::Slider {
                    min: old_min,
                    max: old_max,
                    ..
                }) => old_min != min || old_max != max,
                _ => true,
            };
            // SAFETY: `obj` is a live slider object.
            unsafe {
                if range_changed {
                    rs_lvgl_sys::lv_slider_set_range(obj.as_ptr(), *min, *max);
                }
                if rs_lvgl_sys::lv_slider_get_value(obj.as_ptr()) != *value {
                    suppress(slot, || {
                        rs_lvgl_sys::lv_slider_set_value(obj.as_ptr(), *value, false);
                    });
                }
            }
        }
        #[cfg(feature = "widget-bar")]
        (Props::Bar { min, max, value }, old) => {
            let range_changed = match old {
                Some(Props::Bar {
                    min: old_min,
                    max: old_max,
                    ..
                }) => old_min != min || old_max != max,
                _ => true,
            };
            // SAFETY: `obj` is a live bar object.
            unsafe {
                if range_changed {
                    rs_lvgl_sys::lv_bar_set_range(obj.as_ptr(), *min, *max);
                }
                if rs_lvgl_sys::lv_bar_get_value(obj.as_ptr()) != *value {
                    suppress(slot, || {
                        rs_lvgl_sys::lv_bar_set_value(obj.as_ptr(), *value, false);
                    });
                }
            }
        }
        #[cfg(feature = "widget-arc")]
        (Props::Arc { min, max, value }, old) => {
            let range_changed = match old {
                Some(Props::Arc {
                    min: old_min,
                    max: old_max,
                    ..
                }) => old_min != min || old_max != max,
                _ => true,
            };
            // SAFETY: `obj` is a live arc object.
            unsafe {
                if range_changed {
                    rs_lvgl_sys::lv_arc_set_range(obj.as_ptr(), *min, *max);
                }
                if rs_lvgl_sys::lv_arc_get_value(obj.as_ptr()) != *value {
                    suppress(slot, || {
                        rs_lvgl_sys::lv_arc_set_value(obj.as_ptr(), *value);
                    });
                }
            }
        }
        #[cfg(feature = "widget-spinbox")]
        (
            Props::Spinbox {
                min,
                max,
                value,
                digit_count,
                decimal_pos,
                step,
            },
            old,
        ) => {
            let (format_changed, range_changed, step_changed) = match old {
                Some(Props::Spinbox {
                    min: old_min,
                    max: old_max,
                    digit_count: old_digits,
                    decimal_pos: old_pos,
                    step: old_step,
                    ..
                }) => (
                    old_digits != digit_count || old_pos != decimal_pos,
                    old_min != min || old_max != max,
                    old_step != step,
                ),
                _ => (true, true, true),
            };
            // SAFETY: `obj` is a live spinbox. Each write re-renders the text
            // through the textarea base class, which emits a value event; the
            // write is suppressed so only user edits reach the handlers.
            unsafe {
                if format_changed {
                    suppress(slot, || {
                        rs_lvgl_sys::lv_spinbox_set_digit_format(
                            obj.as_ptr(),
                            *digit_count,
                            *decimal_pos,
                        );
                    });
                }
                if range_changed || format_changed {
                    // The digit format may have clamped the range.
                    suppress(slot, || {
                        rs_lvgl_sys::lv_spinbox_set_range(obj.as_ptr(), *min, *max);
                    });
                }
                if step_changed {
                    // Changing the step also moves the cursor and re-renders.
                    suppress(slot, || {
                        rs_lvgl_sys::lv_spinbox_set_step(obj.as_ptr(), *step);
                    });
                }
                if rs_lvgl_sys::lv_spinbox_get_value(obj.as_ptr()) != *value {
                    suppress(slot, || {
                        rs_lvgl_sys::lv_spinbox_set_value(obj.as_ptr(), *value);
                    });
                }
            }
        }
        _ => unreachable!("property dispatcher selected a different widget family"),
    }
    Ok(())
}
