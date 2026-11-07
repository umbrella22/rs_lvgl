//! Native behavior oracles used before adding declarative editing/range APIs.
mod common;

use core::ptr::NonNull;
use rs_lvgl_sys as sys;
use std::ffi::{CStr, CString};

struct Native(NonNull<sys::lv_obj_t>);
impl Drop for Native {
    fn drop(&mut self) {
        // SAFETY: this guard exclusively owns its live test object; the display
        // and callback data outlive it and no rendering/flush is pending.
        unsafe { sys::lv_obj_delete(self.0.as_ptr()) };
    }
}

fn textarea() -> Native {
    // SAFETY: every caller holds common::lock and has a live memory display.
    Native(NonNull::new(unsafe { sys::lv_textarea_create(common::screen().as_ptr()) }).unwrap())
}

fn text(object: &Native) -> String {
    // SAFETY: the live textarea owns this terminated buffer; copy before any
    // further native mutation, including password-buffer replacement.
    unsafe { CStr::from_ptr(sys::lv_textarea_get_text(object.0.as_ptr())) }
        .to_str()
        .unwrap()
        .to_owned()
}

#[test]
fn native_assignment_filters_until_edit_limits_are_temporarily_removed() {
    let _guard = common::lock();
    let _display = common::init();
    let object = textarea();
    let raw = object.0.as_ptr();
    let accepted = CString::new("0123中").unwrap();
    let filtered = CString::new("12x中").unwrap();
    let model = CString::new("A中e\u{301}\nZ").unwrap();
    // SAFETY: raw is this test's live textarea, and LVGL copies both strings.
    unsafe {
        sys::lv_textarea_set_accepted_chars(raw, accepted.as_ptr());
        sys::lv_textarea_set_max_length(raw, 2);
        sys::lv_textarea_set_text(raw, filtered.as_ptr());
    }
    assert_eq!(text(&object), "12", "native set_text obeys editing limits");
    // SAFETY: removing the filters permits authoritative model assignment.
    // Reinstalling the filters does not rewrite already installed model text.
    unsafe {
        sys::lv_textarea_set_accepted_chars(raw, core::ptr::null());
        sys::lv_textarea_set_max_length(raw, 0);
        sys::lv_textarea_set_text(raw, model.as_ptr());
        sys::lv_textarea_set_accepted_chars(raw, accepted.as_ptr());
        sys::lv_textarea_set_max_length(raw, 2);
    }
    assert_eq!(text(&object), model.to_str().unwrap());
    // SAFETY: only reads the textarea's live label and cursor.
    unsafe {
        assert_eq!(sys::lv_textarea_get_cursor_pos(raw), 6);
        let label = sys::lv_textarea_get_label(raw);
        assert_eq!(
            CStr::from_ptr(sys::lv_label_get_text(label)),
            model.as_c_str()
        );
        sys::lv_textarea_add_text(raw, filtered.as_ptr());
    }
    assert_eq!(
        text(&object),
        model.to_str().unwrap(),
        "further input is limited"
    );
}

#[test]
fn native_restore_and_character_cursor_work_in_password_and_one_line_modes() {
    let _guard = common::lock();
    let _display = common::init();
    for password in [false, true] {
        for one_line in [false, true] {
            let object = textarea();
            let raw = object.0.as_ptr();
            let model = CString::new("A中e\u{301}").unwrap();
            let accepted = CString::new("01中").unwrap();
            // SAFETY: all objects and strings are live; each setter copies its
            // text. Model assignment deliberately precedes editing constraints.
            unsafe {
                sys::lv_textarea_set_one_line(raw, one_line);
                sys::lv_textarea_set_password_mode(raw, password);
                sys::lv_textarea_set_text(raw, model.as_ptr());
                sys::lv_textarea_set_accepted_chars(raw, accepted.as_ptr());
                sys::lv_textarea_set_max_length(raw, 1);
                sys::lv_textarea_set_cursor_pos(raw, 2);
            }
            drop(accepted); // The copying setter must not retain Rust storage.
            assert_eq!(text(&object), model.to_str().unwrap());
            // SAFETY: all getters inspect this live textarea; empty insertion
            // cannot move its cursor or change the authoritative text.
            unsafe {
                assert_eq!(sys::lv_textarea_get_cursor_pos(raw), 2);
                assert_eq!(
                    CStr::from_ptr(sys::lv_textarea_get_accepted_chars(raw))
                        .to_str()
                        .unwrap(),
                    "01中"
                );
                sys::lv_textarea_add_text(raw, c"".as_ptr());
                assert_eq!(sys::lv_textarea_get_cursor_pos(raw), 2);
                sys::lv_textarea_set_accepted_chars(raw, core::ptr::null());
                sys::lv_textarea_set_max_length(raw, 0);
                sys::lv_textarea_add_text(raw, c"Z".as_ptr());
            }
            assert_eq!(
                text(&object),
                "A中Ze\u{301}",
                "cursor uses Unicode scalar positions"
            );
            // SAFETY: inspect/reset the existing click-placement option.
            unsafe {
                assert!(sys::lv_textarea_get_cursor_click_pos(raw));
                sys::lv_textarea_set_cursor_click_pos(raw, false);
                assert!(!sys::lv_textarea_get_cursor_click_pos(raw));
                sys::lv_textarea_set_cursor_click_pos(raw, true);
            }
        }
    }
}

#[test]
fn native_input_limits_count_scalar_values_and_zero_means_unlimited() {
    let _guard = common::lock();
    let _display = common::init();
    let object = textarea();
    let raw = object.0.as_ptr();
    // SAFETY: all literals are terminated and copied by this live textarea.
    unsafe {
        sys::lv_textarea_set_text(raw, c"".as_ptr());
        sys::lv_textarea_set_accepted_chars(raw, c"中é".as_ptr());
        sys::lv_textarea_set_max_length(raw, 2);
        sys::lv_textarea_add_text(raw, c"中é中".as_ptr());
    }
    assert_eq!(text(&object), "中中");
    // SAFETY: reset the same test textarea between independent cases.
    unsafe {
        sys::lv_textarea_set_text(raw, c"".as_ptr());
        sys::lv_textarea_set_max_length(raw, 1);
        sys::lv_textarea_add_text(raw, c"é".as_ptr());
    }
    assert_eq!(
        text(&object),
        "e",
        "a combining mark consumes its own scalar position"
    );
    // SAFETY: native zero disables the limit; an empty accepted set also allows
    // every character. Neither represents an explicit zero-character limit.
    unsafe {
        sys::lv_textarea_set_max_length(raw, 0);
        sys::lv_textarea_set_accepted_chars(raw, c"".as_ptr());
        sys::lv_textarea_set_text(raw, c"".as_ptr());
        sys::lv_textarea_add_text(raw, c"A中é".as_ptr());
    }
    assert_eq!(text(&object), "A中e\u{301}");
}

unsafe extern "C" fn count_value_event(event: *mut sys::lv_event_t) {
    // SAFETY: the test registers this callback only for VALUE_CHANGED, with a
    // live stack counter; it deletes the object before that counter expires.
    let counter = unsafe { &mut *sys::lv_event_get_user_data(event).cast::<usize>() };
    *counter += 1;
}

#[test]
fn native_range_write_order_handles_disjoint_shrinking_and_negative_domains() {
    let _guard = common::lock();
    let _display = common::init();
    let mut events = 0usize;
    // SAFETY: the display/screen are live and the guard owns the new slider.
    let object =
        Native(NonNull::new(unsafe { sys::lv_slider_create(common::screen().as_ptr()) }).unwrap());
    let raw = object.0.as_ptr();
    // SAFETY: callback data outlives object deletion; no callback can unwind.
    unsafe {
        assert!(!sys::lv_obj_add_event_cb(
            raw,
            Some(count_value_event),
            sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            (&mut events as *mut usize).cast()
        )
        .is_null());
        sys::lv_slider_set_mode(raw, sys::lv_slider_mode_t_LV_SLIDER_MODE_RANGE);
        sys::lv_slider_set_value(raw, 30, false);
        sys::lv_slider_set_start_value(raw, 10, false);
        sys::lv_slider_set_start_value(raw, 80, false);
        assert_eq!(
            sys::lv_slider_get_left_value(raw),
            30,
            "lower-first assignment is clamped by the old upper"
        );
    }
    for (min, max, lower, upper) in [
        (0, 100, 80, 90),
        (0, 100, 5, 10),
        (20, 30, 22, 28),
        (-100, -10, -90, -30),
        (7, 7, 7, 7),
    ] {
        // SAFETY: resetting normal mode releases the old endpoint dependency.
        // Reestablish the domain, upper endpoint, range mode, then lower. All
        // four canonical values are valid before native mutation.
        unsafe {
            sys::lv_slider_set_mode(raw, sys::lv_slider_mode_t_LV_SLIDER_MODE_NORMAL);
            sys::lv_slider_set_range(raw, min, max);
            sys::lv_slider_set_value(raw, upper, false);
            sys::lv_slider_set_mode(raw, sys::lv_slider_mode_t_LV_SLIDER_MODE_RANGE);
            sys::lv_slider_set_start_value(raw, lower, false);
            assert_eq!(sys::lv_slider_get_left_value(raw), lower);
            assert_eq!(sys::lv_slider_get_value(raw), upper);
        }
    }
    assert_eq!(
        events, 0,
        "native programmatic endpoint setters do not emit input"
    );
    // SAFETY: explicitly test the registered native event with its normal null
    // parameter, while object/counter remain live.
    unsafe {
        sys::lv_obj_send_event(
            raw,
            sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            core::ptr::null_mut(),
        )
    };
    assert_eq!(events, 1);
}

#[cfg(feature = "input-keypad")]
#[test]
fn flex_flow_wrap_reverse_reset_and_mapping_preserve_keyed_native_children() {
    use rs_lvgl_ui::prelude::*;
    let _guard = common::lock();
    let display = common::init();
    struct Flow {
        column: bool,
        flags: Option<(bool, bool)>,
        reorder: bool,
    }
    impl App for Flow {
        type Action = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            let children = (0..4)
                .map(|index| {
                    let id = if self.reorder { 3 - index } else { index };
                    label(format!("{id}"))
                        .key(Key::Id(id))
                        .width(px(24))
                        .height(px(16))
                        .focusable(true)
                })
                .collect::<Vec<_>>();
            let view = if self.column {
                let node = typed::column(children).map(|()| ());
                if let Some((wrap, reverse)) = self.flags {
                    node.flex_wrap(wrap).flex_reverse(reverse).into_view()
                } else {
                    node.into_view()
                }
            } else {
                let node = typed::row(children).map(|()| ());
                if let Some((wrap, reverse)) = self.flags {
                    node.flex_wrap(wrap).flex_reverse(reverse).into_view()
                } else {
                    node.into_view()
                }
            };
            view.width(px(60))
                .height(px(40))
                .padding(0)
                .gap(0)
                .justify_content(FlexAlign::Start)
                .align_items(ItemAlign::Center)
                .align_content(FlexAlign::SpaceBetween)
                .background(Color::rgb(245, 245, 245))
                .text_color(Color::rgb(10, 10, 10))
        }
    }
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = Flow {
        column: false,
        flags: None,
        reorder: false,
    };
    ui.mount(&app).unwrap();
    ui.enable_focus().unwrap();
    assert!(ui.focus(Key::Id(1)).unwrap());
    for column in [false, true] {
        app.column = column;
        app.flags = None;
        ui.update(&app).unwrap();
        let root = common::app_root();
        let ids: Vec<_> = (0..4).map(|i| common::child(root, i)).collect();
        assert!(ui.focus(Key::Id(1)).unwrap());
        for (wrap, reverse, flow) in [
            (
                true,
                false,
                if column {
                    sys::lv_flex_flow_t_LV_FLEX_FLOW_COLUMN_WRAP
                } else {
                    sys::lv_flex_flow_t_LV_FLEX_FLOW_ROW_WRAP
                },
            ),
            (
                true,
                true,
                if column {
                    sys::lv_flex_flow_t_LV_FLEX_FLOW_COLUMN_WRAP_REVERSE
                } else {
                    sys::lv_flex_flow_t_LV_FLEX_FLOW_ROW_WRAP_REVERSE
                },
            ),
            (
                false,
                true,
                if column {
                    sys::lv_flex_flow_t_LV_FLEX_FLOW_COLUMN_REVERSE
                } else {
                    sys::lv_flex_flow_t_LV_FLEX_FLOW_ROW_REVERSE
                },
            ),
            (
                false,
                false,
                if column {
                    sys::lv_flex_flow_t_LV_FLEX_FLOW_COLUMN
                } else {
                    sys::lv_flex_flow_t_LV_FLEX_FLOW_ROW
                },
            ),
        ] {
            app.flags = Some((wrap, reverse));
            ui.update(&app).unwrap();
            assert_eq!(common::app_root(), root);
            for (index, object) in ids.iter().enumerate() {
                assert_eq!(common::child(root, index as i32), *object);
            }
            // SAFETY: root/children remain live across compatible flow updates.
            unsafe {
                sys::lv_obj_update_layout(root.as_ptr());
                assert_eq!(sys::lv_obj_get_style_flex_flow(root.as_ptr(), 0), flow);
                assert_eq!(
                    sys::lv_obj_get_style_flex_cross_place(root.as_ptr(), 0),
                    sys::lv_flex_align_t_LV_FLEX_ALIGN_CENTER
                );
                assert_eq!(
                    sys::lv_obj_get_style_flex_track_place(root.as_ptr(), 0),
                    sys::lv_flex_align_t_LV_FLEX_ALIGN_SPACE_BETWEEN
                );
                assert!(sys::lv_obj_has_state(
                    ids[1].as_ptr(),
                    sys::lv_state_t_LV_STATE_FOCUSED
                ));
                let main = |i: usize| {
                    if column {
                        sys::lv_obj_get_y(ids[i].as_ptr())
                    } else {
                        sys::lv_obj_get_x(ids[i].as_ptr())
                    }
                };
                let cross = |i: usize| {
                    if column {
                        sys::lv_obj_get_x(ids[i].as_ptr())
                    } else {
                        sys::lv_obj_get_y(ids[i].as_ptr())
                    }
                };
                assert_eq!(main(0) > main(1), reverse, "main-axis order");
                assert_eq!(cross(0) != cross(2), wrap, "additional track");
                if wrap {
                    assert_eq!(
                        cross(0) > cross(2),
                        reverse,
                        "native reverse track traversal"
                    );
                }
            }
        }
        app.flags = Some((true, true));
        ui.update(&app).unwrap();
        if let Ok(folder) = std::env::var("RS_LVGL_DEPTH_ARTIFACTS") {
            let snapshot = ui.snapshot().unwrap();
            let mut ppm = format!("P6\n{} {}\n255\n", snapshot.width, snapshot.height).into_bytes();
            for rgba in snapshot.pixels.chunks_exact(4) {
                ppm.extend_from_slice(&rgba[..3]);
            }
            std::fs::write(
                std::path::Path::new(&folder).join(if column {
                    "column-wrap-reverse.ppm"
                } else {
                    "row-wrap-reverse.ppm"
                }),
                ppm,
            )
            .unwrap();
        }
        app.flags = Some((true, true));
        app.reorder = true;
        ui.update(&app).unwrap();
        for i in 0..4 {
            assert_eq!(common::child(root, i), ids[(3 - i) as usize]);
        }
        app.reorder = false;
        app.flags = None;
        ui.update(&app).unwrap();
        // SAFETY: omission resets flow while retaining the row/column object.
        assert_eq!(
            unsafe { sys::lv_obj_get_style_flex_flow(root.as_ptr(), 0) },
            if column {
                sys::lv_flex_flow_t_LV_FLEX_FLOW_COLUMN
            } else {
                sys::lv_flex_flow_t_LV_FLEX_FLOW_ROW
            }
        );
    }
}

#[cfg(feature = "strict-properties")]
#[test]
fn erased_flex_misuse_rejects_before_mutating_the_existing_tree() {
    use rs_lvgl_ui::prelude::*;
    let _guard = common::lock();
    let display = common::init();
    struct Page(bool);
    impl App for Page {
        type Action = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            if self.0 {
                label("bad").flex_wrap(true)
            } else {
                label("kept")
            }
        }
    }
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    ui.mount(&Page(false)).unwrap();
    let root = common::app_root();
    assert_eq!(ui.update(&Page(true)), Err(UiError::InvalidProperty));
    assert_eq!(common::app_root(), root);
}

#[derive(Clone, Debug, PartialEq)]
enum RangeAction {
    Pair(rs_lvgl_ui::SliderRange),
    Single(i32),
    Generic,
    Click,
}
struct RangePage {
    min: i32,
    max: i32,
    range: rs_lvgl_ui::SliderRange,
    accept: bool,
    single: bool,
    seen: Vec<RangeAction>,
}
impl rs_lvgl_ui::App for RangePage {
    type Action = RangeAction;
    fn update(&mut self, action: RangeAction) {
        if let RangeAction::Pair(pair) = action {
            if self.accept {
                self.range = pair;
            }
        }
        self.seen.push(action);
    }
    fn view(&self, _: &rs_lvgl_ui::Theme) -> rs_lvgl_ui::View<RangeAction> {
        use rs_lvgl_ui::prelude::*;
        let field = if self.single {
            slider(self.min, self.max, self.range.upper).on_value_changed(RangeAction::Single)
        } else {
            typed::range_slider(self.min, self.max, binding(self.range, RangeAction::Pair))
                .map(|v| v)
                .into_view()
        };
        column([
            field
                .key(Key::Id(1))
                .width(px(40))
                .height(px(8))
                .on_click(RangeAction::Click)
                .on_event(|event| (*event == Event::ValueChanged).then_some(RangeAction::Generic)),
            label("Stable").key(Key::Id(2)),
        ])
        .width(px(64))
        .height(px(64))
        .padding(12)
        .gap(4)
    }
}
fn send_range_event(object: NonNull<sys::lv_obj_t>) {
    // SAFETY: callers retain their mounted Slider and test UI throughout dispatch.
    unsafe {
        sys::lv_obj_send_event(
            object.as_ptr(),
            sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            core::ptr::null_mut(),
        )
    };
}
#[test]
fn controlled_range_corrects_rejection_and_preserves_single_slider_and_event_slots() {
    use rs_lvgl_ui::prelude::*;
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = RangePage {
        min: 0,
        max: 100,
        range: SliderRange::new(20, 40),
        accept: false,
        single: false,
        seen: vec![],
    };
    ui.mount(&app).unwrap();
    let object = common::child(common::app_root(), 0);
    for pair in [SliderRange::new(20, 80), SliderRange::new(5, 40)] {
        // SAFETY: emulate a native thumb move, using the known live range Slider.
        unsafe {
            sys::lv_slider_set_value(object.as_ptr(), pair.upper, false);
            sys::lv_slider_set_start_value(object.as_ptr(), pair.lower, false);
        }
        send_range_event(object);
        assert_eq!(ui.drain(&mut app, 8).unwrap().processed, 2);
        assert_eq!(
            &app.seen[app.seen.len() - 2..],
            [RangeAction::Pair(pair), RangeAction::Generic]
        );
        assert_eq!(
            ui.drain(&mut app, 8).unwrap().processed,
            0,
            "no programmatic correction feedback"
        );
        // SAFETY: keyed object retained; exact canonical endpoints are restored.
        unsafe {
            assert_eq!(sys::lv_slider_get_left_value(object.as_ptr()), 20);
            assert_eq!(sys::lv_slider_get_value(object.as_ptr()), 40);
        }
    }
    for (min, max, pair) in [
        (0, 100, SliderRange::new(80, 90)),
        (0, 100, SliderRange::new(5, 10)),
        (-100, -10, SliderRange::new(-90, -30)),
        (7, 7, SliderRange::new(7, 7)),
        (0, 100, SliderRange::new(0, 100)),
    ] {
        app.min = min;
        app.max = max;
        app.range = pair;
        ui.update(&app).unwrap();
        assert_eq!(common::child(common::app_root(), 0), object);
        unsafe {
            assert_eq!(sys::lv_slider_get_left_value(object.as_ptr()), pair.lower);
            assert_eq!(sys::lv_slider_get_value(object.as_ptr()), pair.upper);
        }
        assert_eq!(ui.drain(&mut app, 8).unwrap().processed, 0);
    }
    app.accept = true;
    unsafe {
        sys::lv_slider_set_value(object.as_ptr(), 60, false);
        sys::lv_slider_set_start_value(object.as_ptr(), 15, false);
    }
    send_range_event(object);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.range, SliderRange::new(15, 60));
    app.single = true;
    ui.update(&app).unwrap();
    unsafe {
        assert_eq!(
            sys::lv_slider_get_mode(object.as_ptr()),
            sys::lv_slider_mode_t_LV_SLIDER_MODE_NORMAL
        );
        sys::lv_slider_set_value(object.as_ptr(), 45, false);
    }
    send_range_event(object);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(
        &app.seen[app.seen.len() - 2..],
        [RangeAction::Single(45), RangeAction::Generic]
    );
    app.single = false;
    ui.update(&app).unwrap();
    assert_eq!(common::child(common::app_root(), 0), object);
    unsafe {
        sys::lv_obj_send_event(
            object.as_ptr(),
            sys::lv_event_code_t_LV_EVENT_CLICKED,
            core::ptr::null_mut(),
        );
    }
    assert_eq!(ui.drain(&mut app, 8).unwrap().processed, 1);
    assert_eq!(app.seen.last(), Some(&RangeAction::Click));
}

#[test]
fn invalid_ranges_reject_without_clamping_or_mutating_either_native_child() {
    use rs_lvgl_ui::prelude::*;
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = RangePage {
        min: 0,
        max: 100,
        range: SliderRange::new(20, 40),
        accept: false,
        single: false,
        seen: vec![],
    };
    ui.mount(&app).unwrap();
    let root = common::app_root();
    let first = common::child(root, 0);
    let second = common::child(root, 1);
    for (min, max, range) in [
        (0, 100, SliderRange::new(50, 40)),
        (0, 100, SliderRange::new(-1, 40)),
        (0, 100, SliderRange::new(20, 101)),
        (100, 0, SliderRange::new(20, 40)),
        (i32::MIN, i32::MAX, SliderRange::new(0, 1)),
        (
            i32::MIN,
            i32::MIN + 100,
            SliderRange::new(i32::MIN, i32::MIN + 50),
        ),
        (
            i32::MAX - 100,
            i32::MAX,
            SliderRange::new(i32::MAX - 50, i32::MAX),
        ),
    ] {
        app.min = min;
        app.max = max;
        app.range = range;
        assert_eq!(ui.update(&app), Err(UiError::InvalidProperty));
        assert_eq!(common::app_root(), root);
        assert_eq!(common::child(root, 0), first);
        assert_eq!(common::child(root, 1), second);
        unsafe {
            assert_eq!(sys::lv_slider_get_left_value(first.as_ptr()), 20);
            assert_eq!(sys::lv_slider_get_value(first.as_ptr()), 40);
        }
    }
}

#[test]
fn range_binding_replacement_drops_captures_once_and_uses_one_shared_value_slot() {
    use rs_lvgl_ui::prelude::*;
    use std::{cell::Cell, rc::Rc};
    struct Probe(Rc<Cell<usize>>);
    impl Drop for Probe {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let drops = Rc::new(Cell::new(0));
    let probe = Probe(drops.clone());
    let node = typed::range_slider(
        0,
        100,
        binding(SliderRange::new(20, 40), move |_| {
            let _ = &probe;
        }),
    );
    let node = node.on_range_changed(|_| ());
    assert_eq!(drops.get(), 1);
    drop(node);
    assert_eq!(drops.get(), 1);
    let probe = Probe(drops.clone());
    let node = range_slider(0, 100, SliderRange::new(20, 40)).on_range_changed(move |_| {
        let _ = &probe;
    });
    let node = node.on_value_changed(|_| ());
    assert_eq!(drops.get(), 2);
    drop(node);
    assert_eq!(drops.get(), 2);
}

struct TextPage {
    value: String,
    accepted: Option<String>,
    maximum: Option<core::num::NonZeroU32>,
    cursor: Option<u32>,
    click: bool,
    password: bool,
    multiline: bool,
    accept: bool,
    seen: Vec<String>,
}
impl Default for TextPage {
    fn default() -> Self {
        Self {
            value: String::new(),
            accepted: None,
            maximum: None,
            cursor: None,
            click: true,
            password: false,
            multiline: false,
            accept: false,
            seen: vec![],
        }
    }
}
impl rs_lvgl_ui::App for TextPage {
    type Action = String;
    fn update(&mut self, value: String) {
        if self.accept {
            self.value = value.clone();
        }
        self.seen.push(value);
    }
    fn view(&self, _: &rs_lvgl_ui::Theme) -> rs_lvgl_ui::View<String> {
        use rs_lvgl_ui::prelude::*;
        let mut field = typed::textarea(binding(&self.value, |s: &str| s.to_owned()))
            .key(Key::Id(1))
            .input_max_length(self.maximum)
            .cursor_position(self.cursor)
            .cursor_click_position(self.click)
            .password(self.password)
            .keyboard_id(9)
            .map(|s| s);
        if let Some(chars) = &self.accepted {
            field = field.accepted_chars(chars.clone());
        }
        if self.multiline {
            field = field.multiline();
        }
        #[allow(unused_mut)]
        let mut root = column([field.into_view(), label("stable").key(Key::Id(2))]);
        #[cfg(feature = "widget-keyboard")]
        {
            root = root.child(keyboard(9).key(Key::Id(3)));
        }
        root
    }
}
fn field_value(raw: *mut sys::lv_obj_t) -> String {
    // SAFETY: all callers keep their mounted textarea live throughout this copy.
    unsafe { CStr::from_ptr(sys::lv_textarea_get_text(raw)) }
        .to_str()
        .unwrap()
        .to_owned()
}

#[test]
fn text_options_preserve_full_model_unicode_cursor_and_keyboard_binding() {
    use rs_lvgl_ui::prelude::*;
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = TextPage {
        value: "A中e\u{301}\nZ".into(),
        accepted: Some("01中".into()),
        maximum: core::num::NonZeroU32::new(1),
        cursor: Some(3),
        click: false,
        multiline: true,
        ..Default::default()
    };
    ui.mount(&app).unwrap();
    let field = common::child(common::app_root(), 0);
    let sibling = common::child(common::app_root(), 1);
    for password in [true, false, true] {
        app.password = password;
        ui.update(&app).unwrap();
        assert_eq!(field_value(field.as_ptr()), app.value);
        assert_eq!(common::child(common::app_root(), 0), field);
        assert_eq!(common::child(common::app_root(), 1), sibling);
        // SAFETY: synchronous getters on the live controlled field and keyboard.
        unsafe {
            assert_eq!(sys::lv_textarea_get_cursor_pos(field.as_ptr()), 3);
            assert!(!sys::lv_textarea_get_cursor_click_pos(field.as_ptr()));
            assert_eq!(
                CStr::from_ptr(sys::lv_textarea_get_accepted_chars(field.as_ptr())),
                c"01中"
            );
            assert_eq!(sys::lv_textarea_get_max_length(field.as_ptr()), 1);
            #[cfg(feature = "widget-keyboard")]
            assert_eq!(
                sys::lv_keyboard_get_textarea(common::child(common::app_root(), 2).as_ptr()),
                field.as_ptr()
            );
            sys::lv_textarea_set_cursor_pos(field.as_ptr(), 0);
        }
        ui.update(&app).unwrap();
        assert_eq!(
            unsafe { sys::lv_textarea_get_cursor_pos(field.as_ptr()) },
            3
        );
        assert_eq!(
            ui.drain(&mut app, 32).unwrap().processed,
            0,
            "no programmatic feedback"
        );
    }
    app.accepted = None;
    app.maximum = None;
    app.cursor = None;
    app.click = true;
    ui.update(&app).unwrap();
    unsafe {
        assert!(sys::lv_textarea_get_accepted_chars(field.as_ptr()).is_null());
        assert_eq!(sys::lv_textarea_get_max_length(field.as_ptr()), 0);
        assert!(sys::lv_textarea_get_cursor_click_pos(field.as_ptr()));
        sys::lv_textarea_set_cursor_pos(field.as_ptr(), 2);
    }
    ui.update(&app).unwrap();
    assert_eq!(
        unsafe { sys::lv_textarea_get_cursor_pos(field.as_ptr()) },
        2,
        "omitted explicit cursor preserves the native position on unchanged text"
    );
    app.value = "Z".into();
    ui.update(&app).unwrap();
    assert_eq!(field_value(field.as_ptr()), "Z");
}

#[test]
fn editing_filters_apply_to_input_and_rejected_text_is_corrected_without_feedback() {
    use rs_lvgl_ui::prelude::*;
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = TextPage {
        accepted: Some("中e\u{301}".into()),
        maximum: core::num::NonZeroU32::new(2),
        ..Default::default()
    };
    ui.mount(&app).unwrap();
    let field = common::child(common::app_root(), 0);
    unsafe {
        sys::lv_textarea_add_text(field.as_ptr(), c"中xé中".as_ptr());
    }
    assert_eq!(field_value(field.as_ptr()), "中e");
    assert!(ui.drain(&mut app, 32).unwrap().processed > 0);
    assert_eq!(app.seen.last().unwrap(), "中e");
    assert_eq!(field_value(field.as_ptr()), "");
    assert_eq!(ui.drain(&mut app, 32).unwrap().processed, 0);
    app.accepted = Some("Z".into());
    app.maximum = None;
    app.accept = true;
    ui.update(&app).unwrap();
    unsafe {
        sys::lv_textarea_add_text(field.as_ptr(), c"中Z".as_ptr());
    }
    ui.drain(&mut app, 32).unwrap();
    assert_eq!(app.value, "Z");
    assert_eq!(field_value(field.as_ptr()), "Z");
    // Canonical assignment bypasses installed filters in both modes.
    for password in [false, true] {
        app.password = password;
        app.value = "A中e\u{301}".into();
        ui.update(&app).unwrap();
        assert_eq!(field_value(field.as_ptr()), app.value);
    }
}

#[test]
fn invalid_text_options_preserve_both_native_children_before_mutation() {
    use rs_lvgl_ui::prelude::*;
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = TextPage {
        value: "中e\u{301}".into(),
        ..Default::default()
    };
    ui.mount(&app).unwrap();
    let root = common::app_root();
    let field = common::child(root, 0);
    let sibling = common::child(root, 1);
    for (accepted, cursor) in [
        (Some(""), None),
        (Some("a\0b"), None),
        (None, Some(4)),
        (None, Some(u32::MAX)),
    ] {
        app.accepted = accepted.map(str::to_owned);
        app.cursor = cursor;
        assert_eq!(ui.update(&app), Err(UiError::InvalidProperty));
        assert_eq!(common::app_root(), root);
        assert_eq!(common::child(root, 0), field);
        assert_eq!(common::child(root, 1), sibling);
        assert_eq!(field_value(field.as_ptr()), app.value);
    }
}

#[cfg(feature = "strict-properties")]
#[test]
fn erased_text_options_reject_wrong_native_family_before_mutation() {
    use rs_lvgl_ui::prelude::*;
    struct Page(bool);
    impl App for Page {
        type Action = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            let node = label("stable");
            if self.0 {
                node.accepted_chars("x")
            } else {
                node
            }
        }
    }
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    ui.mount(&Page(false)).unwrap();
    let root = common::app_root();
    assert_eq!(ui.update(&Page(true)), Err(UiError::InvalidProperty));
    assert_eq!(common::app_root(), root);
}
