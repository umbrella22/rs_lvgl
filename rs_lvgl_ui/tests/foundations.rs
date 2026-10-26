//! Host tests for the composition foundations: message mapping, state
//! styles, controlled checked state, floating offsets, the overlay layer and
//! symbol glyphs.

mod common;

use core::ptr::{self, NonNull};
use rs_lvgl_ui::prelude::*;

use common::{app_root, child, init, lock};
use rs_lvgl_sys::lv_obj_t;

#[derive(Clone, Debug, PartialEq)]
enum Inner {
    Clicked,
    Value(i32),
    Event,
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Inner(Inner),
    Outer(Inner),
    Close,
}

/// A view built from a function of a small state value, with an optional
/// overlay built the same way.
struct TestApp {
    state: u32,
    view: fn(u32) -> View<Msg>,
    overlay: Option<fn(u32) -> View<Msg>>,
    received: Vec<Msg>,
}

impl TestApp {
    fn new(view: fn(u32) -> View<Msg>) -> Self {
        Self {
            state: 0,
            view,
            overlay: None,
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

    fn overlay(&self, _: &Theme) -> Option<View<Msg>> {
        self.overlay.map(|overlay| overlay(self.state))
    }
}

fn mount(app: &TestApp) -> (Ui<Msg>, NonNull<lv_obj_t>) {
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    ui.mount(app).unwrap();
    let widget = child(app_root(), 0);
    (ui, widget)
}

fn send(obj: NonNull<lv_obj_t>, code: rs_lvgl_sys::lv_event_code_t) {
    // SAFETY: tests only send events to live mounted objects.
    unsafe { rs_lvgl_sys::lv_obj_send_event(obj.as_ptr(), code, ptr::null_mut()) };
}

fn click(obj: NonNull<lv_obj_t>) {
    send(obj, rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED);
}

fn child_count(obj: *mut lv_obj_t) -> u32 {
    // SAFETY: tests pass live objects.
    unsafe { rs_lvgl_sys::lv_obj_get_child_count(obj) }
}

fn layer_top() -> *mut lv_obj_t {
    // SAFETY: a display was registered by `init`.
    unsafe { rs_lvgl_sys::lv_layer_top() }
}

fn has_state(obj: NonNull<lv_obj_t>, state: rs_lvgl_sys::lv_state_t) -> bool {
    // SAFETY: tests pass live objects.
    unsafe { rs_lvgl_sys::lv_obj_has_state(obj.as_ptr(), state) }
}

fn has_flag(obj: NonNull<lv_obj_t>, flag: rs_lvgl_sys::lv_obj_flag_t) -> bool {
    // SAFETY: tests pass live objects.
    unsafe { rs_lvgl_sys::lv_obj_has_flag(obj.as_ptr(), flag) }
}

/// The local value of `prop` under `selector`, if one is set.
fn local_prop(
    obj: NonNull<lv_obj_t>,
    prop: u32,
    selector: u32,
) -> Option<rs_lvgl_sys::lv_style_value_t> {
    let mut value = rs_lvgl_sys::lv_style_value_t { num: 0 };
    // SAFETY: tests pass live objects and a valid out pointer.
    let found = unsafe {
        rs_lvgl_sys::lv_obj_get_local_style_prop(
            obj.as_ptr(),
            prop as rs_lvgl_sys::lv_style_prop_t,
            &mut value,
            selector,
        )
    };
    (found == rs_lvgl_sys::lv_style_res_t_LV_STYLE_RES_FOUND).then_some(value)
}

fn bg_color(obj: NonNull<lv_obj_t>, selector: u32) -> Option<(u8, u8, u8)> {
    let value = local_prop(obj, rs_lvgl_sys::_lv_style_id_t_LV_STYLE_BG_COLOR, selector)?;
    // SAFETY: BG_COLOR stores a color.
    let color = unsafe { value.color };
    Some((color.red, color.green, color.blue))
}

const PRESSED: u32 = rs_lvgl_sys::lv_state_t_LV_STATE_PRESSED;
const CHECKED: u32 = rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED;

#[test]
fn map_converts_click_value_and_event_messages() {
    let _guard = lock();
    let mut app = TestApp::new(|_| {
        let inner: View<Inner> = column([])
            .child(button("go").on_click(Inner::Clicked))
            .child(slider(0, 10, 3).on_value_changed(Inner::Value))
            .child(obj().on_event(|event| (*event == Event::Clicked).then_some(Inner::Event)));
        // Nested maps compose.
        inner.map(Msg::Inner).map(|msg| match msg {
            Msg::Inner(inner) => Msg::Outer(inner),
            other => other,
        })
    });
    let (mut ui, widget) = mount(&app);

    click(child(widget, 0));
    // SAFETY: the slider is live; the set does not send an event itself.
    unsafe { rs_lvgl_sys::lv_slider_set_value(child(widget, 1).as_ptr(), 7, false) };
    send(
        child(widget, 1),
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
    );
    click(child(widget, 2));
    ui.drain(&mut app, 8).unwrap();

    assert_eq!(
        app.received,
        vec![
            Msg::Outer(Inner::Clicked),
            Msg::Outer(Inner::Value(7)),
            Msg::Outer(Inner::Event),
        ]
    );
}

#[test]
fn map_keeps_object_identity_across_renders() {
    let _guard = lock();
    let mut app = TestApp::new(|state| {
        button(format!("n{state}"))
            .on_click(Inner::Clicked)
            .map(Msg::Inner)
    });
    let (mut ui, widget) = mount(&app);
    app.state = 1;
    ui.update(&app).unwrap();
    assert_eq!(child(app_root(), 0), widget);
    click(widget);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.received, vec![Msg::Inner(Inner::Clicked)]);
}

#[test]
fn state_styles_apply_and_clear() {
    let _guard = lock();
    let mut app = TestApp::new(|state| {
        let node = button("b").style(Style::default().bg(Color::rgb(1, 2, 3)));
        match state {
            0 => node.state_style(State::PRESSED, Style::default().bg(Color::rgb(9, 8, 7))),
            1 => node.state_style(
                State::PRESSED | State::CHECKED,
                Style::default().bg(Color::rgb(4, 5, 6)),
            ),
            _ => node,
        }
    });
    let (mut ui, widget) = mount(&app);
    assert_eq!(bg_color(widget, 0), Some((1, 2, 3)));
    assert_eq!(bg_color(widget, PRESSED), Some((9, 8, 7)));

    app.state = 1;
    ui.update(&app).unwrap();
    assert_eq!(bg_color(widget, PRESSED), None);
    assert_eq!(bg_color(widget, PRESSED | CHECKED), Some((4, 5, 6)));
    assert_eq!(bg_color(widget, 0), Some((1, 2, 3)));

    app.state = 2;
    ui.update(&app).unwrap();
    assert_eq!(bg_color(widget, PRESSED | CHECKED), None);
    assert_eq!(bg_color(widget, 0), Some((1, 2, 3)));
}

#[test]
fn part_state_style_targets_part_and_state() {
    let _guard = lock();
    let app = TestApp::new(|_| {
        slider(0, 10, 5).part_state_style(
            Part::Knob,
            State::PRESSED,
            Style::default().bg(Color::rgb(10, 20, 30)),
        )
    });
    let (_ui, widget) = mount(&app);
    let knob = rs_lvgl_sys::lv_part_t_LV_PART_KNOB;
    assert_eq!(bg_color(widget, knob | PRESSED), Some((10, 20, 30)));
    assert_eq!(bg_color(widget, knob), None);
}

#[test]
fn state_style_ignores_object_wide_props() {
    let _guard = lock();
    let app = TestApp::new(|_| {
        obj().state_style(State::PRESSED, Style::default().hidden(true).floating(true))
    });
    let (_ui, widget) = mount(&app);
    assert!(!has_flag(
        widget,
        rs_lvgl_sys::lv_obj_flag_t_LV_OBJ_FLAG_HIDDEN
    ));
    assert!(!has_flag(
        widget,
        rs_lvgl_sys::lv_obj_flag_t_LV_OBJ_FLAG_FLOATING
    ));
}

#[test]
fn checked_is_controlled() {
    let _guard = lock();
    let mut app = TestApp::new(|state| match state {
        0 => button("b").checked(true),
        1 => button("b").checked(false),
        _ => button("b"),
    });
    let (mut ui, widget) = mount(&app);
    let checked = rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED;
    assert!(has_state(widget, checked));

    app.state = 1;
    ui.update(&app).unwrap();
    assert!(!has_state(widget, checked));

    // A native change is corrected on the next render.
    // SAFETY: the button is live.
    unsafe { rs_lvgl_sys::lv_obj_add_state(widget.as_ptr(), checked) };
    ui.update(&app).unwrap();
    assert!(!has_state(widget, checked));

    // Unset leaves the native state alone.
    app.state = 2;
    // SAFETY: the button is live.
    unsafe { rs_lvgl_sys::lv_obj_add_state(widget.as_ptr(), checked) };
    ui.update(&app).unwrap();
    assert!(has_state(widget, checked));
}

#[test]
fn checked_is_ignored_for_switch() {
    let _guard = lock();
    let app = TestApp::new(|_| switch(true).checked(false));
    let (_ui, widget) = mount(&app);
    assert!(has_state(widget, rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED));
}

#[test]
fn floating_and_offset_apply_and_clear() {
    let _guard = lock();
    let mut app = TestApp::new(|state| {
        if state == 0 {
            obj().style(
                Style::default()
                    .floating(true)
                    .align(Align::TopRight)
                    .offset(-4, 6),
            )
        } else {
            obj()
        }
    });
    let (mut ui, widget) = mount(&app);
    let floating = rs_lvgl_sys::lv_obj_flag_t_LV_OBJ_FLAG_FLOATING;
    assert!(has_flag(widget, floating));
    let x = local_prop(widget, rs_lvgl_sys::_lv_style_id_t_LV_STYLE_X, 0).unwrap();
    let y = local_prop(widget, rs_lvgl_sys::_lv_style_id_t_LV_STYLE_Y, 0).unwrap();
    // SAFETY: X and Y store numbers.
    assert_eq!(unsafe { (x.num, y.num) }, (-4, 6));

    app.state = 1;
    ui.update(&app).unwrap();
    assert!(!has_flag(widget, floating));
    assert!(local_prop(widget, rs_lvgl_sys::_lv_style_id_t_LV_STYLE_X, 0).is_none());
    assert!(local_prop(widget, rs_lvgl_sys::_lv_style_id_t_LV_STYLE_Y, 0).is_none());
}

fn flex_place(obj: NonNull<lv_obj_t>, prop: u32) -> Option<i32> {
    // SAFETY: flex placement props store numbers.
    local_prop(obj, prop, 0).map(|value| unsafe { value.num })
}

#[test]
fn flex_align_applies_and_clears() {
    let _guard = lock();
    let mut app = TestApp::new(|state| {
        let column = column([]).child(label("a"));
        if state == 0 {
            column.style(Style::default().flex_align(FlexAlign::SpaceBetween, FlexAlign::Center))
        } else {
            column
        }
    });
    let (mut ui, widget) = mount(&app);
    let main = rs_lvgl_sys::_lv_style_id_t_LV_STYLE_FLEX_MAIN_PLACE;
    let cross = rs_lvgl_sys::_lv_style_id_t_LV_STYLE_FLEX_CROSS_PLACE;
    assert_eq!(
        flex_place(widget, main),
        Some(rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_SPACE_BETWEEN as i32)
    );
    assert_eq!(
        flex_place(widget, cross),
        Some(rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_CENTER as i32)
    );
    assert_eq!(
        flex_place(
            widget,
            rs_lvgl_sys::_lv_style_id_t_LV_STYLE_FLEX_TRACK_PLACE
        ),
        Some(rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_CENTER as i32)
    );

    // Unset falls back to Start.
    app.state = 1;
    ui.update(&app).unwrap();
    // SAFETY: the column is live.
    let (main_now, cross_now) = unsafe {
        (
            rs_lvgl_sys::lv_obj_get_style_flex_main_place(widget.as_ptr(), 0),
            rs_lvgl_sys::lv_obj_get_style_flex_cross_place(widget.as_ptr(), 0),
        )
    };
    assert_eq!(main_now, rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_START);
    assert_eq!(cross_now, rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_START);
}

#[test]
fn flex_align_centers_children_after_layout() {
    let _guard = lock();
    let app = TestApp::new(|_| {
        column([])
            .style(
                Style::default()
                    .width(Size::Px(60))
                    .pad(0)
                    .flex_align(FlexAlign::Start, FlexAlign::Center),
            )
            .child(obj().style(Style::neutral().width(Size::Px(20)).height(Size::Px(10))))
    });
    let (_ui, widget) = mount(&app);
    let item = child(widget, 0);
    // SAFETY: both objects are live.
    let (parent_x, item_x) = unsafe {
        rs_lvgl_sys::lv_obj_update_layout(widget.as_ptr());
        (
            rs_lvgl_sys::lv_obj_get_x(widget.as_ptr()),
            rs_lvgl_sys::lv_obj_get_x(item.as_ptr()),
        )
    };
    assert_eq!(parent_x, 0);
    assert_eq!(item_x, 20);
}

#[test]
fn overlay_mounts_updates_and_unmounts_on_top_layer() {
    let _guard = lock();
    let mut app = TestApp::new(|_| label("view"));
    let (mut ui, _) = mount(&app);
    assert_eq!(child_count(layer_top()), 0);
    let view_nodes = ui.stats().mounted_nodes;

    app.overlay = Some(|state| {
        column([])
            .child(label(format!("overlay {state}")))
            .child(button("close").on_click(Msg::Close))
    });
    ui.update(&app).unwrap();
    assert_eq!(child_count(layer_top()), 1);
    let overlay_root = child(NonNull::new(layer_top()).unwrap(), 0);
    assert!(!has_flag(
        overlay_root,
        rs_lvgl_sys::lv_obj_flag_t_LV_OBJ_FLAG_CLICKABLE
    ));
    let tree = child(overlay_root, 0);
    assert_eq!(ui.stats().mounted_nodes, view_nodes + 3);

    // Updates keep identity; events reach the shared queue.
    app.state = 1;
    ui.update(&app).unwrap();
    assert_eq!(child(overlay_root, 0), tree);
    click(child(tree, 1));
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.received, vec![Msg::Close]);

    app.overlay = None;
    ui.update(&app).unwrap();
    assert_eq!(child_count(layer_top()), 0);
    assert_eq!(ui.stats().mounted_nodes, view_nodes);
}

#[test]
fn overlay_is_deleted_on_unmount_and_drop() {
    let _guard = lock();
    let mut app = TestApp::new(|_| label("view"));
    app.overlay = Some(|_| label("overlay"));
    let (mut ui, _) = mount(&app);
    assert_eq!(child_count(layer_top()), 1);
    ui.unmount();
    assert_eq!(child_count(layer_top()), 0);

    let (ui, _) = mount(&app);
    assert_eq!(child_count(layer_top()), 1);
    drop(ui);
    assert_eq!(child_count(layer_top()), 0);
}

#[test]
fn keyboard_in_overlay_binds_textarea_in_view() {
    let _guard = lock();
    let mut app = TestApp::new(|_| textarea("").keyboard_id(1));
    app.overlay = Some(|_| keyboard(1));
    let (_ui, textarea) = mount(&app);
    let keyboard = child(child(NonNull::new(layer_top()).unwrap(), 0), 0);
    // SAFETY: the keyboard is live.
    let bound = unsafe { rs_lvgl_sys::lv_keyboard_get_textarea(keyboard.as_ptr()) };
    assert_eq!(bound, textarea.as_ptr());
}

#[test]
fn symbols_are_lvgl_glyphs() {
    assert_eq!(symbol::OK, "\u{F00C}");
    assert_eq!(symbol::CLOSE, "\u{F00D}");
    assert_eq!(symbol::BULLET, "\u{2022}");
    let label = format!("{} Save", symbol::SAVE);
    assert!(label.ends_with(" Save"));
}
