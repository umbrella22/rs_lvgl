mod common;

use core::ptr;
use std::ptr::NonNull;

use common::{app_root, child, init, lock, screen};
use rs_lvgl_ui::{
    App, ChartSeries, ChartType, Event, GridTrack, ImageSource, Key, Node, Part, Size, Style,
    TextAlign, Theme, Ui, UiError,
};

#[derive(Clone)]
struct TreeApp {
    kind: u8,
    text: &'static str,
}

impl App for TreeApp {
    type Msg = ();

    fn update(&mut self, _: ()) {}

    fn view(&self, _: &Theme) -> Node<()> {
        Node::column()
            .child(Node::label(self.text).key(Key::Named("first")))
            .child(if self.kind == 0 {
                Node::label("second").key(Key::Id(1))
            } else {
                Node::slider(0, 10, 5).key(Key::Id(1))
            })
    }
}

#[test]
fn mount_update_kind_change_and_unmount() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = TreeApp {
        kind: 0,
        text: "first",
    };

    ui.mount(&app).unwrap();
    assert_eq!(ui.stats().mounted_nodes, 3);
    assert_eq!(ui.stats().created_nodes, 3);
    assert_eq!(ui.stats().deleted_nodes, 0);

    app.text = "changed";
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 3);
    assert_eq!(ui.stats().deleted_nodes, 0);

    app.kind = 1;
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().mounted_nodes, 3);
    assert_eq!(ui.stats().created_nodes, 4);
    assert_eq!(ui.stats().deleted_nodes, 1);

    ui.unmount();
    assert_eq!(ui.stats().mounted_nodes, 0);
    assert_eq!(ui.stats().deleted_nodes, 4);
}

#[derive(Clone)]
struct ListApp {
    items: Vec<(u64, u32)>,
}

fn list_app() -> ListApp {
    ListApp {
        items: (0..100).map(|value| (value as u64, value)).collect(),
    }
}

impl App for ListApp {
    type Msg = ();

    fn update(&mut self, _: ()) {}

    fn view(&self, _: &Theme) -> Node<()> {
        Node::column().children(
            self.items
                .iter()
                .map(|(id, value)| Node::label(format!("{value}")).key(Key::Id(*id))),
        )
    }
}

#[test]
fn keyed_list_single_value_update_creates_nothing() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = list_app();

    ui.mount(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 101);
    assert_eq!(ui.stats().mounted_nodes, 101);

    app.items[37].1 = 999;
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 101);
    assert_eq!(ui.stats().deleted_nodes, 0);
    assert_eq!(ui.stats().moved_nodes, 0);
}

#[test]
fn keyed_list_reorder_moves_two_objects() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = list_app();
    ui.mount(&app).unwrap();

    let first = app.items.remove(0);
    let last = app.items.pop().unwrap();
    app.items.insert(0, last);
    app.items.push(first);

    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 101);
    assert_eq!(ui.stats().deleted_nodes, 0);
    assert_eq!(ui.stats().moved_nodes, 2);
}

#[test]
fn keyed_list_append_and_remove_touch_only_affected_nodes() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = list_app();
    ui.mount(&app).unwrap();

    app.items.remove(10);
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 101);
    assert_eq!(ui.stats().deleted_nodes, 1);
    assert_eq!(ui.stats().moved_nodes, 0);

    app.items.push((500, 500));
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 102);
    assert_eq!(ui.stats().deleted_nodes, 1);
    assert_eq!(ui.stats().moved_nodes, 0);
}

#[test]
fn auto_and_keyed_siblings_mix() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    struct MixedApp {
        suffix: &'static str,
    }
    impl App for MixedApp {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            Node::column()
                .child(Node::label(format!("auto-a{}", self.suffix)))
                .child(Node::label(format!("keyed{}", self.suffix)).key(Key::Id(1)))
                .child(Node::label(format!("auto-b{}", self.suffix)))
        }
    }

    let mut app = MixedApp { suffix: "" };
    ui.mount(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 4);

    app.suffix = "!";
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 4);
    assert_eq!(ui.stats().deleted_nodes, 0);
    assert_eq!(ui.stats().moved_nodes, 0);
}

#[derive(Clone, Debug, PartialEq)]
enum SpeedMsg {
    Speed(i32),
}

struct SliderApp {
    speed: i32,
}

impl App for SliderApp {
    type Msg = SpeedMsg;

    fn update(&mut self, msg: SpeedMsg) {
        match msg {
            SpeedMsg::Speed(value) => self.speed = value,
        }
    }

    fn view(&self, _: &Theme) -> Node<SpeedMsg> {
        Node::column().child(
            Node::slider(0, 100, self.speed)
                .key(Key::Named("speed"))
                .on_value_changed(SpeedMsg::Speed),
        )
    }
}

#[test]
fn slider_events_enqueue_and_programmatic_writes_are_suppressed() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = SliderApp { speed: 10 };
    ui.mount(&app).unwrap();

    let slider = child(app_root(), 0);
    // LVGL 9 emits VALUE_CHANGED from its input handling, not from
    // `lv_slider_set_value`; emulate the native user event after the value
    // write.
    unsafe {
        rs_lvgl_sys::lv_slider_set_value(slider.as_ptr(), 42, false);
        rs_lvgl_sys::lv_obj_send_event(
            slider.as_ptr(),
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            ptr::null_mut(),
        );
    }
    let drained = ui.drain(&mut app, 8).unwrap();
    assert_eq!(drained.processed, 1);
    assert_eq!(app.speed, 42);

    // The reconciler writes the view value. That write must not feed back into
    // the queue.
    app.speed = 70;
    ui.update(&app).unwrap();
    let drained = ui.drain(&mut app, 8).unwrap();
    assert_eq!(drained.processed, 0);
    assert_eq!(app.speed, 70);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_slider_get_value(slider.as_ptr()) },
        70
    );
}

#[derive(Clone, Debug, PartialEq)]
enum ClickMsg {
    A,
    B,
}

struct ClickApp {
    version: u8,
    clicked: Option<ClickMsg>,
}

impl App for ClickApp {
    type Msg = ClickMsg;

    fn update(&mut self, msg: ClickMsg) {
        self.clicked = Some(msg);
    }

    fn view(&self, _: &Theme) -> Node<ClickMsg> {
        Node::column().child(Node::button("go").key(Key::Named("go")).on_click(
            if self.version == 0 {
                ClickMsg::A
            } else {
                ClickMsg::B
            },
        ))
    }
}

#[test]
fn handler_replacement_keeps_one_native_callback() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = ClickApp {
        version: 0,
        clicked: None,
    };
    ui.mount(&app).unwrap();

    let button = child(app_root(), 0);
    unsafe {
        rs_lvgl_sys::lv_obj_send_event(
            button.as_ptr(),
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED,
            ptr::null_mut(),
        )
    };
    let drained = ui.drain(&mut app, 8).unwrap();
    assert_eq!(drained.processed, 1);
    assert_eq!(app.clicked, Some(ClickMsg::A));

    app.version = 1;
    app.clicked = None;
    ui.update(&app).unwrap();
    unsafe {
        rs_lvgl_sys::lv_obj_send_event(
            button.as_ptr(),
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED,
            ptr::null_mut(),
        )
    };
    let drained = ui.drain(&mut app, 8).unwrap();
    assert_eq!(
        drained.processed, 1,
        "replacement must not duplicate callbacks"
    );
    assert_eq!(app.clicked, Some(ClickMsg::B));
}

#[test]
fn queue_overflow_and_drain_cap() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::with_capacity(&display, common::theme(), 2).unwrap();

    #[derive(Clone)]
    struct CounterApp {
        count: u32,
    }
    #[derive(Clone)]
    struct Inc;
    impl App for CounterApp {
        type Msg = Inc;

        fn update(&mut self, _: Inc) {
            self.count += 1;
        }

        fn view(&self, _: &Theme) -> Node<Inc> {
            Node::column().child(Node::label(format!("{}", self.count)))
        }
    }

    let mut app = CounterApp { count: 0 };
    ui.mount(&app).unwrap();
    for _ in 0..5 {
        ui.post(Inc).unwrap();
    }
    assert_eq!(ui.stats().messages_dropped, 3);

    let first = ui.drain(&mut app, 1).unwrap();
    assert_eq!(first.processed, 1);
    assert_eq!(app.count, 1);

    let second = ui.drain(&mut app, 10).unwrap();
    assert_eq!(second.processed, 1);
    assert_eq!(app.count, 2);
    assert_eq!(ui.stats().messages_processed, 2);
    assert_eq!(ui.stats().messages_dropped, 3);
}

#[test]
fn validation_rejects_bad_views_before_mutation() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    struct BadApp(u8);
    impl App for BadApp {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            match self.0 {
                0 => Node::column()
                    .child(Node::label("a").key(Key::Named("dup")))
                    .child(Node::label("b").key(Key::Named("dup"))),
                1 => Node::column().child(Node::slider(10, 0, 5).key(Key::Id(1))),
                2 => Node::column().child(Node::label("a\0b").key(Key::Id(1))),
                _ => Node::column().child(Node::label("parent").child(Node::label("child"))),
            }
        }
    }

    for (mode, expected) in [
        (0, UiError::DuplicateKey),
        (1, UiError::InvalidProperty),
        (2, UiError::InvalidProperty),
        (3, UiError::InvalidProperty),
    ] {
        let app = BadApp(mode);
        assert_eq!(ui.mount(&app).unwrap_err(), expected);
        assert_eq!(ui.stats().mounted_nodes, 0);
        assert_eq!(ui.stats().created_nodes, 0);
    }
}

#[test]
fn raw_subtree_is_opaque_and_deleted_with_its_parent() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    struct RawApp;
    impl App for RawApp {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            let raw: Node<()> = unsafe {
                Node::raw(|parent: NonNull<rs_lvgl_sys::lv_obj_t>| {
                    let obj = rs_lvgl_sys::lv_obj_create(parent.as_ptr());
                    let obj = NonNull::new(obj)?;
                    let label = rs_lvgl_sys::lv_label_create(obj.as_ptr());
                    if label.is_null() {
                        rs_lvgl_sys::lv_obj_delete(obj.as_ptr());
                        return None;
                    }
                    Some(obj)
                })
            };
            Node::column().child(raw.key(Key::Named("raw")))
        }
    }

    let app = RawApp;
    ui.mount(&app).unwrap();
    assert_eq!(ui.stats().mounted_nodes, 2);
    assert_eq!(ui.stats().created_nodes, 2);

    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 2);
    assert_eq!(ui.stats().deleted_nodes, 0);

    ui.unmount();
    assert_eq!(ui.stats().deleted_nodes, 2);
    assert_eq!(screen().as_ptr(), unsafe {
        rs_lvgl_sys::lv_screen_active()
    });
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_obj_get_child_count(screen().as_ptr()) },
        0
    );
}

#[test]
fn non_container_rejects_children() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    struct ChildApp;
    impl App for ChildApp {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            Node::column().child(Node::button("b").child(Node::label("x")))
        }
    }

    assert_eq!(ui.mount(&ChildApp).unwrap_err(), UiError::InvalidProperty);
    assert_eq!(ui.stats().created_nodes, 0);
}

#[test]
fn styled_nodes_keep_class_defaults_when_unset() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    struct StyledApp;
    impl App for StyledApp {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            Node::column().child(Node::slider(0, 100, 20).key(Key::Id(1)))
        }
    }

    ui.mount(&StyledApp).unwrap();
    let slider = child(app_root(), 0);
    // No local background was applied, so the slider's class background is
    // still visible.
    let bg_opa = unsafe {
        rs_lvgl_sys::lv_obj_get_style_bg_opa(slider.as_ptr(), rs_lvgl_sys::lv_part_t_LV_PART_MAIN)
    };
    assert!(bg_opa > 0);
}

#[test]
fn style_is_copyable_and_theme_helpers_resolve_tokens() {
    let theme = Theme::dark();
    let style = Style::title(&theme);
    assert_eq!(style.foreground(), theme.text);
    assert_eq!(style.text_font(), theme.title_font);
}

#[derive(Clone, Debug, PartialEq)]
enum SwitchMsg {
    State(i32),
}

struct SwitchApp {
    checked: bool,
}

impl App for SwitchApp {
    type Msg = SwitchMsg;

    fn update(&mut self, msg: SwitchMsg) {
        match msg {
            SwitchMsg::State(value) => self.checked = value != 0,
        }
    }

    fn view(&self, _: &Theme) -> Node<SwitchMsg> {
        Node::column().child(
            Node::switch(self.checked)
                .key(Key::Named("switch"))
                .on_value_changed(SwitchMsg::State),
        )
    }
}

#[test]
fn switch_state_is_written_and_user_events_enqueue() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = SwitchApp { checked: false };
    ui.mount(&app).unwrap();
    let switch = child(app_root(), 0);

    app.checked = true;
    ui.update(&app).unwrap();
    assert!(unsafe {
        rs_lvgl_sys::lv_obj_has_state(switch.as_ptr(), rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED)
    });
    assert_eq!(ui.drain(&mut app, 8).unwrap().processed, 0);

    // Emulate the native value event produced by an input release.
    unsafe {
        rs_lvgl_sys::lv_obj_remove_state(switch.as_ptr(), rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED);
        rs_lvgl_sys::lv_obj_send_event(
            switch.as_ptr(),
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            ptr::null_mut(),
        );
    }
    let drained = ui.drain(&mut app, 8).unwrap();
    assert_eq!(drained.processed, 1);
    assert!(!app.checked);
}

#[test]
fn image_source_and_scale_update_in_place() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    static PIXELS: [u8; 2] = [0x00, 0xf8];
    let mut descriptor = rs_lvgl_sys::lv_image_dsc_t::default();
    descriptor
        .header
        .set_magic(rs_lvgl_sys::LV_IMAGE_HEADER_MAGIC);
    descriptor
        .header
        .set_cf(rs_lvgl_sys::lv_color_format_t_LV_COLOR_FORMAT_RGB565);
    descriptor.header.set_w(1);
    descriptor.header.set_h(1);
    descriptor.header.set_stride(2);
    descriptor.data_size = 2;
    descriptor.data = PIXELS.as_ptr();
    let descriptor: &'static rs_lvgl_sys::lv_image_dsc_t = Box::leak(Box::new(descriptor));
    let source = unsafe { ImageSource::from_raw(descriptor) };

    struct ImageApp {
        source: ImageSource,
        scale: u16,
    }
    impl App for ImageApp {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            Node::column()
                .child(Node::image_scaled(self.source, self.scale).key(Key::Named("image")))
        }
    }

    let mut app = ImageApp { source, scale: 256 };
    ui.mount(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 2);

    app.scale = 512;
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 2);
    assert_eq!(ui.stats().deleted_nodes, 0);
}

#[test]
fn gallery_message_flow_updates_state_and_reconciles() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    #[derive(Clone)]
    enum Msg {
        Inc,
        Speed(i32),
        ToggleMute,
        MoveDown(u64),
        Add,
    }

    struct Gallery {
        count: i32,
        speed: i32,
        muted: bool,
        items: Vec<u64>,
        next: u64,
    }

    impl App for Gallery {
        type Msg = Msg;

        fn update(&mut self, msg: Msg) {
            match msg {
                Msg::Inc => self.count += 1,
                Msg::Speed(value) => self.speed = value,
                Msg::ToggleMute => self.muted = !self.muted,
                Msg::MoveDown(id) => {
                    if let Some(index) = self.items.iter().position(|item| *item == id) {
                        let item = self.items.remove(index);
                        let target = if index + 1 < self.items.len() {
                            index + 1
                        } else {
                            0
                        };
                        self.items.insert(target, item);
                    }
                }
                Msg::Add => {
                    self.items.push(self.next);
                    self.next += 1;
                }
            }
        }

        fn view(&self, theme: &Theme) -> Node<Msg> {
            Node::column()
                .style(Style::screen(theme))
                .children(self.items.iter().map(|id| {
                    Node::row()
                        .key(Key::Id(*id))
                        .child(Node::label(format!("{id}")).style(Style::default().flex_grow(1)))
                        .child(Node::button("down").on_click(Msg::MoveDown(*id)))
                }))
        }
    }

    let mut app = Gallery {
        count: 0,
        speed: 35,
        muted: false,
        items: vec![1, 2, 3],
        next: 4,
    };
    ui.mount(&app).unwrap();

    ui.post(Msg::Inc).unwrap();
    ui.post(Msg::Speed(80)).unwrap();
    ui.post(Msg::ToggleMute).unwrap();
    let drained = ui.drain(&mut app, 8).unwrap();
    assert_eq!(drained.processed, 3);
    assert_eq!(app.count, 1);
    assert_eq!(app.speed, 80);
    assert!(app.muted);

    let created_before = ui.stats().created_nodes;
    ui.post(Msg::Add).unwrap();
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(ui.stats().created_nodes, created_before + 3);

    let moved_before = ui.stats().moved_nodes;
    ui.post(Msg::MoveDown(1)).unwrap();
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.items, vec![2, 1, 3, 4]);
    assert_eq!(ui.stats().moved_nodes, moved_before + 1);
    assert_eq!(ui.stats().created_nodes, created_before + 3);
}

#[derive(Clone, Debug, PartialEq)]
enum CheckMsg {
    Checked(bool),
}

struct CheckApp {
    checked: bool,
}

impl App for CheckApp {
    type Msg = CheckMsg;

    fn update(&mut self, msg: CheckMsg) {
        match msg {
            CheckMsg::Checked(value) => self.checked = value,
        }
    }

    fn view(&self, theme: &Theme) -> Node<CheckMsg> {
        Node::column().child(
            Node::checkbox("Notify", self.checked)
                .key(Key::Named("check"))
                .style(Style::default().fg(theme.text))
                .on_value_changed(|value| CheckMsg::Checked(value != 0)),
        )
    }
}

#[test]
fn checkbox_state_and_value_events() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = CheckApp { checked: false };
    ui.mount(&app).unwrap();
    let checkbox = child(app_root(), 0);

    unsafe {
        rs_lvgl_sys::lv_obj_add_state(checkbox.as_ptr(), rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED);
        rs_lvgl_sys::lv_obj_send_event(
            checkbox.as_ptr(),
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            ptr::null_mut(),
        );
    }
    let drained = ui.drain(&mut app, 8).unwrap();
    assert_eq!(drained.processed, 1);
    assert!(app.checked);

    app.checked = false;
    ui.update(&app).unwrap();
    assert!(!unsafe {
        rs_lvgl_sys::lv_obj_has_state(checkbox.as_ptr(), rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED)
    });
    assert_eq!(ui.drain(&mut app, 8).unwrap().processed, 0);
}

#[derive(Clone, Debug, PartialEq)]
enum TextMsg {
    Text(String),
}

struct TextApp {
    text: String,
}

impl App for TextApp {
    type Msg = TextMsg;

    fn update(&mut self, msg: TextMsg) {
        match msg {
            TextMsg::Text(text) => self.text = text,
        }
    }

    fn view(&self, _: &Theme) -> Node<TextMsg> {
        Node::column().child(
            Node::textarea(self.text.as_str())
                .placeholder("Your name")
                .key(Key::Named("text"))
                .on_text_changed(|text| TextMsg::Text(text.into())),
        )
    }
}

#[test]
fn textarea_text_events_and_programmatic_suppression() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = TextApp {
        text: String::new(),
    };
    ui.mount(&app).unwrap();
    let textarea = child(app_root(), 0);
    let initial = ui.drain(&mut app, 8).unwrap();
    assert_eq!(initial.processed, 0, "mount must not enqueue a text event");

    let user_text = std::ffi::CString::new("hello").unwrap();
    unsafe { rs_lvgl_sys::lv_textarea_set_text(textarea.as_ptr(), user_text.as_ptr()) };
    let drained = ui.drain(&mut app, 8).unwrap();
    assert_eq!(drained.processed, 1);
    assert_eq!(app.text, "hello");

    app.text = "world".into();
    ui.update(&app).unwrap();
    let native = unsafe { rs_lvgl_sys::lv_textarea_get_text(textarea.as_ptr()) };
    assert!(!native.is_null());
    let native = unsafe { std::ffi::CStr::from_ptr(native) };
    assert_eq!(native.to_str().unwrap(), "world");
    assert_eq!(ui.drain(&mut app, 8).unwrap().processed, 0);
}

#[derive(Clone, Debug, PartialEq)]
enum DropMsg {
    Selected(i32),
}

struct DropApp {
    selected: u32,
}

impl App for DropApp {
    type Msg = DropMsg;

    fn update(&mut self, msg: DropMsg) {
        match msg {
            DropMsg::Selected(value) => self.selected = value as u32,
        }
    }

    fn view(&self, _: &Theme) -> Node<DropMsg> {
        Node::column().child(
            Node::dropdown(["Male", "Female", "Other"], self.selected)
                .key(Key::Named("dropdown"))
                .on_value_changed(DropMsg::Selected),
        )
    }
}

#[test]
fn dropdown_selection_events_and_programmatic_updates() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = DropApp { selected: 0 };
    ui.mount(&app).unwrap();
    let dropdown = child(app_root(), 0);

    unsafe {
        rs_lvgl_sys::lv_dropdown_set_selected(dropdown.as_ptr(), 2);
        rs_lvgl_sys::lv_obj_send_event(
            dropdown.as_ptr(),
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            ptr::null_mut(),
        );
    }
    let drained = ui.drain(&mut app, 8).unwrap();
    assert_eq!(drained.processed, 1);
    assert_eq!(app.selected, 2);

    app.selected = 1;
    ui.update(&app).unwrap();
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_dropdown_get_selected(dropdown.as_ptr()) },
        1
    );
    assert_eq!(ui.drain(&mut app, 8).unwrap().processed, 0);
}

#[derive(Clone, Debug, PartialEq)]
enum ArcMsg {
    Value(i32),
}

struct ArcApp {
    value: i32,
}

impl App for ArcApp {
    type Msg = ArcMsg;

    fn update(&mut self, msg: ArcMsg) {
        match msg {
            ArcMsg::Value(value) => self.value = value,
        }
    }

    fn view(&self, theme: &Theme) -> Node<ArcMsg> {
        Node::column().child(
            Node::arc(0, 100, self.value)
                .key(Key::Named("arc"))
                .style(
                    Style::default()
                        .width(Size::Px(100))
                        .height(Size::Px(100))
                        .arc_color(theme.accent)
                        .arc_width(12),
                )
                .on_value_changed(ArcMsg::Value),
        )
    }
}

#[test]
fn arc_value_events_and_part_style() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = ArcApp { value: 20 };
    ui.mount(&app).unwrap();
    let arc = child(app_root(), 0);

    assert_eq!(
        unsafe {
            rs_lvgl_sys::lv_obj_get_style_arc_width(
                arc.as_ptr(),
                rs_lvgl_sys::lv_part_t_LV_PART_INDICATOR,
            )
        },
        12
    );
    assert_eq!(
        unsafe {
            rs_lvgl_sys::lv_obj_get_style_arc_opa(arc.as_ptr(), rs_lvgl_sys::lv_part_t_LV_PART_MAIN)
        },
        0
    );

    unsafe {
        rs_lvgl_sys::lv_arc_set_value(arc.as_ptr(), 73);
        rs_lvgl_sys::lv_obj_send_event(
            arc.as_ptr(),
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            ptr::null_mut(),
        );
    }
    let drained = ui.drain(&mut app, 8).unwrap();
    assert_eq!(drained.processed, 1);
    assert_eq!(app.value, 73);

    app.value = 31;
    ui.update(&app).unwrap();
    assert_eq!(unsafe { rs_lvgl_sys::lv_arc_get_value(arc.as_ptr()) }, 31);
    assert_eq!(ui.drain(&mut app, 8).unwrap().processed, 0);
}

#[test]
fn scale_properties_update_in_place() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    struct ScaleApp {
        ticks: u32,
        major: u32,
    }
    impl App for ScaleApp {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            Node::column().child(
                Node::scale(0, 100, self.ticks, self.major)
                    .key(Key::Named("scale"))
                    .style(
                        Style::default()
                            .width(Size::Percent(100))
                            .height(Size::Px(60)),
                    ),
            )
        }
    }

    let mut app = ScaleApp {
        ticks: 11,
        major: 5,
    };
    ui.mount(&app).unwrap();
    let scale = child(app_root(), 0);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_scale_get_total_tick_count(scale.as_ptr()) },
        11
    );
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_scale_get_major_tick_every(scale.as_ptr()) },
        5
    );

    app.ticks = 21;
    app.major = 10;
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 2);
    assert_eq!(ui.stats().deleted_nodes, 0);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_scale_get_total_tick_count(scale.as_ptr()) },
        21
    );
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_scale_get_major_tick_every(scale.as_ptr()) },
        10
    );
}

#[test]
fn style_sides_alignment_bounds_and_states() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    struct StyleApp {
        hidden: bool,
        disabled: bool,
    }
    impl App for StyleApp {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            Node::column().child(
                Node::obj().key(Key::Named("box")).style(
                    Style::default()
                        .width(Size::Px(50))
                        .height(Size::Px(50))
                        .pad_left(4)
                        .pad_right(8)
                        .pad_top(2)
                        .pad_bottom(6)
                        .text_align(TextAlign::Center)
                        .max_width(Size::Px(100))
                        .min_height(Size::Px(10))
                        .hidden(self.hidden)
                        .disabled(self.disabled),
                ),
            )
        }
    }

    let mut app = StyleApp {
        hidden: false,
        disabled: true,
    };
    ui.mount(&app).unwrap();
    let object = child(app_root(), 0);
    let left = unsafe {
        rs_lvgl_sys::lv_obj_get_style_pad_left(object.as_ptr(), rs_lvgl_sys::lv_part_t_LV_PART_MAIN)
    };
    let right = unsafe {
        rs_lvgl_sys::lv_obj_get_style_pad_right(
            object.as_ptr(),
            rs_lvgl_sys::lv_part_t_LV_PART_MAIN,
        )
    };
    let top = unsafe {
        rs_lvgl_sys::lv_obj_get_style_pad_top(object.as_ptr(), rs_lvgl_sys::lv_part_t_LV_PART_MAIN)
    };
    let bottom = unsafe {
        rs_lvgl_sys::lv_obj_get_style_pad_bottom(
            object.as_ptr(),
            rs_lvgl_sys::lv_part_t_LV_PART_MAIN,
        )
    };
    assert_eq!((left, right, top, bottom), (4, 8, 2, 6));
    assert_eq!(
        unsafe {
            rs_lvgl_sys::lv_obj_get_style_text_align(
                object.as_ptr(),
                rs_lvgl_sys::lv_part_t_LV_PART_MAIN,
            )
        },
        rs_lvgl_sys::lv_text_align_t_LV_TEXT_ALIGN_CENTER
    );
    assert_eq!(
        unsafe {
            rs_lvgl_sys::lv_obj_get_style_max_width(
                object.as_ptr(),
                rs_lvgl_sys::lv_part_t_LV_PART_MAIN,
            )
        },
        100
    );
    assert_eq!(
        unsafe {
            rs_lvgl_sys::lv_obj_get_style_min_height(
                object.as_ptr(),
                rs_lvgl_sys::lv_part_t_LV_PART_MAIN,
            )
        },
        10
    );
    assert!(unsafe {
        rs_lvgl_sys::lv_obj_has_state(object.as_ptr(), rs_lvgl_sys::lv_state_t_LV_STATE_DISABLED)
    });
    assert!(!unsafe { rs_lvgl_sys::lv_obj_is_hidden(object.as_ptr()) });

    app.hidden = true;
    app.disabled = false;
    ui.update(&app).unwrap();
    assert!(unsafe { rs_lvgl_sys::lv_obj_is_hidden(object.as_ptr()) });
    assert!(!unsafe {
        rs_lvgl_sys::lv_obj_has_state(object.as_ptr(), rs_lvgl_sys::lv_state_t_LV_STATE_DISABLED)
    });
}

fn theme_color() -> rs_lvgl_ui::Color {
    rs_lvgl_ui::Color::rgb(1, 2, 3)
}

#[test]
fn invalid_new_widget_properties_are_rejected() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    struct BadApp(u8);
    impl App for BadApp {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            match self.0 {
                0 => Node::column().child(Node::dropdown(Vec::<String>::new(), 0)),
                1 => Node::column().child(Node::dropdown(["a", "b"], 2)),
                2 => Node::column().child(Node::scale(0, 100, 1, 1)),
                3 => Node::column().child(Node::textarea("ok").on_text_changed(|_| ())),
                4 => Node::column().child(Node::label("x").on_text_changed(|_| ())),
                5 => Node::column().child(Node::chart(ChartType::Line, 0, 100, Vec::new())),
                _ => Node::column().child(Node::chart(
                    ChartType::Line,
                    0,
                    100,
                    vec![
                        ChartSeries::new(theme_color(), vec![1, 2]),
                        ChartSeries::new(theme_color(), vec![1]),
                    ],
                )),
            }
        }
    }

    for mode in [0u8, 1, 2, 4, 5, 6, 7, 8, 9] {
        assert_eq!(
            ui.mount(&BadApp(mode)).unwrap_err(),
            UiError::InvalidProperty,
            "mode {mode}"
        );
        assert_eq!(ui.stats().created_nodes, 0);
    }

    let valid = BadApp(3);
    ui.mount(&valid).unwrap();
}

#[test]
fn chart_type_values_and_series_update_in_place() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    struct ChartApp {
        chart_type: ChartType,
        values: Vec<i32>,
        second_series: bool,
    }
    impl App for ChartApp {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, theme: &Theme) -> Node<()> {
            let mut series = vec![ChartSeries::new(theme.accent, self.values.clone())];
            if self.second_series {
                series.push(ChartSeries::new(theme.muted, self.values.clone()));
            }
            Node::column().child(
                Node::chart(self.chart_type, 0, 100, series)
                    .key(Key::Named("chart"))
                    .style(
                        Style::default()
                            .width(Size::Percent(100))
                            .height(Size::Px(120)),
                    ),
            )
        }
    }

    let mut app = ChartApp {
        chart_type: ChartType::Line,
        values: vec![10, 20, 30, 40],
        second_series: false,
    };
    ui.mount(&app).unwrap();
    let chart = child(app_root(), 0);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_chart_get_type(chart.as_ptr()) },
        rs_lvgl_sys::lv_chart_type_t_LV_CHART_TYPE_LINE
    );
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_chart_get_point_count(chart.as_ptr()) },
        4
    );

    app.values = vec![5, 15, 25, 35];
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 2);
    assert_eq!(ui.stats().deleted_nodes, 0);

    app.chart_type = ChartType::Bar;
    app.second_series = true;
    app.values = vec![1, 2, 3, 4, 5, 6];
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 2);
    assert_eq!(ui.stats().deleted_nodes, 0);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_chart_get_type(chart.as_ptr()) },
        rs_lvgl_sys::lv_chart_type_t_LV_CHART_TYPE_BAR
    );
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_chart_get_point_count(chart.as_ptr()) },
        6
    );
    let mut series_count = 0;
    let mut series = unsafe { rs_lvgl_sys::lv_chart_get_series_next(chart.as_ptr(), ptr::null()) };
    while !series.is_null() {
        series_count += 1;
        series = unsafe { rs_lvgl_sys::lv_chart_get_series_next(chart.as_ptr(), series) };
    }
    assert_eq!(series_count, 2);

    app.second_series = false;
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 2);
    assert_eq!(ui.stats().deleted_nodes, 0);
    series_count = 0;
    series = unsafe { rs_lvgl_sys::lv_chart_get_series_next(chart.as_ptr(), ptr::null()) };
    while !series.is_null() {
        series_count += 1;
        series = unsafe { rs_lvgl_sys::lv_chart_get_series_next(chart.as_ptr(), series) };
    }
    assert_eq!(series_count, 1);
}

#[test]
fn grid_tracks_and_cells_place_children_and_update_in_place() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    struct GridApp {
        wide_first: bool,
    }
    impl App for GridApp {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            let columns = if self.wide_first {
                vec![GridTrack::Fr(3), GridTrack::Fr(1)]
            } else {
                vec![GridTrack::Fr(1), GridTrack::Fr(1)]
            };
            let cell = |column, row| {
                Node::obj()
                    .key(Key::Id((row * 2 + column) as u64))
                    .grid_cell(column as u16, row as u16, 1, 1)
                    .style(Style::default().width(Size::Px(10)).height(Size::Px(10)))
            };
            Node::column()
                .style(
                    Style::default()
                        .width(Size::Percent(100))
                        .height(Size::Percent(100)),
                )
                .child(
                    Node::grid(columns, [GridTrack::Fr(1), GridTrack::Fr(1)])
                        .key(Key::Named("grid"))
                        .style(Style::default().width(Size::Percent(100)))
                        .child(cell(0, 0))
                        .child(cell(1, 0))
                        .child(cell(0, 1))
                        .child(cell(1, 1)),
                )
        }
    }

    let mut app = GridApp { wide_first: false };
    ui.mount(&app).unwrap();
    let grid = child(app_root(), 0);
    unsafe { rs_lvgl_sys::lv_obj_update_layout(grid.as_ptr()) };
    let first = child(grid, 0);
    let second = child(grid, 1);
    let third = child(grid, 2);
    let first_x = unsafe { rs_lvgl_sys::lv_obj_get_x(first.as_ptr()) };
    let second_x = unsafe { rs_lvgl_sys::lv_obj_get_x(second.as_ptr()) };
    let first_y = unsafe { rs_lvgl_sys::lv_obj_get_y(first.as_ptr()) };
    let third_y = unsafe { rs_lvgl_sys::lv_obj_get_y(third.as_ptr()) };
    assert!(first_x < second_x, "columns must advance");
    assert!(first_y < third_y, "rows must advance");

    app.wide_first = true;
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 6);
    assert_eq!(ui.stats().deleted_nodes, 0);
    unsafe { rs_lvgl_sys::lv_obj_update_layout(grid.as_ptr()) };
    let second_x_after = unsafe { rs_lvgl_sys::lv_obj_get_x(second.as_ptr()) };
    assert!(
        second_x_after > second_x,
        "widening the first track must move the second column"
    );
}

#[test]
fn grid_content_row_sizes_to_children() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    struct App1;
    impl App for App1 {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            Node::column()
                .style(
                    Style::default()
                        .width(Size::Percent(100))
                        .height(Size::Percent(100)),
                )
                .child(
                    Node::grid([GridTrack::Fr(1)], [GridTrack::Content])
                        .key(Key::Named("grid"))
                        .style(Style::default().width(Size::Percent(100)))
                        .child(
                            Node::obj()
                                .grid_cell(0, 0, 1, 1)
                                .style(Style::default().width(Size::Px(10)).height(Size::Px(10))),
                        ),
                )
        }
    }

    ui.mount(&App1).unwrap();
    let grid = child(app_root(), 0);
    unsafe { rs_lvgl_sys::lv_obj_update_layout(grid.as_ptr()) };
    assert_eq!(unsafe { rs_lvgl_sys::lv_obj_get_height(grid.as_ptr()) }, 10);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_obj_get_height(child(grid, 0).as_ptr()) },
        10
    );
}

#[test]
fn grid_with_column_children_content_height() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    struct App2;
    impl App for App2 {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            Node::column()
                .style(
                    Style::default()
                        .width(Size::Percent(100))
                        .height(Size::Percent(100)),
                )
                .child(
                    Node::column()
                        .style(Style::default().width(Size::Percent(100)))
                        .child(
                            Node::grid([GridTrack::Fr(1), GridTrack::Fr(1)], [GridTrack::Content])
                                .key(Key::Named("grid"))
                                .style(Style::default().width(Size::Percent(100)).gap(12))
                                .child(
                                    Node::column()
                                        .grid_cell(0, 0, 1, 1)
                                        .style(Style::default().gap(4))
                                        .child(Node::label("Gender")),
                                )
                                .child(
                                    Node::column()
                                        .grid_cell(1, 0, 1, 1)
                                        .style(Style::default().gap(4))
                                        .child(Node::label("Birthday")),
                                ),
                        ),
                )
        }
    }

    ui.mount(&App2).unwrap();
    let card = child(app_root(), 0);
    let grid = child(card, 0);
    unsafe { rs_lvgl_sys::lv_obj_update_layout(grid.as_ptr()) };
    let grid_h = unsafe { rs_lvgl_sys::lv_obj_get_height(grid.as_ptr()) };
    assert!(
        grid_h < 60,
        "grid height should follow label content, got {grid_h}"
    );
}

#[test]
fn grid_with_input_children_content_height() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    struct App3;
    impl App for App3 {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            let input = Style::default()
                .bg(rs_lvgl_ui::Color::rgb(20, 20, 20))
                .fg(rs_lvgl_ui::Color::rgb(240, 240, 240))
                .border(rs_lvgl_ui::Color::rgb(90, 90, 90), 1)
                .radius(6)
                .pad(8)
                .width(Size::Percent(100));
            Node::column()
                .style(
                    Style::default()
                        .width(Size::Percent(100))
                        .height(Size::Percent(100)),
                )
                .child(
                    Node::grid([GridTrack::Fr(1), GridTrack::Fr(1)], [GridTrack::Content])
                        .key(Key::Named("grid"))
                        .style(Style::default().width(Size::Percent(100)).gap(12))
                        .child(
                            Node::column()
                                .grid_cell(0, 0, 1, 1)
                                .style(Style::default().gap(4))
                                .child(Node::label("Gender"))
                                .child(Node::dropdown(["Male", "Female"], 0).style(input)),
                        )
                        .child(
                            Node::column()
                                .grid_cell(1, 0, 1, 1)
                                .style(Style::default().gap(4))
                                .child(Node::label("Birthday"))
                                .child(Node::textarea("").placeholder("01.01.1990").style(input)),
                        ),
                )
        }
    }

    ui.mount(&App3).unwrap();
    let grid = child(app_root(), 0);
    unsafe { rs_lvgl_sys::lv_obj_update_layout(grid.as_ptr()) };
    let grid_h = unsafe { rs_lvgl_sys::lv_obj_get_height(grid.as_ptr()) };
    assert!(
        grid_h < 100,
        "grid height should follow input content, got {grid_h}"
    );
}

#[test]
fn rgb565_image_source_builds_descriptor_and_updates_in_place() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    static PIXELS_A: [u8; 8] = [0x00, 0xf8, 0xe0, 0x07, 0x1f, 0x00, 0xff, 0xff];
    static PIXELS_B: [u8; 8] = [0xff, 0xff, 0x00, 0x00, 0x1f, 0x00, 0xe0, 0x07];

    struct ImageApp {
        second: bool,
        scale: u16,
    }
    impl App for ImageApp {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            let source = if self.second {
                ImageSource::rgb565(2, 2, &PIXELS_B)
            } else {
                ImageSource::rgb565(2, 2, &PIXELS_A)
            };
            Node::column().child(
                Node::image_scaled(source, self.scale)
                    .key(Key::Named("image"))
                    .style(Style::default().width(Size::Px(8)).height(Size::Px(8))),
            )
        }
    }

    let mut app = ImageApp {
        second: false,
        scale: 256,
    };
    ui.mount(&app).unwrap();
    let image = child(app_root(), 0);
    let first_src = unsafe { rs_lvgl_sys::lv_image_get_src(image.as_ptr()) };
    assert!(!first_src.is_null());

    app.scale = 512;
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 2);
    assert_eq!(ui.stats().deleted_nodes, 0);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_image_get_scale(image.as_ptr()) },
        512
    );
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_image_get_src(image.as_ptr()) },
        first_src
    );

    app.second = true;
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 2);
    assert_eq!(ui.stats().deleted_nodes, 0);
    let second_src = unsafe { rs_lvgl_sys::lv_image_get_src(image.as_ptr()) };
    assert!(!second_src.is_null());
    assert_ne!(second_src, first_src);
}

#[test]
fn rgb565_image_rejects_wrong_data_length() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    static SHORT: [u8; 2] = [0, 0];
    struct BadImage;
    impl App for BadImage {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            Node::column().child(Node::image(ImageSource::rgb565(2, 2, &SHORT)))
        }
    }

    assert_eq!(ui.mount(&BadImage).unwrap_err(), UiError::InvalidProperty);
    assert_eq!(ui.stats().created_nodes, 0);
}

#[test]
fn part_styles_override_and_restore_the_target_part() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    let indicator = rs_lvgl_ui::Color::rgb(200, 10, 20);

    struct PartApp {
        styled: bool,
    }
    impl App for PartApp {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            let slider = Node::slider(0, 100, 50).key(Key::Named("slider")).style(
                Style::default()
                    .width(Size::Percent(100))
                    .height(Size::Px(16)),
            );
            let slider = if self.styled {
                slider.part_style(
                    Part::Indicator,
                    Style::default().bg(rs_lvgl_ui::Color::rgb(200, 10, 20)),
                )
            } else {
                slider
            };
            Node::column().child(slider)
        }
    }

    let mut app = PartApp { styled: true };
    ui.mount(&app).unwrap();
    let slider = child(app_root(), 0);
    let color = unsafe {
        rs_lvgl_sys::lv_obj_get_style_bg_color(
            slider.as_ptr(),
            rs_lvgl_sys::lv_part_t_LV_PART_INDICATOR,
        )
    };
    assert_eq!((color.red, color.green, color.blue), (200, 10, 20));

    app.styled = false;
    ui.update(&app).unwrap();
    let color = unsafe {
        rs_lvgl_sys::lv_obj_get_style_bg_color(
            slider.as_ptr(),
            rs_lvgl_sys::lv_part_t_LV_PART_INDICATOR,
        )
    };
    assert_ne!((color.red, color.green, color.blue), (200, 10, 20));
    assert_eq!(ui.stats().created_nodes, 2);
    assert_eq!(ui.stats().deleted_nodes, 0);
    let _ = indicator;
}

#[test]
fn animation_duration_style_applies() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    struct AnimApp {
        duration: u32,
    }
    impl App for AnimApp {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            Node::column().child(
                Node::obj().key(Key::Named("box")).style(
                    Style::default()
                        .width(Size::Px(10))
                        .height(Size::Px(10))
                        .anim_duration(self.duration),
                ),
            )
        }
    }

    let mut app = AnimApp { duration: 250 };
    ui.mount(&app).unwrap();
    let object = child(app_root(), 0);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_obj_get_style_anim_duration(object.as_ptr(), 0) },
        250
    );

    app.duration = 0;
    ui.update(&app).unwrap();
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_obj_get_style_anim_duration(object.as_ptr(), 0) },
        0
    );
}

#[test]
fn tabview_switches_pages_updates_titles_and_rebuilds_on_tab_count_change() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    struct TabApp {
        tabs: Vec<&'static str>,
        active: u32,
        value: i32,
    }
    impl App for TabApp {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            let mut tabview = Node::tabview(self.tabs.iter().copied(), self.active)
                .key(Key::Named("tabs"))
                .style(
                    Style::default()
                        .width(Size::Percent(100))
                        .height(Size::Px(300)),
                );
            for index in 0..self.tabs.len() {
                tabview = tabview.child(
                    Node::column()
                        .key(Key::Id(index as u64))
                        .child(Node::label(format!("page {index} value {}", self.value))),
                );
            }
            Node::column().child(tabview)
        }
    }

    let mut app = TabApp {
        tabs: vec!["Profile", "Analytics", "Shop"],
        active: 0,
        value: 1,
    };
    ui.mount(&app).unwrap();
    let mut tabview = child(app_root(), 0);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_tabview_get_tab_active(tabview.as_ptr()) },
        0
    );
    assert_eq!(ui.stats().mounted_nodes, 8);

    app.active = 2;
    app.value = 9;
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 8);
    assert_eq!(ui.stats().deleted_nodes, 0);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_tabview_get_tab_active(tabview.as_ptr()) },
        2
    );

    app.tabs[0] = "Me";
    app.active = 1;
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 8);
    assert_eq!(ui.stats().deleted_nodes, 0);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_tabview_get_tab_active(tabview.as_ptr()) },
        1
    );

    app.tabs = vec!["One", "Two"];
    app.active = 1;
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().mounted_nodes, 6);
    assert_eq!(ui.stats().deleted_nodes, 7);
    tabview = child(app_root(), 0);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_tabview_get_tab_active(tabview.as_ptr()) },
        1
    );
}

#[test]
fn tabview_validation_rejects_bad_tabs() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    struct BadTabs(u8);
    impl App for BadTabs {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            match self.0 {
                0 => Node::column().child(Node::tabview(Vec::<String>::new(), 0)),
                1 => Node::column().child(Node::tabview(["A"], 1).child(Node::column())),
                2 => Node::column().child(Node::tabview(["A", "B"], 0)),
                _ => Node::column().child(Node::tabview(["A"], 0).child(Node::column())),
            }
        }
    }

    for mode in [0u8, 1, 2] {
        assert_eq!(
            ui.mount(&BadTabs(mode)).unwrap_err(),
            UiError::InvalidProperty,
            "mode {mode}"
        );
        assert_eq!(ui.stats().created_nodes, 0);
    }

    ui.mount(&BadTabs(3)).unwrap();
}

#[test]
fn tabview_value_event_updates_app_state() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Tab(i32),
    }
    struct TabEventApp {
        active: u32,
    }
    impl App for TabEventApp {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) {
            match msg {
                Msg::Tab(index) => self.active = index.max(0) as u32,
            }
        }
        fn view(&self, _: &Theme) -> Node<Msg> {
            Node::column().child(
                Node::tabview(["A", "B", "C"], self.active)
                    .key(Key::Named("tabs"))
                    .on_value_changed(Msg::Tab)
                    .child(Node::column().key(Key::Named("a")))
                    .child(Node::column().key(Key::Named("b")))
                    .child(Node::column().key(Key::Named("c"))),
            )
        }
    }

    let mut app = TabEventApp { active: 0 };
    ui.mount(&app).unwrap();
    let tabview = child(app_root(), 0);
    unsafe {
        rs_lvgl_sys::lv_tabview_set_active(tabview.as_ptr(), 2, false);
        rs_lvgl_sys::lv_obj_send_event(
            tabview.as_ptr(),
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            ptr::null_mut(),
        );
    }
    let drained = ui.drain(&mut app, 8).unwrap();
    assert_eq!(drained.processed, 1);
    assert_eq!(app.active, 2);
    assert_eq!(ui.stats().created_nodes, 5);
    assert_eq!(ui.stats().deleted_nodes, 0);
}

#[derive(Clone, Debug, PartialEq)]
enum KeyboardMsg {
    Focus(bool),
}

#[test]
fn keyboard_binds_to_keyboard_id_targets() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    struct KeyboardApp {
        target: u64,
        focused: bool,
    }
    impl App for KeyboardApp {
        type Msg = KeyboardMsg;
        fn update(&mut self, msg: KeyboardMsg) {
            match msg {
                KeyboardMsg::Focus(value) => self.focused = value,
            }
        }
        fn view(&self, _: &Theme) -> Node<KeyboardMsg> {
            let textarea = |text, id: u64| {
                Node::textarea(text)
                    .keyboard_id(id)
                    .key(Key::Id(id))
                    .on_event(move |event| match event {
                        Event::Focused => Some(KeyboardMsg::Focus(true)),
                        Event::Defocused | Event::Ready | Event::Cancel => {
                            Some(KeyboardMsg::Focus(false))
                        }
                        _ => None,
                    })
            };
            Node::column()
                .child(textarea("a", 7))
                .child(textarea("b", 8))
                .child(Node::keyboard(self.target).key(Key::Id(100)).on_event(
                    |event| match event {
                        Event::Ready | Event::Cancel => Some(KeyboardMsg::Focus(false)),
                        _ => None,
                    },
                ))
        }
    }

    let mut app = KeyboardApp {
        target: 7,
        focused: false,
    };
    ui.mount(&app).unwrap();
    let first = child(app_root(), 0);
    let second = child(app_root(), 1);
    let keyboard = child(app_root(), 2);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_keyboard_get_textarea(keyboard.as_ptr()) },
        first.as_ptr()
    );

    unsafe {
        rs_lvgl_sys::lv_obj_send_event(
            first.as_ptr(),
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_FOCUSED,
            ptr::null_mut(),
        );
    }
    let drained = ui.drain(&mut app, 8).unwrap();
    assert_eq!(drained.processed, 1);
    assert!(app.focused);

    unsafe {
        rs_lvgl_sys::lv_obj_send_event(
            keyboard.as_ptr(),
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_READY,
            ptr::null_mut(),
        );
    }
    let drained = ui.drain(&mut app, 8).unwrap();
    assert_eq!(drained.processed, 1);
    assert!(!app.focused);

    app.target = 8;
    ui.update(&app).unwrap();
    assert_eq!(ui.stats().created_nodes, 4);
    assert_eq!(ui.stats().deleted_nodes, 0);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_keyboard_get_textarea(keyboard.as_ptr()) },
        second.as_ptr()
    );
}

#[test]
fn keyboard_validation_rejects_wrong_targets() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();

    struct BadKeyboard(u8);
    impl App for BadKeyboard {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            match self.0 {
                0 => Node::column().child(Node::label("x").keyboard_id(1)),
                1 => Node::column().child(Node::keyboard(1).child(Node::label("x"))),
                _ => Node::column()
                    .child(Node::textarea("").keyboard_id(1))
                    .child(Node::keyboard(1)),
            }
        }
    }

    for mode in [0u8, 1] {
        assert_eq!(
            ui.mount(&BadKeyboard(mode)).unwrap_err(),
            UiError::InvalidProperty,
            "mode {mode}"
        );
        assert_eq!(ui.stats().created_nodes, 0);
    }
    ui.mount(&BadKeyboard(2)).unwrap();
}
