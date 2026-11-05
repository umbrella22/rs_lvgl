//! Read widget values and borrow native text during a callback.

use rs_lvgl_sys::lv_obj_t;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ValueSource {
    None,
    #[cfg(feature = "widget-slider")]
    Slider,
    #[cfg(feature = "widget-bar")]
    Bar,
    #[cfg(feature = "widget-switch")]
    Switch,
    #[cfg(feature = "widget-checkbox")]
    Checkbox,
    #[cfg(feature = "widget-dropdown")]
    Dropdown,
    #[cfg(feature = "widget-arc")]
    Arc,
    #[cfg(feature = "widget-tabview")]
    Tabview,
    #[cfg(feature = "widget-roller")]
    Roller,
    #[cfg(feature = "widget-spinbox")]
    Spinbox,
    #[cfg(feature = "widget-buttonmatrix")]
    ButtonMatrix,
    #[cfg(feature = "widget-table")]
    Table,
    #[cfg(feature = "widget-tileview")]
    Tileview,
    #[cfg(feature = "widget-calendar")]
    Calendar,
    /// An index carried in the event parameter as `index + 1` (message box
    /// footer buttons); `-1` when the parameter is null.
    EventIndex,
}

/// Semantic payload copied while the native target is live. Invalid native
/// representations return no payload; valid absence is represented by Option.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ControlValue {
    Number(i32),
    Toggle(bool),
    Selection(Option<u32>),
    Cell(Option<crate::TableCell>),
    Date(Option<crate::CalendarDate>),
}

impl ControlValue {
    pub(crate) fn legacy(self) -> Option<i32> {
        Some(match self {
            Self::Number(value) => value,
            Self::Toggle(value) => i32::from(value),
            Self::Selection(Some(index)) => i32::try_from(index).ok()?,
            Self::Selection(None) | Self::Cell(None) | Self::Date(None) => -1,
            // Preserve all 32 packed bits, including rows above i16::MAX.
            Self::Cell(Some(cell)) => ((cell.row << 16) | cell.column) as i32,
            Self::Date(Some(date)) => date.to_value(),
        })
    }
}

fn selection(index: u32, count: u32) -> Option<ControlValue> {
    if count == 0 && index == 0 {
        Some(ControlValue::Selection(None))
    } else {
        (index < count).then_some(ControlValue::Selection(Some(index)))
    }
}

fn event_index(param: *mut cty::c_void) -> Option<ControlValue> {
    let encoded = param as usize;
    if encoded == 0 {
        Some(ControlValue::Selection(None))
    } else {
        let index = u32::try_from(encoded - 1).ok()?;
        Some(ControlValue::Selection(Some(index)))
    }
}

pub(super) fn read_value(
    obj: *mut lv_obj_t,
    source: ValueSource,
    param: *mut cty::c_void,
) -> Option<ControlValue> {
    if obj.is_null() {
        return None;
    }
    // SAFETY: dispatch supplies the slot's live target and the matching native
    // class. These synchronous getters do not retain pointers or borrow outputs.
    unsafe {
        Some(match source {
            ValueSource::None => ControlValue::Number(0),
            #[cfg(feature = "widget-slider")]
            ValueSource::Slider => ControlValue::Number(rs_lvgl_sys::lv_slider_get_value(obj)),
            #[cfg(feature = "widget-bar")]
            ValueSource::Bar => ControlValue::Number(rs_lvgl_sys::lv_bar_get_value(obj)),
            #[cfg(feature = "widget-switch")]
            ValueSource::Switch => ControlValue::Toggle(rs_lvgl_sys::lv_obj_has_state(
                obj,
                rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED,
            )),
            #[cfg(feature = "widget-checkbox")]
            ValueSource::Checkbox => ControlValue::Toggle(rs_lvgl_sys::lv_obj_has_state(
                obj,
                rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED,
            )),
            #[cfg(feature = "widget-dropdown")]
            ValueSource::Dropdown => {
                return selection(
                    rs_lvgl_sys::lv_dropdown_get_selected(obj),
                    rs_lvgl_sys::lv_dropdown_get_option_count(obj),
                )
            }
            #[cfg(feature = "widget-arc")]
            ValueSource::Arc => ControlValue::Number(rs_lvgl_sys::lv_arc_get_value(obj)),
            #[cfg(feature = "widget-tabview")]
            ValueSource::Tabview => {
                return selection(
                    rs_lvgl_sys::lv_tabview_get_tab_active(obj),
                    rs_lvgl_sys::lv_tabview_get_tab_count(obj),
                )
            }
            #[cfg(feature = "widget-roller")]
            ValueSource::Roller => {
                return selection(
                    rs_lvgl_sys::lv_roller_get_selected(obj),
                    rs_lvgl_sys::lv_roller_get_option_count(obj),
                )
            }
            #[cfg(feature = "widget-spinbox")]
            ValueSource::Spinbox => ControlValue::Number(rs_lvgl_sys::lv_spinbox_get_value(obj)),
            #[cfg(feature = "widget-buttonmatrix")]
            ValueSource::ButtonMatrix => {
                let index = rs_lvgl_sys::lv_buttonmatrix_get_selected_button(obj);
                ControlValue::Selection(
                    (index != rs_lvgl_sys::LV_BUTTONMATRIX_BUTTON_NONE).then_some(index),
                )
            }
            #[cfg(feature = "widget-table")]
            ValueSource::Table => {
                let (mut row, mut column) = (0, 0);
                rs_lvgl_sys::lv_table_get_selected_cell(obj, &mut row, &mut column);
                let cell = if row == rs_lvgl_sys::LV_TABLE_CELL_NONE
                    || column == rs_lvgl_sys::LV_TABLE_CELL_NONE
                {
                    None
                } else if row < rs_lvgl_sys::lv_table_get_row_count(obj)
                    && column < rs_lvgl_sys::lv_table_get_column_count(obj)
                {
                    Some(crate::TableCell { row, column })
                } else {
                    return None;
                };
                ControlValue::Cell(cell)
            }
            #[cfg(feature = "widget-tileview")]
            ValueSource::Tileview => {
                let tile = rs_lvgl_sys::lv_tileview_get_tile_active(obj);
                let index = if tile.is_null() {
                    None
                } else {
                    Some(u32::try_from(rs_lvgl_sys::lv_obj_get_index(tile)).ok()?)
                };
                ControlValue::Selection(index)
            }
            #[cfg(feature = "widget-calendar")]
            ValueSource::Calendar => {
                let mut date = rs_lvgl_sys::lv_calendar_date_t {
                    year: 0,
                    month: 0,
                    day: 0,
                };
                let matrix = rs_lvgl_sys::lv_calendar_get_btnmatrix(obj);
                let selected = rs_lvgl_sys::lv_buttonmatrix_get_selected_button(matrix);
                let enabled = selected != rs_lvgl_sys::LV_BUTTONMATRIX_BUTTON_NONE
                    && !rs_lvgl_sys::lv_buttonmatrix_has_button_ctrl(
                        matrix,
                        selected,
                        rs_lvgl_sys::lv_buttonmatrix_ctrl_t_LV_BUTTONMATRIX_CTRL_DISABLED,
                    );
                if enabled
                    && rs_lvgl_sys::lv_calendar_get_pressed_date(obj, &mut date)
                        == rs_lvgl_sys::lv_result_t_LV_RESULT_OK
                {
                    let date = crate::CalendarDate::new(date.year, date.month, date.day);
                    if !date.is_valid() {
                        return None;
                    }
                    ControlValue::Date(Some(date))
                } else {
                    ControlValue::Date(None)
                }
            }
            ValueSource::EventIndex => {
                let value = event_index(param)?;
                #[cfg(feature = "widget-msgbox")]
                if let ControlValue::Selection(Some(index)) = value {
                    let footer = rs_lvgl_sys::lv_msgbox_get_footer(obj);
                    if footer.is_null() || index >= rs_lvgl_sys::lv_obj_get_child_count(footer) {
                        return None;
                    }
                }
                value
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absence_and_invalid_indices_are_distinct() {
        assert_eq!(selection(0, 0), Some(ControlValue::Selection(None)));
        assert_eq!(selection(0, 1), Some(ControlValue::Selection(Some(0))));
        assert_eq!(selection(2, 2), None);
        assert_eq!(
            selection(u32::MAX - 1, u32::MAX),
            Some(ControlValue::Selection(Some(u32::MAX - 1)))
        );
        assert_eq!(ControlValue::Selection(Some(u32::MAX - 1)).legacy(), None);
        assert_eq!(
            event_index(core::ptr::null_mut()),
            Some(ControlValue::Selection(None))
        );
        assert_eq!(
            event_index(1usize as *mut _),
            Some(ControlValue::Selection(Some(0)))
        );
        #[cfg(target_pointer_width = "64")]
        assert_eq!(event_index((u32::MAX as usize + 2) as *mut _), None);
    }

    #[test]
    fn legacy_table_packing_preserves_high_rows() {
        for (row, column) in [(0, 0), (32768, 1), (65534, 65534)] {
            let packed = ControlValue::Cell(Some(crate::TableCell { row, column }))
                .legacy()
                .unwrap();
            assert_eq!(crate::table_cell(packed), Some((row, column)));
        }
        assert_eq!(crate::table_cell(-1), None);
        assert_eq!(crate::table_cell(0x0001_FFFF), None);
    }
}

/// Borrow the textarea's current text for one synchronous callback.
///
/// # Safety
///
/// `obj` must be a live textarea object. The borrowed text is only passed to
/// `f` and is not returned, so it cannot outlive LVGL's buffer.
#[cfg(feature = "widget-textarea")]
pub(super) unsafe fn with_text<R>(obj: *mut lv_obj_t, f: impl FnOnce(&str) -> R) -> Option<R> {
    if obj.is_null() {
        return None;
    }
    let ptr = unsafe { rs_lvgl_sys::lv_textarea_get_text(obj) };
    if ptr.is_null() {
        return None;
    }
    // SAFETY: the textarea returns a NUL-terminated string owned by LVGL and
    // valid until its text changes; `f` runs before this function returns.
    let text = unsafe { cstr_core::CStr::from_ptr(ptr) }.to_str().ok()?;
    Some(f(text))
}
