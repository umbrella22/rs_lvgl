use super::*;
#[path = "audit.rs"]
mod native;
use rs_lvgl::input_device::key;

#[test]
fn keyboard_focus_is_visible_on_all_sides_and_actions_and_scroll_work() {
    let _lock = native::LOCK.lock().unwrap();
    let display = native::init(640, 420);
    let keyboard = native::keys::Keyboard::new(&display);
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = InputDemo {
        count: 0,
        last: "Ready".into(),
    };
    ui.mount(&app).unwrap();
    ui.bind_keypad(&keyboard.device).unwrap();
    ui.focus(Key::Named("add")).unwrap();
    native::settle(&mut ui);
    let before = native::capture(native::screen(), "input-add");
    keyboard.send(key::NEXT);
    native::settle(&mut ui);
    let focused = native::capture(native::screen(), "input-reset-focused");
    let reset = native::parent(native::label(native::screen(), "Reset"));
    native::outline_changed(&before, &focused, reset);
    keyboard.send(key::PREV);
    keyboard.send(key::ENTER);
    ui.drain(&mut app, 64).unwrap();
    assert_eq!(app.count, 1);
    keyboard.send(key::NEXT);
    keyboard.send(key::ENTER);
    ui.drain(&mut app, 64).unwrap();
    assert_eq!(app.count, 0);
    let list = native::parent(native::label(native::screen(), "Scrollable item 1"));
    native::settle(&mut ui);
    let before_list = native::capture(native::screen(), "input-before-list-focus");
    keyboard.send(key::NEXT);
    native::settle(&mut ui);
    let list_focus = native::capture(native::screen(), "input-list-focused");
    native::outline_changed(&before_list, &list_focus, list);
    keyboard.send(key::DOWN);
    ui.drain(&mut app, 64).unwrap();
    native::settle(&mut ui);
    assert!(unsafe { rs_lvgl_sys::lv_obj_get_scroll_y(list) } > 0);
    unsafe { rs_lvgl_sys::lv_obj_scroll_to_y(list, 0, false) };
    ui.drain(&mut app, 64).unwrap();
    let [x1, y1, _, y2] = native::rect(list);
    let mut pointer = native::Pointer::new();
    for y in (y1 + 8..y2 - 5).rev().step_by(5) {
        pointer.read(x1 + 30, y, true);
    }
    pointer.read(x1 + 30, y1 + 8, false);
    ui.drain(&mut app, 64).unwrap();
    assert!(unsafe { rs_lvgl_sys::lv_obj_get_scroll_y(list) } > 0);
    assert!(app.last.starts_with("Scroll"));
    native::capture(native::screen(), "input-scrolled");
}
