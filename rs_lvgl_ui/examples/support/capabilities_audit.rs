use super::*;
#[path = "audit.rs"]
mod native;
use rs_lvgl::input_device::key;

fn app() -> Gallery {
    Gallery {
        page: 0,
        next_page: None,
        dialog: false,
        snapshot: false,
        text: "你好".into(),
        status: "Ready".into(),
        quit: false,
    }
}

#[test]
fn footer_and_modal_focus_render_complete_and_restore_identically() {
    let _lock = native::LOCK.lock().unwrap();
    let display = native::init(960, 760);
    let keyboard = native::keys::Keyboard::new(&display);
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = app();
    ui.mount(&app).unwrap();
    ui.bind_keypad(&keyboard.device).unwrap();
    native::settle(&mut ui);
    let before = native::capture(native::screen(), "capabilities-tab-focus");
    ui.focus(Key::Named("dialog-trigger")).unwrap();
    keyboard.send(key::NEXT);
    keyboard.send(key::PREV);
    native::settle(&mut ui);
    let trigger = native::parent(native::label(native::screen(), "Open dialog"));
    let focused = native::capture(native::screen(), "capabilities-trigger-focused");
    native::outline_changed(&before, &focused, trigger);
    keyboard.send(key::ENTER);
    ui.drain(&mut app, 64).unwrap();
    assert!(app.dialog);
    native::settle(&mut ui);
    let cancel = native::capture(native::top(), "capabilities-dialog-cancel");
    keyboard.send(key::NEXT);
    native::settle(&mut ui);
    let next = native::capture(native::top(), "capabilities-dialog-continue");
    let confirm = native::parent(native::label(native::top(), "Continue"));
    native::outline_changed(&cancel, &next, confirm);
    let group = unsafe { rs_lvgl_sys::lv_indev_get_group(keyboard.device.raw().unwrap().as_ptr()) };
    assert_eq!(unsafe { rs_lvgl_sys::lv_group_get_obj_count(group) }, 2);
    keyboard.send(key::ENTER);
    ui.drain(&mut app, 64).unwrap();
    assert!(!app.dialog);
    assert_eq!(unsafe { rs_lvgl_sys::lv_group_get_focused(group) }, trigger);
    native::settle(&mut ui);
    let restored = native::capture(native::screen(), "capabilities-trigger-restored");
    let [x1, y1, x2, y2] = native::rect(trigger);
    for y in y1 - 6..=y2 + 6 {
        for x in x1 - 6..=x2 + 6 {
            assert_eq!(
                native::pixel(&focused, x, y),
                native::pixel(&restored, x, y),
                "keyboard/restored trigger differs at {x},{y}"
            );
        }
    }
}

#[test]
fn all_pages_render_and_file_actions_survive_navigation() {
    let _lock = native::LOCK.lock().unwrap();
    let display = native::init(960, 760);
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = app();
    ui.mount(&app).unwrap();
    for (page, name) in ["styles", "canvas", "media", "text-files"]
        .iter()
        .enumerate()
    {
        ui.post(Msg::Page(page)).unwrap();
        ui.drain(&mut app, 64).unwrap();
        app.page = app.next_page.take().unwrap();
        ui.update(&app).unwrap();
        native::settle(&mut ui);
        let snapshot = ui.snapshot().unwrap();
        assert_eq!((snapshot.width, snapshot.height), (960, 760));
        native::capture(native::screen(), &format!("capabilities-{name}"));
    }
    let mut pointer = native::Pointer::new();
    // File entries are application-owned; clicking only updates the demo status.
    pointer.click(native::parent(native::label(
        native::screen(),
        "moving-square.json",
    )));
    ui.drain(&mut app, 64).unwrap();
    assert_eq!(app.status, "Selected entry 2");
}

#[test]
fn style_cards_leave_room_for_outline_and_shadow() {
    let _lock = native::LOCK.lock().unwrap();
    let display = native::init(960, 760);
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    ui.mount(&app()).unwrap();
    native::settle(&mut ui);
    let outlined = native::parent(native::label(native::screen(), "OUTLINE / TEXT"));
    // Compare the actual example's rendered outline with that same object
    // temporarily unoutlined. No copied or simplified UI fixture is involved.
    unsafe { rs_lvgl_sys::lv_obj_set_style_outline_width(outlined, 0, 0) };
    let before = native::capture(native::screen(), "capabilities-card-without-outline");
    unsafe { rs_lvgl_sys::lv_obj_set_style_outline_width(outlined, 1, 0) };
    let after = native::capture(native::screen(), "capabilities-card-outline");
    native::outline_changed(&before, &after, outlined);
    let shadowed = native::parent(native::label(native::screen(), "GRADIENT / SHADOW"));
    let [x1, _, x2, y2] = native::rect(shadowed);
    let shadow = native::pixel(&after, (x1 + x2) / 2, y2 + 8);
    let background = native::pixel(&after, 10, y2 + 8);
    assert!(
        shadow[..3].iter().zip(&background[..3]).all(|(a, b)| a < b),
        "card's lower shadow is clipped: shadow={shadow:?}, background={background:?}"
    );
}
