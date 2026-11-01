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

pub(super) fn read_value(obj: *mut lv_obj_t, source: ValueSource, param: *mut cty::c_void) -> i32 {
    if obj.is_null() {
        return 0;
    }
    // SAFETY: the target object is the object this slot is registered on and is
    // valid for the duration of the event dispatch. The value getters accept
    // the concrete class created for `ValueSource`.
    unsafe {
        match source {
            ValueSource::None => 0,
            #[cfg(feature = "widget-slider")]
            ValueSource::Slider => rs_lvgl_sys::lv_slider_get_value(obj),
            #[cfg(feature = "widget-bar")]
            ValueSource::Bar => rs_lvgl_sys::lv_bar_get_value(obj),
            #[cfg(feature = "widget-switch")]
            ValueSource::Switch => {
                rs_lvgl_sys::lv_obj_has_state(obj, rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED) as i32
            }
            #[cfg(feature = "widget-checkbox")]
            ValueSource::Checkbox => {
                rs_lvgl_sys::lv_obj_has_state(obj, rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED) as i32
            }
            #[cfg(feature = "widget-dropdown")]
            ValueSource::Dropdown => rs_lvgl_sys::lv_dropdown_get_selected(obj) as i32,
            #[cfg(feature = "widget-arc")]
            ValueSource::Arc => rs_lvgl_sys::lv_arc_get_value(obj),
            #[cfg(feature = "widget-tabview")]
            ValueSource::Tabview => rs_lvgl_sys::lv_tabview_get_tab_active(obj) as i32,
            #[cfg(feature = "widget-roller")]
            ValueSource::Roller => rs_lvgl_sys::lv_roller_get_selected(obj) as i32,
            #[cfg(feature = "widget-spinbox")]
            ValueSource::Spinbox => rs_lvgl_sys::lv_spinbox_get_value(obj),
            #[cfg(feature = "widget-buttonmatrix")]
            ValueSource::ButtonMatrix => {
                let button = rs_lvgl_sys::lv_buttonmatrix_get_selected_button(obj);
                if button == rs_lvgl_sys::LV_BUTTONMATRIX_BUTTON_NONE {
                    -1
                } else {
                    button as i32
                }
            }
            #[cfg(feature = "widget-table")]
            ValueSource::Table => {
                let mut row = 0;
                let mut column = 0;
                rs_lvgl_sys::lv_table_get_selected_cell(obj, &mut row, &mut column);
                if row == rs_lvgl_sys::LV_TABLE_CELL_NONE
                    || column == rs_lvgl_sys::LV_TABLE_CELL_NONE
                {
                    -1
                } else {
                    ((row << 16) | column) as i32
                }
            }
            #[cfg(feature = "widget-tileview")]
            ValueSource::Tileview => {
                let tile = rs_lvgl_sys::lv_tileview_get_tile_active(obj);
                if tile.is_null() {
                    -1
                } else {
                    rs_lvgl_sys::lv_obj_get_index(tile)
                }
            }
            #[cfg(feature = "widget-calendar")]
            ValueSource::Calendar => {
                let mut date = rs_lvgl_sys::lv_calendar_date_t {
                    year: 0,
                    month: 0,
                    day: 0,
                };
                // Day-name cells and the adjacent months' days are disabled
                // buttons; LVGL would report them with the shown month.
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
                    && date.day != 0
                {
                    crate::node::CalendarDate::new(date.year, date.month, date.day).to_value()
                } else {
                    -1
                }
            }
            ValueSource::EventIndex => (param as usize as i32) - 1,
        }
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
