mod common;

use rs_lvgl_ui::prelude::*;
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Key(u32, u32),
    Rotary(i32),
    Gesture(GestureDirection),
    Long,
    Pointer(PointerEvent),
    Generic(Event),
    Bubble(InputEvent),
    ParentValue(i32),
}

struct Events {
    generation: u32,
    handlers: bool,
    bubble: bool,
    captures: Rc<Cell<usize>>,
    seen: Vec<Msg>,
}

impl App for Events {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) {
        self.seen.push(msg);
    }
    fn view(&self, _: &Theme) -> View<Msg> {
        let generation = self.generation;
        let mut child = obj()
            .style(Style::neutral().width(Size::Px(32)).height(Size::Px(32)))
            .scrollable(false)
            .gesture_bubble(false)
            .event_bubble(self.bubble);
        if self.handlers {
            let captures = self.captures.clone();
            child = child
                .on_key(move |key| {
                    captures.set(captures.get() + 1);
                    Msg::Key(generation, key)
                })
                .on_rotary(Msg::Rotary)
                .on_gesture(Msg::Gesture)
                .on_long_press(Msg::Long)
                .on_pointer(Msg::Pointer)
                .on_event(|event| (*event == Event::Key).then_some(Msg::Generic(*event)));
        }
        // Every new handler must survive message mapping, including the
        // descendant observer. Mapping happens before native registration.
        column([])
            .child(child)
            .on_value_changed(Msg::ParentValue)
            .on_bubbled_event(|event| Some(Msg::Bubble(*event)))
            .map(|msg| msg)
    }
}

unsafe fn send(obj: *mut rs_lvgl_sys::lv_obj_t, code: u32, param: *mut core::ffi::c_void) {
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_obj_send_event(obj, code, param) },
        rs_lvgl_sys::lv_result_t_LV_RESULT_OK
    );
}

#[test]
fn payloads_map_replace_remove_and_bubble_without_parent_value_reads() {
    let _lock = common::lock();
    let display = common::init();
    let captures = Rc::new(Cell::new(0));
    let mut app = Events {
        generation: 1,
        handlers: true,
        bubble: true,
        captures: captures.clone(),
        seen: vec![],
    };
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    ui.mount(&app).unwrap();
    let obj = common::child(common::app_root(), 0).as_ptr();
    let mut key = 0xFEDC_BA98u32;
    let mut rotary = -123_456i32;
    unsafe {
        send(
            obj,
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_KEY,
            (&mut key as *mut u32).cast(),
        );
        send(
            obj,
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_ROTARY,
            (&mut rotary as *mut i32).cast(),
        );
        send(
            obj,
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_GESTURE,
            core::ptr::null_mut(),
        );
        send(
            obj,
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_GESTURE_LEFT,
            core::ptr::null_mut(),
        );
        send(
            obj,
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_LONG_PRESSED,
            core::ptr::null_mut(),
        );
        send(
            obj,
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_LONG_PRESSED_REPEAT,
            core::ptr::null_mut(),
        );
        send(
            obj,
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            core::ptr::null_mut(),
        );
    }
    // Copy before the C event ends; changing stack payloads cannot change
    // the queued Rust messages. No application update has happened yet.
    key = 7;
    rotary = 9;
    assert_eq!((key, rotary), (7, 9));
    assert!(app.seen.is_empty());
    ui.drain(&mut app, 64).unwrap();
    assert!(app.seen.contains(&Msg::Key(1, 0xFEDC_BA98)));
    assert!(app.seen.contains(&Msg::Rotary(-123_456)));
    assert!(app
        .seen
        .contains(&Msg::Bubble(InputEvent::Key(0xFEDC_BA98))));
    assert!(app.seen.contains(&Msg::Generic(Event::Key)));
    assert_eq!(
        app.seen
            .iter()
            .filter(|m| **m == Msg::Gesture(GestureDirection::Left))
            .count(),
        1
    );
    assert_eq!(app.seen.iter().filter(|m| **m == Msg::Long).count(), 1);
    assert!(!app.seen.iter().any(|m| matches!(m, Msg::ParentValue(_))));
    assert_eq!(captures.get(), 1);
    app.generation = 2;
    app.bubble = false;
    app.seen.clear();
    ui.update(&app).unwrap();
    unsafe {
        send(
            obj,
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_KEY,
            (&mut key as *mut u32).cast(),
        );
    }
    ui.drain(&mut app, 64).unwrap();
    assert_eq!(app.seen, vec![Msg::Generic(Event::Key), Msg::Key(2, 7)]);
    app.handlers = false;
    app.seen.clear();
    ui.update(&app).unwrap();
    unsafe {
        send(
            obj,
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_KEY,
            (&mut key as *mut u32).cast(),
        );
    }
    assert_eq!(ui.drain(&mut app, 64).unwrap().processed, 0);
    assert_eq!(
        Rc::strong_count(&captures),
        2,
        "removed handler must drop its capture"
    );
    ui.unmount();
    drop(ui);
    assert_eq!(Rc::strong_count(&captures), 2);
}

#[test]
fn pointer_payload_comes_from_the_native_device() {
    let _lock = common::lock();
    let display = common::init();
    let mut app = Events {
        generation: 0,
        handlers: true,
        bubble: false,
        captures: Rc::new(Cell::new(0)),
        seen: vec![],
    };
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    ui.mount(&app).unwrap();
    unsafe extern "C" fn read(
        _: *mut rs_lvgl_sys::lv_indev_t,
        data: *mut rs_lvgl_sys::lv_indev_data_t,
    ) {
        unsafe {
            (*data).point.x = 12;
            (*data).point.y = 13;
            (*data).state = rs_lvgl_sys::lv_indev_state_t_LV_INDEV_STATE_PRESSED;
        }
    }
    unsafe {
        rs_lvgl_sys::lv_obj_update_layout(common::screen().as_ptr());
        let indev = rs_lvgl_sys::lv_indev_create();
        rs_lvgl_sys::lv_indev_set_type(indev, rs_lvgl_sys::lv_indev_type_t_LV_INDEV_TYPE_POINTER);
        rs_lvgl_sys::lv_indev_set_read_cb(indev, Some(read));
        rs_lvgl_sys::lv_indev_read(indev);
        rs_lvgl_sys::lv_indev_delete(indev);
    }
    ui.drain(&mut app, 64).unwrap();
    assert!(app.seen.contains(&Msg::Pointer(PointerEvent {
        event: Event::Pressed,
        x: 12,
        y: 13
    })));
}

struct Scroller {
    offset: Option<ScrollOffset>,
    reset: bool,
    seen: Vec<ScrollOffset>,
}

impl App for Scroller {
    type Msg = ScrollOffset;
    fn update(&mut self, msg: ScrollOffset) {
        self.seen.push(msg);
    }
    fn view(&self, _: &Theme) -> View<ScrollOffset> {
        let mut node = column([])
            .style(Style::neutral().width(Size::Px(40)).height(Size::Px(40)))
            .scroll_dir(ScrollDir::VER)
            .scrollbar_mode(ScrollbarMode::Active)
            .scroll_snap_x(ScrollSnap::None)
            .scroll_snap_y(ScrollSnap::None)
            .scroll_momentum(!self.reset)
            .scroll_elastic(!self.reset)
            .on_scroll(|p| p)
            .child(obj().style(Style::neutral().height(Size::Px(200)).width(Size::Px(20))));
        if let Some(offset) = self.offset {
            node = node.scroll_to(offset.x, offset.y);
        }
        node
    }
}

#[test]
fn controlled_scroll_uses_layout_bounds_and_unset_releases_control() {
    let _lock = common::lock();
    let display = common::init();
    let mut app = Scroller {
        offset: Some(ScrollOffset { x: 0, y: 50 }),
        reset: false,
        seen: vec![],
    };
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    ui.mount(&app).unwrap();
    let obj = common::app_root().as_ptr();
    unsafe {
        assert_eq!(rs_lvgl_sys::lv_obj_get_scroll_y(obj), 50);
        assert_eq!(
            rs_lvgl_sys::lv_obj_get_scroll_dir(obj),
            rs_lvgl_sys::lv_dir_t_LV_DIR_VER
        );
        assert_eq!(
            rs_lvgl_sys::lv_obj_get_scrollbar_mode(obj),
            rs_lvgl_sys::lv_scrollbar_mode_t_LV_SCROLLBAR_MODE_ACTIVE
        );
        assert_eq!(
            rs_lvgl_sys::lv_obj_get_scroll_snap_y(obj),
            rs_lvgl_sys::lv_scroll_snap_t_LV_SCROLL_SNAP_NONE
        );
        assert!(rs_lvgl_sys::lv_obj_is_scroll_momentum(obj));
        assert!(rs_lvgl_sys::lv_obj_is_scroll_elastic(obj));
        rs_lvgl_sys::lv_obj_scroll_to(obj, 0, 80, false);
    }
    ui.update(&app).unwrap();
    assert_eq!(unsafe { rs_lvgl_sys::lv_obj_get_scroll_y(obj) }, 50);
    ui.drain(&mut app, 64).unwrap();
    assert!(app.seen.contains(&ScrollOffset { x: 0, y: 80 }));
    app.offset = Some(ScrollOffset { x: 0, y: i32::MAX });
    assert_eq!(ui.update(&app), Err(UiError::InvalidProperty));
    assert_eq!(common::app_root().as_ptr(), obj);
    assert_eq!(unsafe { rs_lvgl_sys::lv_obj_get_scroll_y(obj) }, 50);
    app.offset = Some(ScrollOffset { x: 0, y: 10_000 });
    ui.update(&app).unwrap();
    assert_eq!(unsafe { rs_lvgl_sys::lv_obj_get_scroll_y(obj) }, 160);
    app.offset = None;
    app.reset = true;
    ui.update(&app).unwrap();
    unsafe {
        rs_lvgl_sys::lv_obj_scroll_to(obj, 0, 75, false);
    }
    ui.drain(&mut app, 64).unwrap();
    assert_eq!(unsafe { rs_lvgl_sys::lv_obj_get_scroll_y(obj) }, 75);
    assert!(!unsafe { rs_lvgl_sys::lv_obj_is_scroll_momentum(obj) });
    assert!(!unsafe { rs_lvgl_sys::lv_obj_is_scroll_elastic(obj) });
}

#[test]
fn native_pointer_drag_scrolls_and_reports_the_actual_offset() {
    let _lock = common::lock();
    let display = common::init();
    let mut app = Scroller {
        offset: None,
        reset: false,
        seen: vec![],
    };
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    ui.mount(&app).unwrap();
    let obj = common::app_root().as_ptr();
    struct Sample {
        y: Cell<i32>,
        pressed: Cell<bool>,
    }
    unsafe extern "C" fn read(
        indev: *mut rs_lvgl_sys::lv_indev_t,
        data: *mut rs_lvgl_sys::lv_indev_data_t,
    ) {
        // SAFETY: this test's stack sample outlives all synchronous reads.
        unsafe {
            let sample = &*rs_lvgl_sys::lv_indev_get_user_data(indev).cast::<Sample>();
            (*data).point.x = 12;
            (*data).point.y = sample.y.get();
            (*data).state = if sample.pressed.get() {
                rs_lvgl_sys::lv_indev_state_t_LV_INDEV_STATE_PRESSED
            } else {
                rs_lvgl_sys::lv_indev_state_t_LV_INDEV_STATE_RELEASED
            };
        }
    }
    let sample = Sample {
        y: Cell::new(35),
        pressed: Cell::new(true),
    };
    unsafe {
        rs_lvgl_sys::lv_obj_update_layout(common::screen().as_ptr());
        let indev = rs_lvgl_sys::lv_indev_create();
        rs_lvgl_sys::lv_indev_set_type(indev, rs_lvgl_sys::lv_indev_type_t_LV_INDEV_TYPE_POINTER);
        rs_lvgl_sys::lv_indev_set_user_data(indev, (&sample as *const Sample).cast_mut().cast());
        rs_lvgl_sys::lv_indev_set_read_cb(indev, Some(read));
        for y in [35, 30, 25, 20, 15, 10, 5] {
            sample.y.set(y);
            rs_lvgl_sys::lv_tick_inc(20);
            rs_lvgl_sys::lv_indev_read(indev);
        }
        sample.pressed.set(false);
        rs_lvgl_sys::lv_indev_read(indev);
        rs_lvgl_sys::lv_indev_delete(indev);
    }
    ui.drain(&mut app, 64).unwrap();
    let actual = unsafe { rs_lvgl_sys::lv_obj_get_scroll_y(obj) };
    assert!(actual > 0, "native drag must move overflowing content");
    assert!(app.seen.contains(&ScrollOffset { x: 0, y: actual }));
}
