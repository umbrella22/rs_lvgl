//! Composite slots retain styles, keys and mapped action output.
#![cfg(feature = "components")]
mod common;
use common::{app_root, child, init, lock};
use rs_lvgl_ui::components::{dialog::Dialog, list::ListItem, panel::Panel};
use rs_lvgl_ui::prelude::*;

#[derive(Clone, Debug, PartialEq)]
struct PageAction(u8);
struct Page {
    make: fn(&Theme) -> View<PageAction>,
    received: Vec<PageAction>,
}
impl App for Page {
    type Action = PageAction;
    fn update(&mut self, action: PageAction) {
        self.received.push(action);
    }
    fn view(&self, theme: &Theme) -> View<PageAction> {
        (self.make)(theme)
    }
}
fn click(node: core::ptr::NonNull<rs_lvgl_sys::lv_obj_t>) {
    // SAFETY: the node is in the current mounted component.
    unsafe {
        rs_lvgl_sys::lv_obj_send_event(
            node.as_ptr(),
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED,
            core::ptr::null_mut(),
        );
    }
}

#[test]
fn list_slots_are_direct_children_with_independent_mapped_actions() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut page = Page {
        received: vec![],
        make: |theme| {
            ListItem::new("replaced text")
                .leading(button("leading").on_click(1))
                .content(button("main").on_click(2))
                .trailing(
                    button("trailing")
                        .key(Key::Named("user-trailing"))
                        .on_click(3),
                )
                .view(theme)
                .map(PageAction)
        },
    };
    ui.mount(&page).unwrap();
    assert_eq!(
        ui.stats().mounted_nodes,
        4,
        "slot naming adds no native containers"
    );
    for index in 0..3 {
        click(child(app_root(), index));
    }
    ui.drain(&mut page, 8).unwrap();
    assert_eq!(page.received, [PageAction(1), PageAction(2), PageAction(3)]);
    #[cfg(feature = "input-keypad")]
    {
        ui.enable_focus().unwrap();
        assert!(ui.focus(Key::Named("user-trailing")).unwrap());
    }
}

#[test]
fn panel_distinguishes_header_widgets_from_body_and_maps_both() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut page = Page {
        received: vec![],
        make: |theme| {
            Panel::new("title")
                .header_action(button("header").on_click(1))
                .header_actions([button("second").on_click(2)])
                .child(button("replaced").on_click(9))
                .content(button("body").on_click(3))
                .children([button("appended").on_click(4)])
                .view(theme)
                .map(PageAction)
        },
    };
    ui.mount(&page).unwrap();
    let header = child(app_root(), 0);
    let body = child(app_root(), 1);
    click(child(header, 1));
    click(child(header, 2));
    click(child(body, 0));
    click(child(body, 1));
    ui.drain(&mut page, 8).unwrap();
    assert_eq!(
        page.received,
        [PageAction(1), PageAction(2), PageAction(3), PageAction(4)]
    );
}

#[test]
fn dialog_replaces_body_then_appends_children_and_emits_footer_actions() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut page = Page {
        received: vec![],
        make: |theme| {
            Dialog::new("title")
                .text("replaced")
                .content(button("content").on_click(1))
                .children([button("appended").on_click(2)])
                .primary("confirm", 3)
                .view(theme)
                .map(PageAction)
        },
    };
    ui.mount(&page).unwrap();
    let card = child(app_root(), 0);
    click(child(card, 1));
    click(child(card, 2));
    click(child(child(card, 3), 0));
    ui.drain(&mut page, 8).unwrap();
    assert_eq!(page.received, [PageAction(1), PageAction(2), PageAction(3)]);
}
