//! Host tests for the composite components: list, panel, stack navigation,
//! dialog, toasts and overlay layers.

#![cfg(feature = "components")]

mod common;

use core::ptr::{self, NonNull};
use rs_lvgl_ui::prelude::*;
use std::ffi::CStr;

use common::{app_root, child, init, lock};
use rs_lvgl_sys::lv_obj_t;
use rs_lvgl_ui::components::dialog::Dialog;
use rs_lvgl_ui::components::layers;
use rs_lvgl_ui::components::list::{self, ListItem};
use rs_lvgl_ui::components::nav::{self, NavMsg, NavState};
use rs_lvgl_ui::components::panel::{self, Panel};
use rs_lvgl_ui::components::toast::{self, Toasts};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Root,
    Wifi,
    Display,
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Open(u32),
    Toggle(i32),
    Nav(NavMsg<Page>),
    Close,
    Confirm,
    Dismiss(u64),
}

/// An application whose view and overlay are functions of its state.
struct Host {
    nav: NavState<Page>,
    toasts: Toasts,
    dialog: bool,
    view: fn(&Host, &Theme) -> View<Msg>,
    received: Vec<Msg>,
}

impl Host {
    fn new(view: fn(&Host, &Theme) -> View<Msg>) -> Self {
        Self {
            nav: NavState::new(Page::Root),
            toasts: Toasts::new(),
            dialog: false,
            view,
            received: Vec::new(),
        }
    }
}

impl App for Host {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) {
        if let Msg::Nav(nav) = &msg {
            self.nav.update(nav.clone());
        }
        if let Msg::Dismiss(id) = &msg {
            self.toasts.dismiss(*id);
        }
        self.received.push(msg);
    }

    fn view(&self, theme: &Theme) -> View<Msg> {
        column([]).child((self.view)(self, theme))
    }

    fn overlay(&self, theme: &Theme) -> Option<View<Msg>> {
        if !self.dialog && self.toasts.is_empty() {
            return None;
        }
        let mut children = Vec::new();
        if self.dialog {
            children.push(
                Dialog::new("Delete?")
                    .text("This cannot be undone.")
                    .button("Cancel", Msg::Close)
                    .primary("Delete", Msg::Confirm)
                    .on_dismiss(Msg::Close)
                    .view(theme),
            );
        }
        children.push(toast::view(&self.toasts, theme, Msg::Dismiss));
        Some(layers(children))
    }
}

fn mount(app: &Host) -> (Ui<Msg>, NonNull<lv_obj_t>) {
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    ui.mount(app).unwrap();
    let widget = child(app_root(), 0);
    (ui, widget)
}

fn click(obj: NonNull<lv_obj_t>) {
    // SAFETY: tests only send events to live mounted objects.
    unsafe {
        rs_lvgl_sys::lv_obj_send_event(
            obj.as_ptr(),
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED,
            ptr::null_mut(),
        )
    };
}

fn child_count(obj: NonNull<lv_obj_t>) -> u32 {
    // SAFETY: tests pass live objects.
    unsafe { rs_lvgl_sys::lv_obj_get_child_count(obj.as_ptr()) }
}

fn label_text(obj: NonNull<lv_obj_t>) -> String {
    // SAFETY: tests pass live labels; LVGL returns a NUL-terminated string.
    let raw = unsafe { rs_lvgl_sys::lv_label_get_text(obj.as_ptr()) };
    unsafe { CStr::from_ptr(raw) }.to_str().unwrap().to_owned()
}

/// Text of a button's implicit label.
fn button_text(obj: NonNull<lv_obj_t>) -> String {
    label_text(child(obj, 0))
}

fn has_state(obj: NonNull<lv_obj_t>, state: rs_lvgl_sys::lv_state_t) -> bool {
    // SAFETY: tests pass live objects.
    unsafe { rs_lvgl_sys::lv_obj_has_state(obj.as_ptr(), state) }
}

fn has_flag(obj: NonNull<lv_obj_t>, flag: rs_lvgl_sys::lv_obj_flag_t) -> bool {
    // SAFETY: tests pass live objects.
    unsafe { rs_lvgl_sys::lv_obj_has_flag(obj.as_ptr(), flag) }
}

/// The overlay tree root: the first child of the overlay root container.
fn overlay_tree() -> Option<NonNull<lv_obj_t>> {
    // SAFETY: a display was registered by `init`.
    let layer = NonNull::new(unsafe { rs_lvgl_sys::lv_layer_top() })?;
    if child_count(layer) == 0 {
        return None;
    }
    Some(child(child(layer, 0), 0))
}

const CLICKABLE: rs_lvgl_sys::lv_obj_flag_t = rs_lvgl_sys::lv_obj_flag_t_LV_OBJ_FLAG_CLICKABLE;

#[test]
fn list_item_builds_icon_text_detail_and_chevron() {
    let _guard = lock();
    let mut app = Host::new(|_, theme| {
        list::list(
            theme,
            [
                list::section(theme, "Network"),
                ListItem::new("Wi-Fi")
                    .icon(symbol::WIFI)
                    .detail("Home")
                    .chevron(true)
                    .on_click(Msg::Open(1))
                    .view(theme),
                ListItem::new("Info").view(theme),
            ],
        )
    });
    let (mut ui, list) = mount(&app);
    assert_eq!(child_count(list), 3);
    assert_eq!(label_text(child(list, 0)), "Network");

    let row = child(list, 1);
    assert_eq!(child_count(row), 4);
    assert_eq!(label_text(child(row, 0)), symbol::WIFI);
    assert_eq!(label_text(child(row, 1)), "Wi-Fi");
    assert_eq!(label_text(child(row, 2)), "Home");
    assert_eq!(label_text(child(row, 3)), symbol::RIGHT);
    assert!(has_flag(row, CLICKABLE));

    // A row without a click message does not take input.
    let plain = child(list, 2);
    assert_eq!(child_count(plain), 1);
    assert!(!has_flag(plain, CLICKABLE));

    click(row);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.received, vec![Msg::Open(1)]);
}

#[test]
fn list_item_selection_is_controlled() {
    let _guard = lock();
    let mut app = Host::new(|host, theme| {
        let selected = host.received.len() as u32 % 2;
        list::list(
            theme,
            (0..2).map(|index| {
                ListItem::new(format!("Option {index}"))
                    .selected(index == selected)
                    .on_click(Msg::Open(index))
                    .view(theme)
            }),
        )
    });
    let (mut ui, list) = mount(&app);
    let checked = rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED;
    assert!(has_state(child(list, 0), checked));
    assert!(!has_state(child(list, 1), checked));

    click(child(list, 1));
    ui.drain(&mut app, 8).unwrap();
    assert!(!has_state(child(list, 0), checked));
    assert!(has_state(child(list, 1), checked));
}

#[test]
fn list_item_trailing_node_keeps_its_handler() {
    let _guard = lock();
    let mut app = Host::new(|_, theme| {
        ListItem::new("Bluetooth")
            .trailing(switch(false).on_value_changed(Msg::Toggle))
            .view(theme)
    });
    let (mut ui, row) = mount(&app);
    let switch = child(row, 1);
    // SAFETY: the switch is live.
    unsafe {
        rs_lvgl_sys::lv_obj_add_state(switch.as_ptr(), rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED);
        rs_lvgl_sys::lv_obj_send_event(
            switch.as_ptr(),
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            ptr::null_mut(),
        );
    }
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.received, vec![Msg::Toggle(1)]);
}

#[test]
fn panel_has_header_actions_and_content() {
    let _guard = lock();
    let mut app = Host::new(|_, theme| {
        Panel::new("Logs")
            .action(panel::icon_button(theme, symbol::REFRESH, Msg::Open(7)))
            .child(label("line 1"))
            .child(label("line 2"))
            .view(theme)
    });
    let (mut ui, panel) = mount(&app);
    assert_eq!(child_count(panel), 2);
    let header = child(panel, 0);
    assert_eq!(label_text(child(header, 0)), "Logs");
    let action = child(header, 1);
    assert_eq!(button_text(action), symbol::REFRESH);
    let content = child(panel, 1);
    assert_eq!(child_count(content), 2);
    assert_eq!(label_text(child(content, 1)), "line 2");

    click(action);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.received, vec![Msg::Open(7)]);
}

#[test]
fn nav_state_updates() {
    let mut nav = NavState::new(Page::Root);
    assert!(!nav.can_go_back());
    nav.update(NavMsg::Back);
    assert_eq!(nav.current(), &Page::Root);

    nav.update(NavMsg::Push(Page::Wifi));
    nav.update(NavMsg::Push(Page::Display));
    assert_eq!(nav.depth(), 2);
    assert_eq!(nav.pages(), &[Page::Root, Page::Wifi, Page::Display]);

    nav.update(NavMsg::Replace(Page::Wifi));
    assert_eq!(nav.current(), &Page::Wifi);
    assert_eq!(nav.depth(), 2);

    nav.update(NavMsg::Back);
    assert_eq!(nav.depth(), 1);
    nav.update(NavMsg::Root);
    assert_eq!(nav.current(), &Page::Root);
    assert_eq!(nav.depth(), 0);
}

fn nav_view(host: &Host, theme: &Theme) -> View<Msg> {
    let (title, page) = match host.nav.current() {
        Page::Root => (
            "Settings",
            list::list(
                theme,
                [nav::link(
                    theme,
                    "Wi-Fi",
                    Msg::Nav(NavMsg::Push(Page::Wifi)),
                )],
            ),
        ),
        Page::Wifi => ("Wi-Fi", label("Networks")),
        Page::Display => ("Display", label("Brightness")),
    };
    nav::view(&host.nav, theme, title, page, Msg::Nav(NavMsg::Back))
}

#[test]
fn nav_view_pushes_and_pops_pages() {
    let _guard = lock();
    let mut app = Host::new(nav_view);
    let (mut ui, nav) = mount(&app);
    let header = child(nav, 0);
    // The root page has no back button.
    assert_eq!(child_count(header), 1);
    assert_eq!(label_text(child(header, 0)), "Settings");
    let root_body = child(nav, 1);

    let link = child(child(root_body, 0), 0);
    click(link);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.nav.current(), &Page::Wifi);
    assert_eq!(child_count(header), 2);
    assert_eq!(button_text(child(header, 0)), symbol::LEFT);
    assert_eq!(label_text(child(header, 1)), "Wi-Fi");
    let page_body = child(nav, 1);
    assert_eq!(label_text(child(page_body, 0)), "Networks");

    click(child(header, 0));
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.nav.current(), &Page::Root);
    assert_eq!(child_count(header), 1);
    assert_eq!(child_count(child(nav, 1)), 1);
}

#[test]
fn dialog_mounts_on_top_layer_and_closes_through_state() {
    let _guard = lock();
    let mut app = Host::new(|_, _| label("view"));
    let (mut ui, _) = mount(&app);
    assert!(overlay_tree().is_none());

    app.dialog = true;
    ui.update(&app).unwrap();
    let layers = overlay_tree().expect("overlay mounted");
    assert!(!has_flag(layers, CLICKABLE));
    let backdrop = child(layers, 0);
    assert!(has_flag(backdrop, CLICKABLE));
    let card = child(backdrop, 0);
    assert!(has_flag(card, CLICKABLE));
    assert_eq!(label_text(child(card, 0)), "Delete?");
    assert_eq!(label_text(child(card, 1)), "This cannot be undone.");
    let buttons = child(card, 2);
    // The buttons in insertion order, right-aligned inside the padded row.
    assert_eq!(child_count(buttons), 2);
    assert_eq!(button_text(child(buttons, 0)), "Cancel");
    assert_eq!(button_text(child(buttons, 1)), "Delete");
    // SAFETY: the row and its buttons are live.
    unsafe {
        rs_lvgl_sys::lv_obj_update_layout(buttons.as_ptr());
        let last = child(buttons, 1);
        let mut content = rs_lvgl_sys::lv_area_t {
            x1: 0,
            y1: 0,
            x2: 0,
            y2: 0,
        };
        let mut bounds = content;
        rs_lvgl_sys::lv_obj_get_content_coords(buttons.as_ptr(), &mut content);
        rs_lvgl_sys::lv_obj_get_coords(last.as_ptr(), &mut bounds);
        assert_eq!(bounds.x2, content.x2);
    }

    click(child(buttons, 1));
    click(backdrop);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.received, vec![Msg::Confirm, Msg::Close]);

    app.dialog = false;
    ui.update(&app).unwrap();
    assert!(overlay_tree().is_none());
}

#[test]
fn toasts_expire_drop_oldest_and_dismiss() {
    let mut toasts = Toasts::with_max_visible(2);
    let first = toasts.push("one", 100);
    toasts.push("two", 300);
    toasts.push("three", 300);
    assert_eq!(toasts.texts().collect::<Vec<_>>(), ["two", "three"]);
    toasts.dismiss(first);
    assert_eq!(toasts.len(), 2);

    assert!(!toasts.advance(200));
    assert!(toasts.advance(100));
    assert!(toasts.is_empty());

    let mut toasts = Toasts::new();
    let a = toasts.push("a", 50);
    toasts.push("b", 500);
    assert!(toasts.advance(60));
    assert_eq!(toasts.texts().collect::<Vec<_>>(), ["b"]);
    toasts.dismiss(a);
    assert_eq!(toasts.len(), 1);
}

#[test]
fn toast_view_keeps_identity_and_dismisses_on_click() {
    let _guard = lock();
    let mut app = Host::new(|_, _| label("view"));
    app.toasts.push("Saved", 100);
    let second = app.toasts.push("Synced", 1000);
    let (mut ui, _) = mount(&app);
    let stack = child(overlay_tree().expect("overlay mounted"), 0);
    assert!(!has_flag(stack, CLICKABLE));
    assert_eq!(child_count(stack), 2);
    let synced = child(stack, 1);
    assert_eq!(label_text(synced), "Synced");
    assert!(has_flag(synced, CLICKABLE));

    // The older toast expires; the newer keeps its widget.
    assert!(app.toasts.advance(150));
    ui.update(&app).unwrap();
    assert_eq!(child_count(stack), 1);
    assert_eq!(child(stack, 0), synced);

    click(synced);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.received, vec![Msg::Dismiss(second)]);
    assert!(overlay_tree().is_none());
}

#[test]
fn components_compose_with_map() {
    #[derive(Clone, Debug, PartialEq)]
    enum Row {
        Picked(u32),
    }

    let _guard = lock();
    let mut app = Host::new(|_, theme| {
        let rows: View<Row> = list::list(
            theme,
            [ListItem::new("Pick").on_click(Row::Picked(3)).view(theme)],
        );
        rows.map(|Row::Picked(index)| Msg::Open(index))
    });
    let (mut ui, list) = mount(&app);
    click(child(list, 0));
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.received, vec![Msg::Open(3)]);
}

#[test]
fn file_explorer_preserves_display_names_and_emits_entry_identity() {
    use rs_lvgl_ui::components::file_explorer::{self, FileEntry};
    let _lock = lock();
    let mut app = Host::new(|_, theme| {
        file_explorer::view(
            theme,
            "/assets",
            Some(Msg::Open(0)),
            [
                FileEntry::directory(7, "中文 directory", Msg::Open(7)),
                FileEntry::file(8, "a .. b.txt", Msg::Open(8)),
            ],
        )
    });
    let (mut ui, browser) = mount(&app);
    let entries = child(browser, 1);
    let file = child(entries, 1);
    assert_eq!(label_text(child(file, 1)), "a .. b.txt");
    click(file);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.received, [Msg::Open(8)]);
}
