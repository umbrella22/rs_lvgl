use super::*;
#[path = "audit.rs"]
mod native;

#[test]
fn pointer_actions_preserve_keyed_rows_and_theme_updates() {
    let _lock = native::LOCK.lock().unwrap();
    let display = native::init(WIDTH as u32, HEIGHT as u32);
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = Gallery::new();
    ui.mount(&app).unwrap();
    native::settle(&mut ui);
    native::capture(native::screen(), "gallery-dark");
    let mut pointer = native::Pointer::new();
    let plus = native::parent(native::label(native::screen(), "+"));
    let [x1, y1, x2, y2] = native::rect(plus);
    let normal = native::capture(native::screen(), "gallery-normal");
    pointer.read((x1 + x2) / 2, (y1 + y2) / 2, true);
    native::settle(&mut ui);
    let pressed = native::capture(native::screen(), "gallery-pressed");
    assert_ne!(
        native::pixel(&normal, x1 + 6, y1 + 6),
        native::pixel(&pressed, x1 + 6, y1 + 6)
    );
    pointer.read((x1 + x2) / 2, (y1 + y2) / 2, false);
    ui.drain(&mut app, 32).unwrap();
    assert_eq!(app.count, 1);
    let first = native::parent(native::label(native::screen(), "Item 1"));
    pointer.click(native::parent(native::label(first, "Move down")));
    ui.drain(&mut app, 32).unwrap();
    assert_eq!(&app.items[..2], &[2, 1]);
    assert_eq!(
        native::parent(native::label(native::screen(), "Item 1")),
        first
    );
    pointer.click(native::parent(native::label(native::screen(), "Add item")));
    ui.drain(&mut app, 32).unwrap();
    assert_eq!(app.items.len(), 5);
    native::capture(native::screen(), "gallery-list-scrolled");
    pointer.click(native::parent(native::label(native::screen(), "Theme")));
    ui.drain(&mut app, 32).unwrap();
    assert!(!app.dark);
    ui.set_theme(Theme::light());
    ui.update(&app).unwrap();
    native::settle(&mut ui);
    native::capture(native::screen(), "gallery-light");
    #[cfg(feature = "input-keypad")]
    assert!(
        !unsafe { rs_lvgl_sys::lv_obj_has_state(plus, rs_lvgl_sys::lv_state_t_LV_STATE_FOCUS_KEY) },
        "enabling the feature must not activate keyboard focus in pointer-only examples"
    );
}
