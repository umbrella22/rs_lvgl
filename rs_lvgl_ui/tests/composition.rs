//! Lazy composition preserves order, keyed objects, focus and capture lifetimes.
mod common;

use common::{app_root, child, init, lock};
use rs_lvgl_ui::prelude::*;
use std::{cell::Cell, rc::Rc};

struct Capture(Rc<Cell<usize>>);
impl Drop for Capture {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

struct Page {
    show: bool,
    reverse: bool,
    drops: Rc<Cell<usize>>,
}
impl App for Page {
    type Action = ();
    fn update(&mut self, _: ()) {}
    fn view(&self, _: &Theme) -> View<()> {
        column([])
            .child_if(self.show, || {
                let capture = Capture(self.drops.clone());
                button("optional")
                    .key(Key::Named("optional"))
                    .on_event(move |_| {
                        std::hint::black_box(&capture);
                        None
                    })
            })
            .children((0..3).map(|index| {
                let id = if self.reverse { 2 - index } else { index };
                item(id)
            }))
    }
}

fn item(id: u64) -> View<()> {
    button(format!("item {id}")).key(Key::Id(id)).on_click(())
}

#[test]
fn false_branch_is_lazy_and_accepts_borrowed_fn_once_capture() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    struct Lazy;
    impl App for Lazy {
        type Action = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            let name = String::from("temporary");
            column([label("first")])
                .child_if(false, || -> View<()> {
                    panic!("false branch must not construct a child")
                })
                .child_if(true, || label(name))
                .children((0..2).map(|_| label("last")))
        }
    }
    ui.mount(&Lazy).unwrap();
    // SAFETY: the application root is live for this mounted UI.
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_obj_get_child_count(app_root().as_ptr()) },
        4
    );
}

#[test]
fn removing_optional_child_drops_capture_and_reorders_keyed_siblings_in_place() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    #[cfg(feature = "input-keypad")]
    ui.enable_focus().unwrap();
    let drops = Rc::new(Cell::new(0));
    let mut page = Page {
        show: true,
        reverse: false,
        drops: drops.clone(),
    };
    ui.mount(&page).unwrap();
    let original = [
        child(app_root(), 1),
        child(app_root(), 2),
        child(app_root(), 3),
    ];
    #[cfg(feature = "input-keypad")]
    assert!(ui.focus(Key::Id(1)).unwrap());
    page.show = false;
    page.reverse = true;
    ui.update(&page).unwrap();
    assert_eq!(drops.get(), 1, "removed handler capture is released once");
    for index in 0..3 {
        assert_eq!(child(app_root(), index), original[(2 - index) as usize]);
    }
    #[cfg(feature = "input-keypad")]
    // SAFETY: the surviving keyed object is live and belongs to this UI's group.
    unsafe {
        let group = rs_lvgl_sys::lv_obj_get_group(original[1].as_ptr());
        assert!(!group.is_null());
        assert_eq!(
            rs_lvgl_sys::lv_group_get_focused(group),
            original[1].as_ptr()
        );
    }
    ui.unmount();
    assert_eq!(drops.get(), 1);
}
