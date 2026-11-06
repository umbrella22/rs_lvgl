//! Semantic control payloads cross the real native callback and action queue.
#![cfg(feature = "widgets-all")]
mod common;
use common::{app_root, child, init, lock};
use rs_lvgl_ui::prelude::*;

#[derive(Clone, Debug, PartialEq)]
enum FieldAction {
    Toggle(bool),
    Selection(Option<u32>),
    Cell(Option<TableCell>),
    Date(Option<CalendarDate>),
    Legacy(i32),
}
#[derive(Clone, Debug, PartialEq)]
enum PageAction {
    Field(FieldAction),
}
struct Page {
    make: fn() -> View<FieldAction>,
    received: Vec<PageAction>,
}
impl App for Page {
    type Action = PageAction;
    fn update(&mut self, action: PageAction) {
        self.received.push(action);
    }
    fn view(&self, _: &Theme) -> View<PageAction> {
        (self.make)().map(PageAction::Field)
    }
}
fn send(obj: *mut rs_lvgl_sys::lv_obj_t) {
    // SAFETY: callers pass objects belonging to the current mounted tree.
    unsafe {
        rs_lvgl_sys::lv_obj_send_event(
            obj,
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            core::ptr::null_mut(),
        );
    }
}

#[test]
fn toggles_use_bool_and_replace_the_numeric_handler() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut page = Page {
        make: || {
            column([
                switch(false)
                    .on_value_changed(FieldAction::Legacy)
                    .on_toggle(FieldAction::Toggle),
                checkbox("checked", true).on_toggle(FieldAction::Toggle),
            ])
        },
        received: vec![],
    };
    ui.mount(&page).unwrap();
    let toggle = child(app_root(), 0).as_ptr();
    let check = child(app_root(), 1).as_ptr();
    // SAFETY: these objects are a live switch and checkbox.
    unsafe {
        rs_lvgl_sys::lv_obj_add_state(toggle, rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED);
        rs_lvgl_sys::lv_obj_remove_state(check, rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED);
    }
    send(toggle);
    send(check);
    assert!(page.received.is_empty(), "callbacks only queue actions");
    assert_eq!(ui.drain(&mut page, 8).unwrap().processed, 2);
    assert_eq!(
        page.received,
        [
            PageAction::Field(FieldAction::Toggle(true)),
            PageAction::Field(FieldAction::Toggle(false))
        ]
    );
    assert_eq!(
        ui.drain(&mut page, 8).unwrap().processed,
        0,
        "controlled correction produces no echo"
    );
}

#[test]
fn selectors_deliver_indices_and_valid_absence() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut page = Page {
        make: || {
            column([
                dropdown(["a", "b"], 0).on_selection_changed(FieldAction::Selection),
                roller(["a", "b", "c"], 0).on_selection_changed(FieldAction::Selection),
                button_matrix([["a", "b"]]).on_selection_changed(FieldAction::Selection),
                table([["a", "b"]]).on_cell_selected(FieldAction::Cell),
            ])
        },
        received: vec![],
    };
    ui.mount(&page).unwrap();
    let nodes: Vec<_> = (0..4)
        .map(|index| child(app_root(), index).as_ptr())
        .collect();
    // SAFETY: concrete controls were constructed in this order above.
    unsafe {
        rs_lvgl_sys::lv_dropdown_set_selected(nodes[0], 1);
        rs_lvgl_sys::lv_roller_set_selected(nodes[1], 2, false);
        rs_lvgl_sys::lv_buttonmatrix_set_selected_button(
            nodes[2],
            rs_lvgl_sys::LV_BUTTONMATRIX_BUTTON_NONE,
        );
    }
    for &node in &nodes[..3] {
        send(node);
    }
    // The table setter itself emits VALUE_CHANGED for a new selected cell.
    unsafe {
        rs_lvgl_sys::lv_table_set_selected_cell(nodes[3], 0, 1);
    }
    ui.drain(&mut page, 8).unwrap();
    assert_eq!(
        page.received,
        [
            PageAction::Field(FieldAction::Selection(Some(1))),
            PageAction::Field(FieldAction::Selection(Some(2))),
            PageAction::Field(FieldAction::Selection(None)),
            PageAction::Field(FieldAction::Cell(Some(TableCell { row: 0, column: 1 }))),
        ]
    );
}

#[test]
fn message_box_rejects_invalid_encoded_indices() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut page = Page {
        make: || {
            message_box("body")
                .message_box_buttons(["ok"])
                .on_selection_changed(FieldAction::Selection)
        },
        received: vec![],
    };
    ui.mount(&page).unwrap();
    let node = app_root().as_ptr();
    // SAFETY: VALUE_CHANGED uses an encoded integer, never a dereferenced pointer.
    unsafe {
        for index in [0usize, 1, 2, usize::MAX] {
            rs_lvgl_sys::lv_obj_send_event(
                node,
                rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
                index as *mut _,
            );
        }
    }
    ui.drain(&mut page, 8).unwrap();
    assert_eq!(
        page.received,
        [
            PageAction::Field(FieldAction::Selection(None)),
            PageAction::Field(FieldAction::Selection(Some(0)))
        ]
    );
}

#[test]
fn calendar_delivers_date_and_none_for_day_names() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut page = Page {
        make: || calendar(2026, 10).on_date_selected(FieldAction::Date),
        received: vec![],
    };
    ui.mount(&page).unwrap();
    // SAFETY: this is a live calendar and its owned button matrix.
    unsafe {
        let matrix = rs_lvgl_sys::lv_calendar_get_btnmatrix(app_root().as_ptr());
        let id = (7..49)
            .find(|&id| {
                let text = std::ffi::CStr::from_ptr(rs_lvgl_sys::lv_buttonmatrix_get_button_text(
                    matrix, id,
                ));
                text.to_bytes() == b"15"
                    && !rs_lvgl_sys::lv_buttonmatrix_has_button_ctrl(
                        matrix,
                        id,
                        rs_lvgl_sys::lv_buttonmatrix_ctrl_t_LV_BUTTONMATRIX_CTRL_DISABLED,
                    )
            })
            .unwrap();
        rs_lvgl_sys::lv_buttonmatrix_set_selected_button(matrix, id);
        send(matrix);
        rs_lvgl_sys::lv_buttonmatrix_set_selected_button(matrix, 0);
        send(matrix);
    }
    ui.drain(&mut page, 8).unwrap();
    assert_eq!(
        page.received,
        [
            PageAction::Field(FieldAction::Date(Some(CalendarDate::new(2026, 10, 15)))),
            PageAction::Field(FieldAction::Date(None))
        ]
    );
}
