//! Property updates for data widgets.

use super::super::Props;
use super::super::Retained;
use crate::node::CalendarDate;
use crate::UiError;
use alloc::vec::Vec;
use core::ptr::NonNull;
use cstr_core::CString;
use rs_lvgl_sys::lv_obj_t;

pub(super) fn apply(
    obj: NonNull<lv_obj_t>,
    new: &Props,
    old: Option<&Props>,
    retained: &mut Retained,
) -> Result<(), UiError> {
    match (new, old) {
        #[cfg(feature = "widget-scale")]
        (
            Props::Scale {
                min,
                max,
                total_ticks,
                major_tick_every,
            },
            old,
        ) => {
            let (range_changed, ticks_changed, major_changed) = match old {
                Some(Props::Scale {
                    min: old_min,
                    max: old_max,
                    total_ticks: old_ticks,
                    major_tick_every: old_major,
                }) => (
                    old_min != min || old_max != max,
                    old_ticks != total_ticks,
                    old_major != major_tick_every,
                ),
                _ => (true, true, true),
            };
            // SAFETY: `obj` is a live scale object.
            unsafe {
                if range_changed {
                    rs_lvgl_sys::lv_scale_set_range(obj.as_ptr(), *min, *max);
                }
                if ticks_changed {
                    rs_lvgl_sys::lv_scale_set_total_tick_count(obj.as_ptr(), *total_ticks);
                }
                if major_changed {
                    rs_lvgl_sys::lv_scale_set_major_tick_every(obj.as_ptr(), *major_tick_every);
                }
            }
        }
        #[cfg(feature = "widget-table")]
        (
            Props::Table {
                cells,
                column_widths,
            },
            old,
        ) => {
            let rows = cells.len() as u32;
            let columns = cells.first().map(Vec::len).unwrap_or(0) as u32;
            let (old_cells, old_widths) = match old {
                Some(Props::Table {
                    cells,
                    column_widths,
                }) => (Some(cells), Some(column_widths)),
                _ => (None, None),
            };
            let same_shape = old_cells
                .map(|old| {
                    old.len() == cells.len()
                        && old.first().map(Vec::len) == cells.first().map(Vec::len)
                })
                .unwrap_or(false);
            // SAFETY: `obj` is a live table; LVGL copies every cell text.
            unsafe {
                if !same_shape {
                    rs_lvgl_sys::lv_table_set_column_count(obj.as_ptr(), columns);
                    rs_lvgl_sys::lv_table_set_row_count(obj.as_ptr(), rows);
                }
                for (row, values) in cells.iter().enumerate() {
                    for (column, text) in values.iter().enumerate() {
                        let unchanged = same_shape
                            && old_cells
                                .map(|old| old[row][column] == *text)
                                .unwrap_or(false);
                        if unchanged {
                            continue;
                        }
                        let text =
                            CString::new(text.as_str()).map_err(|_| UiError::InvalidProperty)?;
                        rs_lvgl_sys::lv_table_set_cell_value(
                            obj.as_ptr(),
                            row as u32,
                            column as u32,
                            text.as_ptr(),
                        );
                    }
                }
                if !same_shape || old_widths != Some(column_widths) {
                    for (column, width) in column_widths.iter().enumerate() {
                        rs_lvgl_sys::lv_table_set_column_width(obj.as_ptr(), column as u32, *width);
                    }
                }
            }
        }
        #[cfg(feature = "widget-calendar")]
        (
            Props::Calendar {
                year,
                month,
                today,
                highlighted,
            },
            old,
        ) => {
            let (old_shown, old_today, old_highlighted) = match old {
                Some(Props::Calendar {
                    year,
                    month,
                    today,
                    highlighted,
                }) => (
                    Some((*year, *month)),
                    Some(*today),
                    Some(highlighted.as_slice()),
                ),
                _ => (None, None, None),
            };
            // SAFETY: `obj` is a live calendar. Changing the shown month only
            // rewrites the calendar's own button matrix map; it emits no
            // value event to the calendar.
            unsafe {
                if old_shown != Some((*year, *month)) {
                    rs_lvgl_sys::lv_calendar_set_month_shown(
                        obj.as_ptr(),
                        u32::from(*year),
                        u32::from(*month),
                    );
                }
                if old_today != Some(*today) {
                    // A zero date matches no day, which clears LVGL's default
                    // "today" marker.
                    let date = today.unwrap_or(CalendarDate::new(0, 0, 0));
                    rs_lvgl_sys::lv_calendar_set_today_date(
                        obj.as_ptr(),
                        u32::from(date.year),
                        u32::from(date.month),
                        u32::from(date.day),
                    );
                }
            }
            let changed = old_highlighted != Some(highlighted.as_slice())
                || !matches!(retained, Retained::Dates(_));
            if changed {
                let mut dates: Vec<rs_lvgl_sys::lv_calendar_date_t> =
                    highlighted.iter().map(|date| date.to_raw()).collect();
                let pointer = if dates.is_empty() {
                    core::ptr::null_mut()
                } else {
                    dates.as_mut_ptr()
                };
                // SAFETY: `obj` is a live calendar. LVGL stores the pointer;
                // the buffer moves into `retained` without changing address
                // and the previous buffer is dropped only afterwards.
                unsafe {
                    rs_lvgl_sys::lv_calendar_set_highlighted_dates(
                        obj.as_ptr(),
                        pointer,
                        dates.len(),
                    )
                };
                *retained = Retained::Dates(dates);
            }
        }
        #[cfg(feature = "widget-spinner")]
        (
            Props::Spinner {
                duration_ms,
                arc_angle,
            },
            old,
        ) => {
            let changed = match old {
                Some(Props::Spinner {
                    duration_ms: old_duration,
                    arc_angle: old_angle,
                }) => old_duration != duration_ms || old_angle != arc_angle,
                _ => true,
            };
            if changed {
                // SAFETY: `obj` is a live spinner; LVGL restarts its own
                // animation, which is deleted with the object.
                unsafe {
                    rs_lvgl_sys::lv_spinner_set_anim_params(obj.as_ptr(), *duration_ms, *arc_angle)
                };
            }
        }
        #[cfg(feature = "widget-led")]
        (Props::Led { color, brightness }, old) => {
            let (old_color, old_brightness) = match old {
                Some(Props::Led { color, brightness }) => (Some(*color), Some(*brightness)),
                _ => (None, None),
            };
            // SAFETY: `obj` is a live LED.
            unsafe {
                if old_color != Some(*color) {
                    rs_lvgl_sys::lv_led_set_color(obj.as_ptr(), color.to_raw());
                }
                if old_brightness != Some(*brightness) {
                    rs_lvgl_sys::lv_led_set_brightness(obj.as_ptr(), *brightness);
                }
            }
        }
        _ => unreachable!("property dispatcher selected a different widget family"),
    }
    Ok(())
}
