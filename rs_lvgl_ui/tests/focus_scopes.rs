mod common;
use rs_lvgl::input_device::{key, Keypad, KeypadInputData};
use rs_lvgl_ui::{
    App, GridNavigation, GridTrack, Key, NavigationAxis, Node, Size, Style, Theme, Ui, UiError,
};
use std::{cell::Cell, rc::Rc};

struct Keys {
    device: Keypad,
    sample: Rc<Cell<KeypadInputData>>,
}
impl Keys {
    fn new(display: &rs_lvgl::Display) -> Self {
        let sample = Rc::new(Cell::new(KeypadInputData::released(0)));
        let source = sample.clone();
        Self {
            device: Keypad::register(move || source.get(), display).unwrap(),
            sample,
        }
    }
    fn send(&self, code: u32) {
        for sample in [
            KeypadInputData::pressed(code),
            KeypadInputData::released(code),
        ] {
            self.sample.set(sample);
            unsafe {
                rs_lvgl_sys::lv_indev_read(self.device.raw().unwrap().as_ptr());
            }
        }
    }
    fn group(&self) -> *mut rs_lvgl_sys::lv_group_t {
        unsafe { rs_lvgl_sys::lv_indev_get_group(self.device.raw().unwrap().as_ptr()) }
    }
    fn focused(&self) -> *mut rs_lvgl_sys::lv_obj_t {
        unsafe { rs_lvgl_sys::lv_group_get_focused(self.group()) }
    }
    fn count(&self) -> u32 {
        unsafe { rs_lvgl_sys::lv_group_get_obj_count(self.group()) }
    }
}
fn has_focus(obj: core::ptr::NonNull<rs_lvgl_sys::lv_obj_t>) -> bool {
    unsafe { rs_lvgl_sys::lv_obj_has_state(obj.as_ptr(), rs_lvgl_sys::lv_state_t_LV_STATE_FOCUSED) }
}

#[test]
fn file_entries_accept_keys_and_preserve_focus_across_reorder() {
    use rs_lvgl_ui::components::file_explorer::{self, FileEntry};

    struct Files {
        ids: Vec<u64>,
        selected: Vec<u64>,
    }
    impl App for Files {
        type Msg = u64;
        fn update(&mut self, id: u64) {
            self.selected.push(id);
        }
        fn view(&self, theme: &Theme) -> Node<u64> {
            file_explorer::view(
                theme,
                "/samples",
                Some(0),
                self.ids
                    .iter()
                    .map(|id| FileEntry::file(*id, "same name", *id)),
            )
        }
    }
    let _guard = common::lock();
    let display = common::init();
    let keys = Keys::new(&display);
    let mut app = Files {
        ids: vec![1, 2],
        selected: Vec::new(),
    };
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    ui.mount(&app).unwrap();
    ui.bind_keypad(&keys.device).unwrap();
    assert_eq!(keys.count(), 3, "parent action and both file entries");
    keys.send(key::NEXT);
    assert!(ui.focus(Key::Id(1)).unwrap());
    keys.send(key::NEXT);
    let selected = keys.focused();
    keys.send(key::ENTER);
    assert!(app.selected.is_empty(), "callbacks only enqueue messages");
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.selected, vec![2]);

    app.ids.reverse();
    ui.update(&app).unwrap();
    assert_eq!(keys.focused(), selected);
    keys.send(key::ENTER);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.selected, vec![2, 2]);
    app.ids.retain(|id| *id != 2);
    ui.update(&app).unwrap();
    assert_eq!(keys.count(), 2);
    assert!(!ui.focus(Key::Id(2)).unwrap());
    ui.unmount();
    assert!(keys.group().is_null());
}

#[derive(Default)]
struct Board {
    ids: Vec<u64>,
    config: Option<GridNavigation>,
    disabled: Option<u64>,
    hidden: Option<u64>,
    excluded: Option<u64>,
    modal: bool,
    clicks: Vec<u64>,
}
impl App for Board {
    type Msg = u64;
    fn update(&mut self, id: u64) {
        self.clicks.push(id);
    }
    fn view(&self, _: &Theme) -> Node<u64> {
        let mut grid = Node::grid(
            [GridTrack::Px(25), GridTrack::Px(25)],
            [GridTrack::Px(25), GridTrack::Px(25)],
        )
        .key(Key::Named("grid"))
        .style(Style::neutral().width(Size::Px(55)).height(Size::Px(55)))
        .children(self.ids.iter().enumerate().map(|(i, id)| {
            Node::button("X")
                .key(Key::Id(*id))
                .grid_cell((i % 2) as u16, (i / 2) as u16, 1, 1)
                .style(
                    Style::neutral()
                        .width(Size::Px(20))
                        .height(Size::Px(20))
                        .disabled(self.disabled == Some(*id))
                        .hidden(self.hidden == Some(*id)),
                )
                .focusable(self.excluded != Some(*id))
                .on_click(*id)
        }));
        if let Some(config) = self.config {
            grid = grid.grid_navigation(config);
        }
        Node::column().child(grid).child(
            Node::button("Outside")
                .key(Key::Named("outside"))
                .on_click(99),
        )
    }
    fn overlay(&self, _: &Theme) -> Option<Node<u64>> {
        self.modal.then(|| {
            Node::column()
                .modal(true)
                .child(Node::button("Close").key(Key::Named("close")))
        })
    }
}
fn board() -> Board {
    Board {
        ids: vec![1, 2, 3, 4],
        config: Some(GridNavigation {
            rollover: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}
fn grid_child(index: i32) -> core::ptr::NonNull<rs_lvgl_sys::lv_obj_t> {
    common::child(common::child(common::app_root(), 0), index)
}
#[test]
fn arrows_activate_spatial_children_tab_leaves_and_modal_restores_selection() {
    let _guard = common::lock();
    let display = common::init();
    let keys = Keys::new(&display);
    let mut app = board();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    ui.mount(&app).unwrap();
    ui.bind_keypad(&keys.device).unwrap();
    unsafe {
        rs_lvgl_sys::lv_obj_update_layout(common::screen().as_ptr());
    }
    assert_eq!(keys.count(), 2);
    assert!(has_focus(grid_child(0)));
    keys.send(key::RIGHT);
    assert!(has_focus(grid_child(1)));
    keys.send(key::DOWN);
    assert!(has_focus(grid_child(3)));
    keys.send(key::ENTER);
    ui.drain(&mut app, 32).unwrap();
    assert_eq!(app.clicks, vec![4]);
    keys.send(key::NEXT);
    assert_eq!(
        keys.focused(),
        common::child(common::app_root(), 1).as_ptr()
    );
    assert!(!has_focus(grid_child(3)));
    assert!(ui.focus(Key::Id(3)).unwrap());
    assert!(has_focus(grid_child(2)));
    app.modal = true;
    ui.update(&app).unwrap();
    assert_eq!(keys.count(), 1);
    assert!(!ui.focus(Key::Id(1)).unwrap());
    assert!(!ui.focus(Key::Named("outside")).unwrap());
    for _ in 0..3 {
        ui.update(&app).unwrap();
        keys.send(key::NEXT);
    }
    app.modal = false;
    ui.update(&app).unwrap();
    assert_eq!(keys.count(), 2);
    assert!(has_focus(grid_child(2)));
    keys.send(key::ENTER);
    ui.drain(&mut app, 32).unwrap();
    assert_eq!(app.clicks, vec![4, 3]);
}
#[test]
fn grid_reorder_exclusion_empty_and_disable_restore_native_flags() {
    let _guard = common::lock();
    let display = common::init();
    let keys = Keys::new(&display);
    let mut app = board();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    ui.mount(&app).unwrap();
    let grid = common::child(common::app_root(), 0);
    let arrow = unsafe { rs_lvgl_sys::lv_obj_is_scroll_with_arrow(grid.as_ptr()) };
    let child_flag = unsafe { rs_lvgl_sys::lv_obj_is_click_focusable(grid_child(0).as_ptr()) };
    ui.bind_keypad(&keys.device).unwrap();
    ui.focus(Key::Id(4)).unwrap();
    let selected = grid_child(3);
    app.ids = vec![4, 3, 2, 1];
    ui.update(&app).unwrap();
    assert_eq!(grid_child(0), selected);
    assert!(has_focus(selected));
    app.disabled = Some(4);
    app.hidden = Some(3);
    app.excluded = Some(2);
    ui.update(&app).unwrap();
    assert!(!ui.focus(Key::Id(4)).unwrap());
    assert!(!ui.focus(Key::Id(3)).unwrap());
    assert!(!ui.focus(Key::Id(2)).unwrap());
    assert!(!has_focus(selected));
    assert!(has_focus(grid_child(3)));
    app.ids.clear();
    ui.update(&app).unwrap();
    assert_eq!(keys.count(), 1);
    keys.send(key::ENTER);
    app.ids = vec![1, 2];
    app.config = Some(GridNavigation {
        axis: NavigationAxis::Horizontal,
        ..Default::default()
    });
    ui.update(&app).unwrap();
    ui.focus(Key::Id(1)).unwrap();
    keys.send(key::RIGHT); // Excluded 2: edge leaves the grid.
    assert_eq!(
        keys.focused(),
        common::child(common::app_root(), 1).as_ptr()
    );
    app.config = None;
    ui.update(&app).unwrap();
    assert_eq!(keys.count(), 2);
    unsafe {
        assert_eq!(
            rs_lvgl_sys::lv_obj_is_scroll_with_arrow(grid.as_ptr()),
            arrow
        );
        assert_eq!(
            rs_lvgl_sys::lv_obj_is_click_focusable(grid_child(0).as_ptr()),
            child_flag
        );
    }
}
#[derive(Default)]
struct Modals {
    depth: u8,
    hidden: bool,
    disabled: bool,
    remove_origin: bool,
}
impl App for Modals {
    type Msg = ();
    fn update(&mut self, _: ()) {}
    fn view(&self, _: &Theme) -> Node<()> {
        let mut root = Node::column();
        if !self.remove_origin {
            root = root.child(Node::button("Auto origin"));
        }
        root.child(Node::button("Fallback").key(Key::Named("fallback")))
    }
    fn overlay(&self, _: &Theme) -> Option<Node<()>> {
        if self.depth == 0 {
            return None;
        }
        let mut scope = Node::column()
            .key(Key::Id(10))
            .modal(true)
            .style(Style::neutral().hidden(self.hidden).disabled(self.disabled))
            .child(Node::button("First").key(Key::Id(11)))
            .child(Node::button("Second").key(Key::Id(12)));
        if self.depth == 2 {
            scope = scope.child(
                Node::column()
                    .key(Key::Id(20))
                    .modal(true)
                    .child(Node::button("Nested").key(Key::Id(21))),
            );
        }
        Some(scope.map(|()| ()))
    }
}
#[test]
fn modal_stack_restores_auto_identity_editing_and_nested_scope() {
    let _guard = common::lock();
    let display = common::init();
    let keys = Keys::new(&display);
    let mut app = Modals::default();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    ui.mount(&app).unwrap();
    ui.bind_keypad(&keys.device).unwrap();
    let origin = keys.focused();
    ui.set_editing(true).unwrap();
    app.depth = 1;
    ui.update(&app).unwrap();
    assert_eq!(keys.count(), 2);
    assert!(!ui.focus(Key::Named("fallback")).unwrap());
    assert!(ui.focus(Key::Id(12)).unwrap());
    let second = keys.focused();
    app.depth = 2;
    ui.update(&app).unwrap();
    assert_eq!(keys.count(), 1);
    assert!(!ui.focus(Key::Id(12)).unwrap());
    keys.send(key::NEXT);
    keys.send(key::PREV);
    assert_eq!(keys.count(), 1);
    app.depth = 1;
    ui.update(&app).unwrap();
    assert_eq!(keys.focused(), second);
    app.depth = 0;
    ui.update(&app).unwrap();
    assert_eq!(keys.focused(), origin);
    assert!(unsafe { rs_lvgl_sys::lv_group_get_editing(keys.group()) });
    app.depth = 1;
    ui.update(&app).unwrap();
    app.remove_origin = true;
    ui.update(&app).unwrap();
    app.depth = 0;
    ui.update(&app).unwrap();
    assert_eq!(keys.count(), 1);
    assert_eq!(
        keys.focused(),
        common::child(common::app_root(), 0).as_ptr()
    );
}
#[test]
fn hidden_disabled_modal_is_ignored_and_focus_activation_stays_explicit() {
    let _guard = common::lock();
    let display = common::init();
    let keys = Keys::new(&display);
    let mut app = Modals {
        depth: 1,
        hidden: true,
        ..Default::default()
    };
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    ui.mount(&app).unwrap();
    assert!(unsafe {
        rs_lvgl_sys::lv_obj_get_group(common::child(common::app_root(), 0).as_ptr())
    }
    .is_null());
    ui.bind_keypad(&keys.device).unwrap();
    assert_eq!(keys.count(), 2);
    assert!(ui.focus(Key::Named("fallback")).unwrap());
    app.hidden = false;
    app.disabled = true;
    ui.update(&app).unwrap();
    assert_eq!(keys.count(), 2);
    app.disabled = false;
    ui.update(&app).unwrap();
    assert!(!ui.focus(Key::Named("fallback")).unwrap());
    app.disabled = true;
    ui.update(&app).unwrap();
    assert!(ui.focus(Key::Named("fallback")).unwrap());
    ui.unmount();
    assert!(
        unsafe { rs_lvgl_sys::lv_indev_get_group(keys.device.raw().unwrap().as_ptr()) }.is_null()
    );
    unsafe {
        rs_lvgl::deinit();
    }
    rs_lvgl::init();
    drop(ui);
}
#[test]
fn dialog_marks_modal_and_invalid_grid_scope_preserves_existing_tree() {
    use rs_lvgl_ui::components::dialog::Dialog;
    struct View(u8);
    impl App for View {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> Node<()> {
            match self.0 {
                1 => Node::button("Invalid").grid_navigation(GridNavigation::default()),
                2 => Node::row()
                    .grid_navigation(GridNavigation::default())
                    .child(Node::row().grid_navigation(GridNavigation::default())),
                3 => Node::row()
                    .grid_navigation(GridNavigation::default())
                    .child(Node::row().modal(true)),
                _ => Node::button("Behind").key(Key::Named("behind")),
            }
        }
        fn overlay(&self, theme: &Theme) -> Option<Node<()>> {
            Some(Dialog::new("Confirm").button("OK", ()).view(theme))
        }
    }
    let _guard = common::lock();
    let display = common::init();
    let keys = Keys::new(&display);
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    ui.mount(&View(0)).unwrap();
    let root = common::app_root();
    ui.bind_keypad(&keys.device).unwrap();
    assert_eq!(keys.count(), 1);
    assert!(!ui.focus(Key::Named("behind")).unwrap());
    for invalid in 1..=3 {
        assert_eq!(ui.update(&View(invalid)), Err(UiError::InvalidProperty));
        assert_eq!(common::app_root(), root);
    }
}
