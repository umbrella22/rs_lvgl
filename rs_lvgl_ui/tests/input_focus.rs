mod common;

use rs_lvgl::input_device::{key, Keypad, KeypadInputData};
use rs_lvgl_ui::{App, Key, Node, Style, Theme, Ui, UiError};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

#[derive(Default)]
struct Form {
    ids: Vec<u64>,
    disabled: bool,
    hidden: bool,
    duplicate: bool,
    clicks: Vec<u64>,
}

impl App for Form {
    type Msg = u64;
    fn update(&mut self, msg: u64) {
        self.clicks.push(msg);
    }
    fn view(&self, _: &Theme) -> Node<u64> {
        let mut node = Node::column()
            .child(Node::label("not focusable"))
            .children(
                self.ids
                    .iter()
                    .map(|id| Node::button("activate").key(Key::Id(*id)).on_click(*id)),
            )
            .child(Node::button("excluded").focusable(false))
            .child(
                Node::button("disabled")
                    .key(Key::Named("disabled"))
                    .style(Style::default().disabled(self.disabled)),
            )
            .child(
                Node::column()
                    .style(Style::default().hidden(self.hidden))
                    .child(Node::button("nested").key(Key::Named("nested"))),
            );
        if self.duplicate {
            node = node.child(Node::column().child(Node::button("duplicate").key(Key::Id(1))));
        }
        node
    }
}

fn keypad(display: &rs_lvgl::Display) -> (Keypad, Rc<RefCell<VecDeque<KeypadInputData>>>) {
    let samples = Rc::new(RefCell::new(VecDeque::new()));
    let reader = samples.clone();
    let mut last = KeypadInputData::released(0);
    let keypad = Keypad::register(
        move || {
            if let Some(sample) = reader.borrow_mut().pop_front() {
                last = sample;
            }
            last
        },
        display,
    )
    .unwrap();
    (keypad, samples)
}

fn send(device: &Keypad, samples: &RefCell<VecDeque<KeypadInputData>>, key: u32) {
    for sample in [
        KeypadInputData::pressed(key),
        KeypadInputData::released(key),
    ] {
        samples.borrow_mut().push_back(sample);
        unsafe { rs_lvgl_sys::lv_indev_read(device.raw().unwrap().as_ptr()) };
    }
}

#[test]
fn keypad_navigates_and_activates_then_group_tracks_reorder_and_deletion() {
    let _lock = common::lock();
    let display = common::init();
    let global = unsafe { rs_lvgl_sys::lv_group_create() };
    unsafe { rs_lvgl_sys::lv_group_set_default(global) };
    let (device, samples) = keypad(&display);
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Form {
        ids: vec![1, 2, 3],
        disabled: true,
        hidden: true,
        ..Default::default()
    };
    ui.mount(&app).unwrap();
    ui.bind_keypad(&device).unwrap();
    let group = unsafe { rs_lvgl_sys::lv_indev_get_group(device.raw().unwrap().as_ptr()) };
    assert_ne!(group, global);
    assert_eq!(unsafe { rs_lvgl_sys::lv_group_get_default() }, global);
    assert_eq!(unsafe { rs_lvgl_sys::lv_group_get_obj_count(group) }, 3);
    let first = common::child(common::app_root(), 1);
    let second = common::child(common::app_root(), 2);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_group_get_focused(group) },
        first.as_ptr()
    );
    send(&device, &samples, key::NEXT);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_group_get_focused(group) },
        second.as_ptr()
    );
    send(&device, &samples, key::ENTER);
    assert!(app.clicks.is_empty(), "native callbacks must only enqueue");
    assert_eq!(ui.drain(&mut app, 16).unwrap().processed, 1);
    assert_eq!(app.clicks, vec![2]);
    ui.set_editing(true).unwrap();
    app.ids = vec![2, 3, 1];
    ui.update(&app).unwrap();
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_group_get_obj_by_index(group, 0) },
        second.as_ptr()
    );
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_group_get_focused(group) },
        second.as_ptr()
    );
    assert!(unsafe { rs_lvgl_sys::lv_group_get_editing(group) });
    ui.update(&app).unwrap();
    assert!(unsafe { rs_lvgl_sys::lv_group_get_editing(group) });
    assert!(!ui.focus(Key::Named("disabled")).unwrap());
    assert!(!ui.focus(Key::Named("nested")).unwrap());
    assert!(!ui.focus(Key::Id(100)).unwrap());
    assert_eq!(ui.focus(Key::Auto), Err(UiError::InvalidProperty));
    app.duplicate = true;
    ui.update(&app).unwrap();
    assert_eq!(ui.focus(Key::Id(1)), Err(UiError::InvalidProperty));
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_group_get_focused(group) },
        second.as_ptr()
    );
    app.duplicate = false;
    app.ids = vec![3, 1];
    app.hidden = false;
    ui.update(&app).unwrap();
    assert!(ui.focus(Key::Named("nested")).unwrap());
    for _ in 0..100 {
        ui.unmount();
        assert!(
            unsafe { rs_lvgl_sys::lv_indev_get_group(device.raw().unwrap().as_ptr()) }.is_null()
        );
        ui.mount(&app).unwrap();
        ui.bind_keypad(&device).unwrap();
    }
    drop(ui);
    assert!(unsafe { rs_lvgl_sys::lv_indev_get_group(device.raw().unwrap().as_ptr()) }.is_null());
    unsafe {
        rs_lvgl_sys::lv_group_set_default(core::ptr::null_mut());
        rs_lvgl_sys::lv_group_delete(global);
    }
}

#[test]
fn foreign_display_and_deleted_keypad_are_rejected() {
    let _lock = common::lock();
    let display = common::init();
    let mut ui = Ui::<()>::new(&display, common::theme()).unwrap();
    let other = rs_lvgl::Display::register::<_, 64>(8, 8, |_| {}).unwrap();
    let (device, _) = keypad(&other);
    assert_eq!(ui.bind_keypad(&device), Err(UiError::InvalidProperty));
    unsafe { rs_lvgl_sys::lv_indev_delete(device.raw().unwrap().as_ptr()) };
    assert_eq!(ui.bind_keypad(&device), Err(UiError::InvalidProperty));
}

#[test]
fn focus_activation_is_explicit_and_unmount_releases_native_state() {
    let _lock = common::lock();
    let display = common::init();
    struct Screen {
        focused: usize,
    }
    impl App for Screen {
        type Msg = ();
        fn update(&mut self, _: ()) {
            self.focused += 1;
        }
        fn view(&self, _: &Theme) -> Node<()> {
            Node::label("explicit label focus")
                .key(Key::Named("label"))
                .focusable(true)
                .on_event(|e| (*e == rs_lvgl_ui::Event::Focused).then_some(()))
        }
        fn overlay(&self, _: &Theme) -> Option<Node<()>> {
            Some(Node::button("overlay").key(Key::Named("overlay")))
        }
    }
    let mut app = Screen { focused: 0 };
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    ui.mount(&app).unwrap();
    assert_eq!(ui.drain(&mut app, 16).unwrap().processed, 0);
    ui.enable_focus().unwrap();
    assert_eq!(ui.drain(&mut app, 16).unwrap().processed, 1);
    assert_eq!(app.focused, 1);
    ui.update(&app).unwrap();
    assert_eq!(ui.drain(&mut app, 16).unwrap().processed, 0);
    assert!(ui.focus(Key::Named("overlay")).unwrap());
    assert!(ui.focus(Key::Named("label")).unwrap());
    ui.unmount();
    // The established unmount contract permits native teardown before the
    // inert Rust Ui is dropped. An allocated empty group would violate it.
    unsafe { rs_lvgl::deinit() };
    rs_lvgl::init();
    drop(ui);
}

#[test]
fn failed_subtree_reconcile_removes_group_members_and_can_retry() {
    let _lock = common::lock();
    let display = common::init();
    struct Fallible(bool);
    impl App for Fallible {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            let node = Node::column().child(Node::button("live").key(Key::Id(1)));
            if self.0 {
                // SAFETY: the factory returns failure without creating or
                // retaining any native object.
                node.child(unsafe { Node::raw(|_| None) }.key(Key::Id(2)))
            } else {
                node
            }
        }
    }
    let (device, _) = keypad(&display);
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    ui.mount(&Fallible(false)).unwrap();
    ui.bind_keypad(&device).unwrap();
    let group = unsafe { rs_lvgl_sys::lv_indev_get_group(device.raw().unwrap().as_ptr()) };
    assert_eq!(ui.update(&Fallible(true)), Err(UiError::Allocation));
    assert_eq!(unsafe { rs_lvgl_sys::lv_group_get_obj_count(group) }, 0);
    assert_eq!(ui.stats().mounted_nodes, 0);
    assert_eq!(
        ui.stats().created_nodes,
        ui.stats().deleted_nodes,
        "failed reconciliation must count each deleted node once"
    );
    ui.update(&Fallible(false)).unwrap();
    assert!(ui.focus(Key::Id(1)).unwrap());
    assert_eq!(unsafe { rs_lvgl_sys::lv_group_get_obj_count(group) }, 1);
}
