//! Host tests for the widgets added after the first release: roller,
//! spinbox, spinner, LED, line, button matrix and table.

mod common;

use core::ptr::{self, NonNull};
use rs_lvgl_ui::prelude::*;
use std::ffi::CStr;

use common::{app_root, child, init, lock};
use rs_lvgl_sys::lv_obj_t;

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Value(i32),
}

/// A view built from a function of a small state value.
struct TestApp {
    state: u32,
    view: fn(u32) -> View<Msg>,
    received: Vec<Msg>,
}

impl TestApp {
    fn new(view: fn(u32) -> View<Msg>) -> Self {
        Self {
            state: 0,
            view,
            received: Vec::new(),
        }
    }
}

impl App for TestApp {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) {
        self.received.push(msg);
    }

    fn view(&self, _: &Theme) -> View<Msg> {
        column([]).child((self.view)(self.state))
    }
}

fn mount(app: &TestApp) -> (Ui<Msg>, NonNull<lv_obj_t>) {
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    ui.mount(app).unwrap();
    let widget = child(app_root(), 0);
    (ui, widget)
}

fn value_changed(obj: NonNull<lv_obj_t>) {
    // SAFETY: tests only send events to live mounted objects.
    unsafe {
        rs_lvgl_sys::lv_obj_send_event(
            obj.as_ptr(),
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            ptr::null_mut(),
        )
    };
}

fn text(raw: *const cty::c_char) -> String {
    assert!(!raw.is_null());
    // SAFETY: LVGL returns NUL-terminated strings owned by the widget.
    unsafe { CStr::from_ptr(raw) }.to_str().unwrap().to_owned()
}

#[test]
fn roller_options_and_selection_are_controlled() {
    let _guard = lock();
    let mut app = TestApp::new(|state| {
        let options: &[&str] = if state < 2 {
            &["a", "b", "c"]
        } else {
            &["x", "y"]
        };
        roller(options.iter().copied(), state.min(1))
            .visible_rows(4)
            .roller_mode(RollerMode::Normal)
            .on_value_changed(Msg::Value)
    });
    let (mut ui, roller) = mount(&app);
    // SAFETY: `roller` is a live roller.
    unsafe {
        assert_eq!(rs_lvgl_sys::lv_roller_get_option_count(roller.as_ptr()), 3);
        assert_eq!(
            text(rs_lvgl_sys::lv_roller_get_options(roller.as_ptr())),
            "a\nb\nc"
        );
        assert_eq!(rs_lvgl_sys::lv_roller_get_selected(roller.as_ptr()), 0);
    }

    app.state = 1;
    ui.update(&app).unwrap();
    // SAFETY: as above.
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_roller_get_selected(roller.as_ptr()) },
        1
    );
    // Programmatic selection does not echo back as a message.
    ui.drain(&mut app, 8).unwrap();
    assert!(app.received.is_empty());

    // A user change the application does not accept is reverted.
    // SAFETY: as above.
    unsafe { rs_lvgl_sys::lv_roller_set_selected(roller.as_ptr(), 2, false) };
    value_changed(roller);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.received, vec![Msg::Value(2)]);
    // SAFETY: as above.
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_roller_get_selected(roller.as_ptr()) },
        1
    );

    app.state = 2;
    ui.update(&app).unwrap();
    assert_eq!(child(app_root(), 0), roller);
    // SAFETY: as above.
    unsafe {
        assert_eq!(
            text(rs_lvgl_sys::lv_roller_get_options(roller.as_ptr())),
            "x\ny"
        );
        assert_eq!(rs_lvgl_sys::lv_roller_get_selected(roller.as_ptr()), 1);
    }
}

#[test]
fn infinite_roller_reports_the_real_index() {
    let _guard = lock();
    let mut app = TestApp::new(|_| {
        roller(["a", "b", "c"], 2)
            .roller_mode(RollerMode::Infinite)
            .on_value_changed(Msg::Value)
    });
    let (mut ui, roller) = mount(&app);
    // SAFETY: `roller` is a live roller.
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_roller_get_selected(roller.as_ptr()) },
        2
    );
    value_changed(roller);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.received, vec![Msg::Value(2)]);
}

#[test]
fn spinbox_format_range_and_value_are_applied_without_echo() {
    let _guard = lock();
    let mut app = TestApp::new(|state| {
        spinbox(-500, 500, if state == 0 { 125 } else { -42 })
            .digits(3, 2)
            .step(10)
            .on_value_changed(Msg::Value)
    });
    let (mut ui, spinbox) = mount(&app);
    // SAFETY: `spinbox` is a live spinbox (a textarea subclass).
    unsafe {
        assert_eq!(rs_lvgl_sys::lv_spinbox_get_value(spinbox.as_ptr()), 125);
        assert_eq!(rs_lvgl_sys::lv_spinbox_get_step(spinbox.as_ptr()), 10);
        assert_eq!(
            text(rs_lvgl_sys::lv_textarea_get_text(spinbox.as_ptr())),
            "+12.5"
        );
    }
    ui.drain(&mut app, 8).unwrap();
    assert!(
        app.received.is_empty(),
        "programmatic writes are suppressed"
    );

    app.state = 1;
    ui.update(&app).unwrap();
    // SAFETY: as above.
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_spinbox_get_value(spinbox.as_ptr()) },
        -42
    );
    ui.drain(&mut app, 8).unwrap();
    assert!(app.received.is_empty());

    // A native increment reaches the handler and is reverted by the view.
    // SAFETY: as above.
    unsafe { rs_lvgl_sys::lv_spinbox_increment(spinbox.as_ptr()) };
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.received, vec![Msg::Value(-32)]);
    // SAFETY: as above.
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_spinbox_get_value(spinbox.as_ptr()) },
        -42
    );
}

#[test]
fn spinner_and_led_apply_their_parameters() {
    let _guard = lock();
    let mut app = TestApp::new(|state| {
        if state == 0 {
            row([]).child(spinner()).child(led(Color::rgb(255, 0, 0)))
        } else {
            row([])
                .child(spinner().spinner_anim(500, 90))
                .child(led(Color::rgb(0, 255, 0)).brightness(120))
        }
    });
    let (mut ui, row) = mount(&app);
    let spinner = child(row, 0);
    let led = child(row, 1);
    // SAFETY: both objects are live.
    unsafe {
        assert_eq!(
            rs_lvgl_sys::lv_spinner_get_anim_duration(spinner.as_ptr()),
            1000
        );
        assert_eq!(rs_lvgl_sys::lv_spinner_get_arc_sweep(spinner.as_ptr()), 200);
        assert_eq!(rs_lvgl_sys::lv_led_get_brightness(led.as_ptr()), 255);
        let color = rs_lvgl_sys::lv_led_get_color(led.as_ptr());
        assert_eq!((color.red, color.green, color.blue), (255, 0, 0));
    }
    app.state = 1;
    ui.update(&app).unwrap();
    // SAFETY: as above.
    unsafe {
        assert_eq!(
            rs_lvgl_sys::lv_spinner_get_anim_duration(spinner.as_ptr()),
            500
        );
        assert_eq!(rs_lvgl_sys::lv_spinner_get_arc_sweep(spinner.as_ptr()), 90);
        assert_eq!(rs_lvgl_sys::lv_led_get_brightness(led.as_ptr()), 120);
        let color = rs_lvgl_sys::lv_led_get_color(led.as_ptr());
        assert_eq!((color.red, color.green, color.blue), (0, 255, 0));
    }
}

fn line_points(line: NonNull<lv_obj_t>) -> Vec<(f64, f64)> {
    // SAFETY: `line` is live; LVGL returns its current point buffer.
    unsafe {
        let count = rs_lvgl_sys::lv_line_get_point_count(line.as_ptr()) as usize;
        let points = rs_lvgl_sys::lv_line_get_points(line.as_ptr());
        (0..count)
            .map(|index| {
                let point = *points.add(index);
                // Both integer and LV_USE_FLOAT coordinates convert exactly
                // to f64, preserving the value asserted by this test.
                (f64::from(point.x), f64::from(point.y))
            })
            .collect()
    }
}

#[test]
fn line_points_stay_owned_across_updates() {
    let _guard = lock();
    let mut app = TestApp::new(|state| match state {
        0 | 1 => line([(0, 0), (10, 5), (20, 0)]),
        _ => line([(1, 1), (2, 2)]),
    });
    let (mut ui, line) = mount(&app);
    assert_eq!(
        line_points(line),
        vec![(0.0, 0.0), (10.0, 5.0), (20.0, 0.0)]
    );
    // SAFETY: `line` is live.
    let buffer = unsafe { rs_lvgl_sys::lv_line_get_points(line.as_ptr()) };

    // An unchanged update keeps the retained buffer.
    app.state = 1;
    ui.update(&app).unwrap();
    // SAFETY: as above.
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_line_get_points(line.as_ptr()) },
        buffer
    );

    app.state = 2;
    ui.update(&app).unwrap();
    assert_eq!(line_points(line), vec![(1.0, 1.0), (2.0, 2.0)]);
    // Rendering exercises native reads from the retained buffer.
    // SAFETY: the display is registered.
    unsafe { rs_lvgl_sys::lv_refr_now(ptr::null_mut()) };
}

fn matrix_texts(matrix: NonNull<lv_obj_t>) -> Vec<String> {
    let mut texts = Vec::new();
    // SAFETY: `matrix` is live; the map is terminated by an empty string.
    unsafe {
        let map = rs_lvgl_sys::lv_buttonmatrix_get_map(matrix.as_ptr());
        let mut index = 0;
        loop {
            let entry = text(*map.add(index));
            if entry.is_empty() {
                break;
            }
            texts.push(entry);
            index += 1;
        }
    }
    texts
}

#[test]
fn button_matrix_owns_its_map_and_reports_the_pressed_button() {
    let _guard = lock();
    let mut app = TestApp::new(|state| {
        let rows: Vec<Vec<&str>> = if state == 0 {
            vec![vec!["1", "2", "3"], vec!["4", "5"]]
        } else {
            vec![vec!["OK", "Cancel"]]
        };
        button_matrix(rows).on_value_changed(Msg::Value)
    });
    let (mut ui, matrix) = mount(&app);
    assert_eq!(matrix_texts(matrix), ["1", "2", "3", "\n", "4", "5"]);

    // SAFETY: `matrix` is live; button 3 is "4".
    unsafe { rs_lvgl_sys::lv_buttonmatrix_set_selected_button(matrix.as_ptr(), 3) };
    value_changed(matrix);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.received, vec![Msg::Value(3)]);

    app.state = 1;
    ui.update(&app).unwrap();
    assert_eq!(child(app_root(), 0), matrix);
    assert_eq!(matrix_texts(matrix), ["OK", "Cancel"]);
    // Rendering reads the map strings (checked under ASan).
    // SAFETY: the display is registered.
    unsafe { rs_lvgl_sys::lv_refr_now(ptr::null_mut()) };
}

#[test]
fn table_cells_update_incrementally_and_report_the_selected_cell() {
    let _guard = lock();
    let mut app = TestApp::new(|state| {
        let mut cells = vec![vec!["name", "value"], vec!["a", "1"]];
        if state == 1 {
            cells[1][1] = "2";
        }
        if state == 2 {
            cells.push(vec!["b", "3"]);
        }
        table(cells)
            .column_widths([60, 40])
            .on_value_changed(Msg::Value)
    });
    let (mut ui, table) = mount(&app);
    // SAFETY: `table` is a live table.
    let untouched = unsafe {
        assert_eq!(rs_lvgl_sys::lv_table_get_row_count(table.as_ptr()), 2);
        assert_eq!(rs_lvgl_sys::lv_table_get_column_count(table.as_ptr()), 2);
        assert_eq!(
            rs_lvgl_sys::lv_table_get_column_width(table.as_ptr(), 0),
            60
        );
        assert_eq!(
            text(rs_lvgl_sys::lv_table_get_cell_value(table.as_ptr(), 1, 1)),
            "1"
        );
        rs_lvgl_sys::lv_table_get_cell_value(table.as_ptr(), 0, 0)
    };

    app.state = 1;
    ui.update(&app).unwrap();
    // SAFETY: as above.
    unsafe {
        assert_eq!(
            text(rs_lvgl_sys::lv_table_get_cell_value(table.as_ptr(), 1, 1)),
            "2"
        );
        // Unchanged cells are not rewritten.
        assert_eq!(
            rs_lvgl_sys::lv_table_get_cell_value(table.as_ptr(), 0, 0),
            untouched
        );
    }

    app.state = 2;
    ui.update(&app).unwrap();
    // SAFETY: as above.
    unsafe {
        assert_eq!(rs_lvgl_sys::lv_table_get_row_count(table.as_ptr()), 3);
        assert_eq!(
            text(rs_lvgl_sys::lv_table_get_cell_value(table.as_ptr(), 2, 0)),
            "b"
        );
        rs_lvgl_sys::lv_table_set_selected_cell(table.as_ptr(), 2, 1);
    }
    value_changed(table);
    ui.drain(&mut app, 8).unwrap();
    let Msg::Value(value) = app.received[0];
    assert_eq!(table_cell(value), Some((2, 1)));
    assert_eq!(table_cell(-1), None);
}

#[test]
fn invalid_widget_properties_are_rejected() {
    let _guard = lock();
    let cases: [fn(u32) -> View<Msg>; 12] = [
        |_| roller(Vec::<String>::new(), 0),
        |_| roller(["a"], 1),
        |_| roller(["a\nb"], 0),
        |_| spinbox(0, 10, 11),
        |_| spinbox(0, 1000, 0).digits(3, 0),
        |_| spinbox(0, 10, 0).digits(2, 2),
        |_| spinbox(0, 10, 0).step(0),
        |_| spinner().spinner_anim(1000, 361),
        |_| button_matrix([[""]]),
        |_| button_matrix(Vec::<Vec<String>>::new()),
        |_| table([vec!["a", "b"], vec!["c"]]),
        |_| table([["a"]]).column_widths([10, 20]),
    ];
    for (index, case) in cases.into_iter().enumerate() {
        let display = init();
        let mut ui = Ui::<Msg>::new(&display, Theme::dark()).unwrap();
        let app = TestApp::new(case);
        assert_eq!(
            ui.mount(&app),
            Err(UiError::InvalidProperty),
            "case {index} should be rejected"
        );
    }
}
