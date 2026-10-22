//! Regression tests for the issues found in the rs_lvgl_ui review.
//!
//! Each test pins one documented contract: style merging and inheritance,
//! identity on key changes, controlled values, idle drains, keyboard binding
//! lifetime, combined handlers, handler-less nodes, bubbling, and custom widget
//! validation.

mod common;

use core::ptr::{self, NonNull};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use common::{app_root, child, init, lock};
use rs_lvgl::Widget;
use rs_lvgl_sys::lv_obj_t;
use rs_lvgl_ui::{
    App, Color, CustomObject, CustomWidget, Event, Font, Key, NativeObject, Node, Obj, Part, Style,
    Theme, Ui, UiError,
};

const MAIN: u32 = rs_lvgl_sys::lv_part_t_LV_PART_MAIN;

fn send(obj: NonNull<lv_obj_t>, code: rs_lvgl_sys::lv_event_code_t) {
    // SAFETY: tests only send events to live mounted objects.
    unsafe { rs_lvgl_sys::lv_obj_send_event(obj.as_ptr(), code, ptr::null_mut()) };
}

fn has_flag(obj: NonNull<lv_obj_t>, flag: rs_lvgl_sys::lv_obj_flag_t) -> bool {
    // SAFETY: tests only query live mounted objects.
    unsafe { rs_lvgl_sys::lv_obj_has_flag(obj.as_ptr(), flag) }
}

/// A view driven by a closure over a `u8` state.
struct FnApp<M: Clone + 'static> {
    state: u8,
    view: fn(u8, &Theme) -> Node<M>,
    received: Vec<M>,
}

impl<M: Clone + 'static> FnApp<M> {
    fn new(view: fn(u8, &Theme) -> Node<M>) -> Self {
        Self {
            state: 0,
            view,
            received: Vec::new(),
        }
    }
}

impl<M: Clone + 'static> App for FnApp<M> {
    type Msg = M;

    fn update(&mut self, msg: M) {
        self.received.push(msg);
    }

    fn view(&self, theme: &Theme) -> Node<M> {
        (self.view)(self.state, theme)
    }
}

#[test]
fn bg_opacity_merges_over_a_surface_color() {
    let _guard = lock();
    let display = init();
    let theme = Theme::dark();
    let mut ui = Ui::new(&display, theme).unwrap();
    let mut app = FnApp::<()>::new(|state, theme| match state {
        0 => Node::obj()
            .style(Style::surface(theme))
            .style(Style::default().bg_opacity(128)),
        _ => Node::obj().style(Style::default().bg(Color::rgb(255, 0, 0)).bg_opacity(64)),
    });
    ui.mount(&app).unwrap();
    let obj = app_root();
    // SAFETY: `obj` is the live application root.
    let (color, opa) = unsafe {
        (
            rs_lvgl_sys::lv_obj_get_style_bg_color(obj.as_ptr(), MAIN),
            rs_lvgl_sys::lv_obj_get_style_bg_opa(obj.as_ptr(), MAIN),
        )
    };
    assert_eq!(
        (color.red, color.green, color.blue),
        (theme.surface.r, theme.surface.g, theme.surface.b)
    );
    assert_eq!(opa, 128);

    app.state = 1;
    ui.update(&app).unwrap();
    // SAFETY: as above.
    let (color, opa) = unsafe {
        (
            rs_lvgl_sys::lv_obj_get_style_bg_color(obj.as_ptr(), MAIN),
            rs_lvgl_sys::lv_obj_get_style_bg_opa(obj.as_ptr(), MAIN),
        )
    };
    assert_eq!((color.red, color.green, color.blue), (255, 0, 0));
    assert_eq!(opa, 64);
}

#[test]
fn labels_inherit_text_color_and_font_from_containers() {
    let _guard = lock();
    let display = init();
    let theme = Theme::dark();
    let mut ui = Ui::new(&display, theme).unwrap();
    let app = FnApp::<()>::new(|_, theme| {
        Node::column()
            .style(Style::surface(theme).font(Font::montserrat_28()))
            .child(Node::label("x"))
    });
    ui.mount(&app).unwrap();
    let label = child(app_root(), 0);
    // SAFETY: `label` is live; the font symbol is a static LVGL font.
    unsafe {
        let color = rs_lvgl_sys::lv_obj_get_style_text_color(label.as_ptr(), MAIN);
        assert_eq!(
            (color.red, color.green, color.blue),
            (theme.text.r, theme.text.g, theme.text.b)
        );
        let font = rs_lvgl_sys::lv_obj_get_style_text_font(label.as_ptr(), MAIN);
        assert_eq!(font, ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_28));
    }
}

#[test]
fn root_key_change_recreates_the_root() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = FnApp::<()>::new(|state, _| Node::label("x").key(Key::Id(u64::from(state))));
    ui.mount(&app).unwrap();
    let before = app_root();
    let created = ui.stats().created_nodes;
    app.state = 1;
    ui.update(&app).unwrap();
    assert_ne!(before, app_root());
    assert_eq!(ui.stats().created_nodes, created + 1);
    assert_eq!(ui.stats().mounted_nodes, 1);
}

#[derive(Clone)]
enum ClampMsg {
    Set(i32),
}

struct ClampApp {
    value: i32,
}

impl App for ClampApp {
    type Msg = ClampMsg;

    fn update(&mut self, msg: ClampMsg) {
        let ClampMsg::Set(value) = msg;
        self.value = value.min(50);
    }

    fn view(&self, _: &Theme) -> Node<ClampMsg> {
        Node::column().child(Node::slider(0, 100, self.value).on_value_changed(ClampMsg::Set))
    }
}

#[test]
fn controlled_slider_follows_application_state() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = ClampApp { value: 50 };
    ui.mount(&app).unwrap();
    let slider = child(app_root(), 0);
    // SAFETY: `slider` is a live slider.
    unsafe { rs_lvgl_sys::lv_slider_set_value(slider.as_ptr(), 80, false) };
    send(slider, rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.value, 50);
    // SAFETY: as above.
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_slider_get_value(slider.as_ptr()) },
        50
    );
}

#[test]
fn idle_drains_skip_reconciliation_until_marked_dirty() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = ClampApp { value: 10 };
    ui.mount(&app).unwrap();
    let before = ui.stats().reconciled_updates;
    for _ in 0..10 {
        let stats = ui.drain(&mut app, 8).unwrap();
        assert_eq!(stats.processed, 0);
        assert!(!stats.reconciled);
    }
    assert_eq!(ui.stats().reconciled_updates, before);

    ui.mark_dirty();
    assert!(ui.drain(&mut app, 8).unwrap().reconciled);
    assert!(!ui.drain(&mut app, 8).unwrap().reconciled);

    ui.set_theme(Theme::dark());
    assert!(ui.drain(&mut app, 8).unwrap().reconciled);

    ui.post(ClampMsg::Set(20)).unwrap();
    let stats = ui.drain(&mut app, 8).unwrap();
    assert_eq!(stats.processed, 1);
    assert!(stats.reconciled);
    assert_eq!(ui.stats().reconciled_updates, before + 3);
}

static STATE_CHANGES: AtomicU32 = AtomicU32::new(0);

unsafe extern "C" fn count_state_changes(_: *mut rs_lvgl_sys::lv_event_t) {
    STATE_CHANGES.fetch_add(1, Ordering::SeqCst);
}

fn keyboard_view(state: u8, _: &Theme) -> Node<()> {
    let mut column = Node::column();
    if state == 0 {
        column = column.child(Node::textarea("a").keyboard_id(1).key(Key::Id(1)));
    }
    column.child(Node::keyboard(1).key(Key::Id(2)))
}

#[test]
fn unchanged_keyboard_binding_is_not_reapplied() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let app = FnApp::new(keyboard_view);
    ui.mount(&app).unwrap();
    let textarea = child(app_root(), 0);
    // SAFETY: `textarea` is live; the callback only counts.
    unsafe {
        rs_lvgl_sys::lv_obj_add_event_cb(
            textarea.as_ptr(),
            Some(count_state_changes),
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_STATE_CHANGED,
            ptr::null_mut(),
        );
    }
    STATE_CHANGES.store(0, Ordering::SeqCst);
    for _ in 0..3 {
        ui.update(&app).unwrap();
    }
    assert_eq!(STATE_CHANGES.load(Ordering::SeqCst), 0);
}

#[test]
fn removing_a_bound_textarea_unbinds_the_keyboard_first() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = FnApp::new(keyboard_view);
    ui.mount(&app).unwrap();
    let textarea = child(app_root(), 0);
    let keyboard = child(app_root(), 1);
    // SAFETY: both objects are live.
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_keyboard_get_textarea(keyboard.as_ptr()) },
        textarea.as_ptr()
    );

    app.state = 1;
    ui.update(&app).unwrap();
    assert_eq!(child(app_root(), 0), keyboard);
    // SAFETY: the keyboard stayed mounted.
    assert!(unsafe { rs_lvgl_sys::lv_keyboard_get_textarea(keyboard.as_ptr()) }.is_null());
    // A further render must not touch the deleted textarea (ASan run).
    ui.update(&app).unwrap();

    app.state = 0;
    ui.update(&app).unwrap();
    let textarea = child(app_root(), 0);
    // SAFETY: both objects are live.
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_keyboard_get_textarea(keyboard.as_ptr()) },
        textarea.as_ptr()
    );
}

#[derive(Clone, Debug, PartialEq)]
enum TwoMsg {
    Click,
    Pressed,
    Value(i32),
}

#[test]
fn handlers_combine_instead_of_replacing_each_other() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = FnApp::new(|_, _| {
        Node::column().child(
            Node::button("b")
                .on_click(TwoMsg::Click)
                .on_event(|event| matches!(event, Event::Pressed).then_some(TwoMsg::Pressed)),
        )
    });
    ui.mount(&app).unwrap();
    let button = child(app_root(), 0);
    send(button, rs_lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED);
    send(button, rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.received, vec![TwoMsg::Pressed, TwoMsg::Click]);
}

#[test]
fn handlers_added_on_update_attach_lazily_without_duplicates() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = FnApp::new(|state, _| {
        let slider = Node::slider(0, 10, 5);
        Node::column().child(match state {
            0 => slider,
            1 => slider.on_click(TwoMsg::Click),
            _ => slider
                .on_click(TwoMsg::Click)
                .on_value_changed(TwoMsg::Value)
                .on_event(|event| matches!(event, Event::Pressed).then_some(TwoMsg::Pressed)),
        })
    });
    ui.mount(&app).unwrap();
    let slider = child(app_root(), 0);
    // SAFETY: `slider` is live.
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_obj_get_event_count(slider.as_ptr()) },
        0
    );

    app.state = 1;
    ui.update(&app).unwrap();
    assert_eq!(child(app_root(), 0), slider);
    send(slider, rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.received, vec![TwoMsg::Click]);

    app.state = 2;
    app.received.clear();
    ui.update(&app).unwrap();
    send(slider, rs_lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED);
    send(slider, rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED);
    send(slider, rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(
        app.received,
        vec![TwoMsg::Pressed, TwoMsg::Click, TwoMsg::Value(5)]
    );
}

#[test]
fn handlerless_nodes_have_no_callbacks_and_keep_class_flags() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = FnApp::<()>::new(|state, _| match state {
        0 => Node::column().child(Node::label("no handlers")),
        _ => Node::column()
            .clickable(false)
            .scrollable(false)
            .child(Node::label("no handlers").clickable(true)),
    });
    ui.mount(&app).unwrap();
    let column = app_root();
    let label = child(column, 0);
    // SAFETY: both objects are live.
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_obj_get_event_count(label.as_ptr()) },
        0
    );
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_obj_get_event_count(column.as_ptr()) },
        0
    );
    let clickable = rs_lvgl_sys::lv_obj_flag_t_LV_OBJ_FLAG_CLICKABLE;
    let scrollable = rs_lvgl_sys::lv_obj_flag_t_LV_OBJ_FLAG_SCROLLABLE;
    assert!(has_flag(column, clickable));
    assert!(has_flag(column, scrollable));
    assert!(!has_flag(label, clickable));

    app.state = 1;
    ui.update(&app).unwrap();
    assert!(!has_flag(column, clickable));
    assert!(!has_flag(column, scrollable));
    assert!(has_flag(label, clickable));
}

#[test]
fn bubbled_child_events_do_not_reach_parent_handlers() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = FnApp::new(|_, _| {
        Node::column()
            .on_event(|event| matches!(event, Event::Pressed).then_some(TwoMsg::Pressed))
            .child(Node::button("child"))
    });
    ui.mount(&app).unwrap();
    let button = child(app_root(), 0);
    // SAFETY: `button` is live; bubbling forwards its events to the column.
    unsafe {
        rs_lvgl_sys::lv_obj_add_flag(
            button.as_ptr(),
            rs_lvgl_sys::lv_obj_flag_t_LV_OBJ_FLAG_EVENT_BUBBLE,
        )
    };
    send(button, rs_lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED);
    ui.drain(&mut app, 8).unwrap();
    assert!(app.received.is_empty());

    send(app_root(), rs_lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.received, vec![TwoMsg::Pressed]);
}

#[test]
fn tabview_active_tab_is_controlled() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let app = FnApp::<()>::new(|_, _| {
        Node::tabview(["a", "b"], 0).children([Node::label("a"), Node::label("b")])
    });
    ui.mount(&app).unwrap();
    let tabview = app_root();
    // SAFETY: `tabview` is live.
    unsafe {
        rs_lvgl_sys::lv_tabview_set_active(tabview.as_ptr(), 1, false);
        assert_eq!(rs_lvgl_sys::lv_tabview_get_tab_active(tabview.as_ptr()), 1);
    }
    ui.update(&app).unwrap();
    // SAFETY: as above.
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_tabview_get_tab_active(tabview.as_ptr()) },
        0
    );
}

/// A lifecycle widget that returns a grandchild instead of a direct child.
struct WrongParent {
    deleted: Rc<Cell<bool>>,
}

impl CustomWidget for WrongParent {
    fn create(&mut self, parent: &mut Obj<'_>) -> Result<CustomObject, UiError> {
        // SAFETY: `parent` is live; both objects are created as its subtree.
        unsafe {
            let wrapper = rs_lvgl_sys::lv_obj_create(parent.raw().as_ptr());
            let inner = rs_lvgl_sys::lv_obj_create(wrapper);
            let inner = NonNull::new(inner).ok_or(UiError::Allocation)?;
            let wrapper = Obj::from_raw(NonNull::new(wrapper).ok_or(UiError::Allocation)?)
                .ok_or(UiError::Allocation)?;
            // Valid against `wrapper`, but not a direct child of `parent`.
            CustomObject::new(&wrapper, inner)
        }
    }

    fn update(&mut self, _: &mut CustomObject, _: u32) -> Result<(), UiError> {
        Ok(())
    }

    fn delete(&mut self, _: &mut CustomObject) {
        self.deleted.set(true);
    }
}

#[test]
fn lifecycle_widget_must_return_a_direct_child() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let deleted = Rc::new(Cell::new(false));
    struct CustomApp(Rc<Cell<bool>>);
    impl App for CustomApp {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            Node::column().child(Node::custom(WrongParent {
                deleted: self.0.clone(),
            }))
        }
    }
    let result = ui.mount(&CustomApp(deleted.clone()));
    assert_eq!(result, Err(UiError::InvalidProperty));
    assert!(!deleted.get(), "delete runs only after a successful create");
}

#[test]
fn validation_covers_keyboard_ids_and_part_styles() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let duplicate = FnApp::<()>::new(|_, _| {
        Node::column()
            .child(Node::column().child(Node::textarea("a").keyboard_id(7)))
            .child(Node::textarea("b").keyboard_id(7))
    });
    assert_eq!(ui.mount(&duplicate), Err(UiError::InvalidProperty));

    let bad_part = FnApp::<()>::new(|_, _| {
        Node::column()
            .child(Node::slider(0, 10, 5).part_style(Part::Knob, Style::default().radius(-1)))
    });
    assert_eq!(ui.mount(&bad_part), Err(UiError::InvalidProperty));
}
