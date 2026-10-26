//! Host tests for the second widget batch: tileview, message box, calendar,
//! span group, image button, animated image and arc label.

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

fn send(obj: *mut lv_obj_t, code: rs_lvgl_sys::lv_event_code_t) {
    // SAFETY: tests only send events to live mounted objects.
    unsafe { rs_lvgl_sys::lv_obj_send_event(obj, code, ptr::null_mut()) };
}

fn value_changed(obj: *mut lv_obj_t) {
    send(obj, rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED);
}

fn text(raw: *const cty::c_char) -> String {
    assert!(!raw.is_null());
    // SAFETY: LVGL returns NUL-terminated strings owned by the widget.
    unsafe { CStr::from_ptr(raw) }.to_str().unwrap().to_owned()
}

fn label_text(label: *mut lv_obj_t) -> String {
    assert!(!label.is_null());
    // SAFETY: tests pass live labels.
    text(unsafe { rs_lvgl_sys::lv_label_get_text(label) })
}

fn tiles() -> [Tile; 3] {
    [
        Tile::new(0, 0, ScrollDir::RIGHT),
        Tile::new(1, 0, ScrollDir::HOR),
        Tile::new(2, 0, ScrollDir::LEFT),
    ]
}

#[test]
fn tileview_active_tile_is_controlled_and_reported() {
    let _guard = lock();
    let mut app = TestApp::new(|state| {
        let active = state.min(2);
        let mut tiles = tiles().to_vec();
        if state == 9 {
            tiles[2] = Tile::new(0, 1, ScrollDir::TOP);
        }
        tileview(tiles, active)
            .child(label("a"))
            .child(label("b"))
            .child(label("c"))
            .on_value_changed(Msg::Value)
    });
    app.state = 1;
    let (mut ui, tileview) = mount(&app);
    let tile = |index| child(tileview, index).as_ptr();
    // SAFETY: the tileview is live for the whole test.
    let active = || unsafe { rs_lvgl_sys::lv_tileview_get_tile_active(tileview.as_ptr()) };
    assert_eq!(active(), tile(1));
    assert_eq!(label_text(child(child(tileview, 2), 0).as_ptr()), "c");

    // A controlled write does not report a change.
    app.state = 2;
    ui.update(&app).unwrap();
    assert_eq!(active(), tile(2));
    ui.drain(&mut app, 8).unwrap();
    assert!(app.received.is_empty());

    // A user swipe ends with a value change carrying the tile index; a
    // native tile change outside the reconciler emits the same event.
    // SAFETY: the tile belongs to the tileview.
    unsafe { rs_lvgl_sys::lv_tileview_set_tile(tileview.as_ptr(), tile(0), false) };
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.received, vec![Msg::Value(0)]);

    // The application state wins on the next update.
    ui.update(&app).unwrap();
    assert_eq!(active(), tile(2));

    // Changing the tile layout recreates the tileview.
    let created = ui.stats().created_nodes;
    app.state = 9;
    ui.update(&app).unwrap();
    assert!(ui.stats().created_nodes > created);
}

#[test]
fn msgbox_places_title_text_children_and_reports_footer_clicks() {
    let _guard = lock();
    let mut app = TestApp::new(|state| {
        let mut node = msgbox(if state == 0 { "hello" } else { "bye" })
            .msgbox_title(if state == 0 { "Title" } else { "Other" })
            .msgbox_buttons(if state == 0 {
                ["OK", "Cancel"]
            } else {
                ["Yes", "No"]
            })
            .child(label("extra"))
            .on_value_changed(Msg::Value);
        if state == 2 {
            node = node.msgbox_buttons(["Only"]);
        }
        node
    });
    let (mut ui, msgbox) = mount(&app);
    // SAFETY: the message box is live for the whole test.
    let (content, footer) = unsafe {
        (
            rs_lvgl_sys::lv_msgbox_get_content(msgbox.as_ptr()),
            rs_lvgl_sys::lv_msgbox_get_footer(msgbox.as_ptr()),
        )
    };
    assert!(!content.is_null() && !footer.is_null());
    let content = NonNull::new(content).unwrap();
    assert_eq!(label_text(child(content, 0).as_ptr()), "hello");
    assert_eq!(label_text(child(content, 1).as_ptr()), "extra");
    // SAFETY: the message box has a title.
    assert_eq!(
        label_text(unsafe { rs_lvgl_sys::lv_msgbox_get_title(msgbox.as_ptr()) }),
        "Title"
    );
    let footer = NonNull::new(footer).unwrap();
    // SAFETY: the footer is live.
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_obj_get_child_count(footer.as_ptr()) },
        2
    );
    assert_eq!(label_text(child(child(footer, 1), 0).as_ptr()), "Cancel");

    send(
        child(footer, 1).as_ptr(),
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED,
    );
    send(
        child(footer, 0).as_ptr(),
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED,
    );
    // A value change without a footer parameter reports -1.
    value_changed(msgbox.as_ptr());
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(
        app.received,
        vec![Msg::Value(1), Msg::Value(0), Msg::Value(-1)]
    );

    // Texts update in place.
    let created = ui.stats().created_nodes;
    app.state = 1;
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, created);
    assert_eq!(label_text(child(content, 0).as_ptr()), "bye");
    assert_eq!(label_text(child(child(footer, 0), 0).as_ptr()), "Yes");
    // SAFETY: the message box has a title.
    assert_eq!(
        label_text(unsafe { rs_lvgl_sys::lv_msgbox_get_title(msgbox.as_ptr()) }),
        "Other"
    );

    // A different button count recreates the message box.
    app.state = 2;
    ui.update(&app).unwrap();
    assert!(ui.stats().created_nodes > created);
    let msgbox = child(app_root(), 0);
    // SAFETY: the new message box is live.
    let footer = unsafe { rs_lvgl_sys::lv_msgbox_get_footer(msgbox.as_ptr()) };
    // SAFETY: the footer is live.
    assert_eq!(unsafe { rs_lvgl_sys::lv_obj_get_child_count(footer) }, 1);
}

#[test]
fn calendar_shows_month_marks_dates_and_reports_pressed_date() {
    let _guard = lock();
    let mut app = TestApp::new(|state| {
        let mut node = calendar(2026, if state == 0 { 10 } else { 11 })
            .highlighted([
                CalendarDate::new(2026, 10, 3),
                CalendarDate::new(2026, 10, 9),
            ])
            .on_value_changed(Msg::Value);
        if state == 0 {
            node = node.today(CalendarDate::new(2026, 10, 21));
        }
        node
    });
    let (mut ui, calendar) = mount(&app);
    let raw = calendar.as_ptr();
    // SAFETY: the calendar is live for the whole test.
    unsafe {
        let shown = &*rs_lvgl_sys::lv_calendar_get_showed_date(raw);
        assert_eq!((shown.year, shown.month), (2026, 10));
        let today = &*rs_lvgl_sys::lv_calendar_get_today_date(raw);
        assert_eq!((today.year, today.month, today.day), (2026, 10, 21));
        assert_eq!(rs_lvgl_sys::lv_calendar_get_highlighted_dates_num(raw), 2);
        assert_eq!(
            (*rs_lvgl_sys::lv_calendar_get_highlighted_dates(raw)).day,
            3
        );
    }

    // Press "15" of the shown month on the inner button matrix; its events
    // bubble to the calendar node.
    // SAFETY: the calendar and its button matrix are live.
    let matrix = unsafe { rs_lvgl_sys::lv_calendar_get_btnmatrix(raw) };
    let find = |label: &str| {
        (7..49)
            .find(|&id| {
                // SAFETY: ids index the calendar's 7x7 button map.
                unsafe {
                    !rs_lvgl_sys::lv_buttonmatrix_has_button_ctrl(
                        matrix,
                        id,
                        rs_lvgl_sys::lv_buttonmatrix_ctrl_t_LV_BUTTONMATRIX_CTRL_DISABLED,
                    ) && text(rs_lvgl_sys::lv_buttonmatrix_get_button_text(matrix, id)) == label
                }
            })
            .unwrap()
    };
    // SAFETY: the matrix is live and the ids are in range.
    unsafe { rs_lvgl_sys::lv_buttonmatrix_set_selected_button(matrix, find("15")) };
    value_changed(matrix);
    // A day-name cell is not a date.
    // SAFETY: as above.
    unsafe { rs_lvgl_sys::lv_buttonmatrix_set_selected_button(matrix, 0) };
    value_changed(matrix);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.received, vec![Msg::Value(20261015), Msg::Value(-1)]);
    assert_eq!(
        CalendarDate::from_value(20261015),
        Some(CalendarDate::new(2026, 10, 15))
    );
    assert_eq!(CalendarDate::from_value(-1), None);

    // Month and today update in place; removing today clears the marker.
    let created = ui.stats().created_nodes;
    app.state = 1;
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, created);
    // SAFETY: the calendar is live.
    unsafe {
        let shown = &*rs_lvgl_sys::lv_calendar_get_showed_date(raw);
        assert_eq!(shown.month, 11);
        assert_eq!((*rs_lvgl_sys::lv_calendar_get_today_date(raw)).day, 0);
        assert_eq!(rs_lvgl_sys::lv_calendar_get_highlighted_dates_num(raw), 2);
    }
}

#[test]
fn spans_reuse_native_spans_and_restore_styles() {
    let _guard = lock();
    let mut app = TestApp::new(|state| match state {
        0 => spans([Span::new("a").color(Color::rgb(255, 0, 0)), Span::new("b")]),
        1 => spans([Span::new("x"), Span::new("b")]),
        _ => spans([Span::new("one")]),
    });
    let (mut ui, group) = mount(&app);
    let raw = group.as_ptr();
    // SAFETY: the span group and its spans are live while inspected.
    let span = |index| unsafe { rs_lvgl_sys::lv_spangroup_get_child(raw, index) };
    let has_color = |index| {
        // SAFETY: the style value is a plain C union.
        let mut value: rs_lvgl_sys::lv_style_value_t = unsafe { core::mem::zeroed() };
        // SAFETY: the span is live.
        unsafe {
            rs_lvgl_sys::lv_style_get_prop(
                rs_lvgl_sys::lv_span_get_style(span(index)),
                rs_lvgl_sys::_lv_style_id_t_LV_STYLE_TEXT_COLOR as rs_lvgl_sys::lv_style_prop_t,
                &mut value,
            ) == rs_lvgl_sys::lv_style_res_t_LV_STYLE_RES_FOUND
        }
    };
    // SAFETY: the span group is live.
    assert_eq!(unsafe { rs_lvgl_sys::lv_spangroup_get_span_count(raw) }, 2);
    // SAFETY: the span is live.
    assert_eq!(text(unsafe { rs_lvgl_sys::lv_span_get_text(span(0)) }), "a");
    assert!(has_color(0));
    assert!(!has_color(1));

    let first = span(0);
    app.state = 1;
    ui.update(&app).unwrap();
    assert_eq!(span(0), first);
    // SAFETY: the span is live.
    assert_eq!(text(unsafe { rs_lvgl_sys::lv_span_get_text(span(0)) }), "x");
    assert!(!has_color(0));

    app.state = 2;
    ui.update(&app).unwrap();
    // SAFETY: the span group is live.
    assert_eq!(unsafe { rs_lvgl_sys::lv_spangroup_get_span_count(raw) }, 1);
    // SAFETY: the span is live.
    assert_eq!(
        text(unsafe { rs_lvgl_sys::lv_span_get_text(span(0)) }),
        "one"
    );
}

static PIXELS_A: [u8; 8] = [0x00, 0xf8, 0xe0, 0x07, 0x1f, 0x00, 0xff, 0xff];
static PIXELS_B: [u8; 8] = [0xff, 0xff, 0x00, 0x00, 0x1f, 0x00, 0xe0, 0x07];
static PIXELS_C: [u8; 8] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];

#[test]
fn image_button_keeps_generated_descriptors_alive() {
    let _guard = lock();
    let mut app = TestApp::new(|state| {
        let node = image_button(ImageSource::rgb565(2, 2, &PIXELS_A));
        if state == 0 {
            node.pressed_image(ImageSource::rgb565(2, 2, &PIXELS_B))
        } else {
            node
        }
    });
    let (mut ui, button) = mount(&app);
    let raw = button.as_ptr();
    let src = |state| {
        // SAFETY: the image button is live.
        unsafe { rs_lvgl_sys::lv_imagebutton_get_src_middle(raw, state) }
    };
    let released = rs_lvgl_sys::lv_imagebutton_state_t_LV_IMAGEBUTTON_STATE_RELEASED;
    let pressed = rs_lvgl_sys::lv_imagebutton_state_t_LV_IMAGEBUTTON_STATE_PRESSED;
    assert!(!src(released).is_null());
    assert!(!src(pressed).is_null());
    assert_ne!(src(released), src(pressed));
    // SAFETY: the stored pointer is a descriptor retained by the tree.
    unsafe {
        let descriptor = &*(src(pressed) as *const rs_lvgl_sys::lv_image_dsc_t);
        assert_eq!(descriptor.data, PIXELS_B.as_ptr());
        // The content size comes from the released image header.
        rs_lvgl_sys::lv_obj_update_layout(raw);
        assert_eq!(rs_lvgl_sys::lv_obj_get_width(raw), 2);
    }

    app.state = 1;
    ui.update(&app).unwrap();
    assert!(src(pressed).is_null());
    // SAFETY: as above.
    let descriptor = unsafe { &*(src(released) as *const rs_lvgl_sys::lv_image_dsc_t) };
    assert_eq!(descriptor.data, PIXELS_A.as_ptr());
    // SAFETY: drawing reads the retained descriptors.
    unsafe {
        rs_lvgl_sys::lv_obj_invalidate(raw);
        rs_lvgl_sys::lv_refr_now(ptr::null_mut());
    }
}

#[test]
fn anim_image_swaps_frames_without_dangling_pointers() {
    let _guard = lock();
    let mut app = TestApp::new(|state| {
        if state == 0 {
            anim_image(
                [
                    ImageSource::rgb565(2, 2, &PIXELS_A),
                    ImageSource::rgb565(2, 2, &PIXELS_B),
                ],
                100,
            )
        } else {
            anim_image([ImageSource::rgb565(2, 2, &PIXELS_C)], 50).repeat(3)
        }
    });
    let (mut ui, image) = mount(&app);
    let raw = image.as_ptr();
    let advance = || {
        for _ in 0..4 {
            // SAFETY: LVGL is initialized; timers only touch live objects.
            unsafe {
                rs_lvgl_sys::lv_tick_inc(30);
                rs_lvgl_sys::lv_timer_handler();
            }
        }
    };
    // SAFETY: the animated image is live.
    unsafe {
        assert_eq!(rs_lvgl_sys::lv_animimg_get_src_count(raw), 2);
        assert_eq!(rs_lvgl_sys::lv_animimg_get_duration(raw), 100);
        assert_eq!(
            rs_lvgl_sys::lv_animimg_get_repeat_count(raw),
            rs_lvgl_ui::ANIM_REPEAT_INFINITE
        );
    }
    advance();

    app.state = 1;
    ui.update(&app).unwrap();
    // SAFETY: the animated image is live; its source is a retained frame.
    unsafe {
        assert_eq!(rs_lvgl_sys::lv_animimg_get_src_count(raw), 1);
        assert_eq!(rs_lvgl_sys::lv_animimg_get_duration(raw), 50);
        assert_eq!(rs_lvgl_sys::lv_animimg_get_repeat_count(raw), 3);
        let current = &*(rs_lvgl_sys::lv_image_get_src(raw) as *const rs_lvgl_sys::lv_image_dsc_t);
        assert_eq!(current.data, PIXELS_C.as_ptr());
    }
    // Running the animation after the swap only reads the new frames.
    advance();
    drop(ui);
}

#[test]
fn arc_label_updates_geometry_in_place() {
    let _guard = lock();
    let mut app = TestApp::new(|state| {
        if state == 0 {
            arc_label("round", 20, 180, 90)
        } else {
            arc_label("again", 25, 90, 180)
        }
    });
    let (mut ui, label) = mount(&app);
    let raw = label.as_ptr();
    // SAFETY: the arc label is live.
    unsafe {
        assert_eq!(rs_lvgl_sys::lv_arclabel_get_radius(raw), 20);
        assert_eq!(rs_lvgl_sys::lv_arclabel_get_angle_start(raw) as i32, 180);
        assert_eq!(rs_lvgl_sys::lv_arclabel_get_angle_size(raw) as i32, 90);
    }
    let created = ui.stats().created_nodes;
    app.state = 1;
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, created);
    // SAFETY: the arc label is live.
    unsafe {
        assert_eq!(rs_lvgl_sys::lv_arclabel_get_radius(raw), 25);
        assert_eq!(rs_lvgl_sys::lv_arclabel_get_angle_start(raw) as i32, 90);
        assert_eq!(rs_lvgl_sys::lv_arclabel_get_angle_size(raw) as i32, 180);
    }
}

#[test]
fn invalid_batch_two_properties_are_rejected() {
    let _guard = lock();
    let cases: [fn(u32) -> View<Msg>; 9] = [
        // Tile count differs from the child count.
        |_| tileview(tiles(), 0).child(label("a")),
        // Active tile out of range.
        |_| tileview([Tile::new(0, 0, ScrollDir::NONE)], 1).child(label("a")),
        // Duplicate tile position.
        |_| {
            tileview(
                [
                    Tile::new(0, 0, ScrollDir::NONE),
                    Tile::new(0, 0, ScrollDir::NONE),
                ],
                0,
            )
            .child(label("a"))
            .child(label("b"))
        },
        |_| msgbox("a\0b"),
        |_| calendar(2026, 13),
        |_| calendar(2026, 1).highlighted([CalendarDate::new(2026, 2, 0)]),
        |_| anim_image([], 100),
        |_| anim_image([ImageSource::rgb565(2, 2, &PIXELS_A)], 0),
        |_| arc_label("a", 10, 0, 400),
    ];
    for case in cases {
        let app = TestApp::new(case);
        let display = init();
        let mut ui = Ui::new(&display, Theme::dark()).unwrap();
        assert_eq!(ui.mount(&app).unwrap_err(), UiError::InvalidProperty);
    }
}
