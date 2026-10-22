use super::*;
#[path = "audit.rs"]
mod native;

#[test]
fn navigation_dialog_choices_and_toast_expiry() {
    let _lock = native::LOCK.lock().unwrap();
    let display = native::init(WIDTH as u32, HEIGHT as u32);
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = Demo::new();
    ui.mount(&app).unwrap();
    let mut pointer = native::Pointer::new();
    native::capture(native::screen(), "components-root");
    pointer.click(native::parent(native::label(native::screen(), "Display")));
    ui.drain(&mut app, 32).unwrap();
    assert_eq!(app.nav.current(), &Page::Display);
    pointer.click(native::parent(native::label(native::screen(), "High")));
    ui.drain(&mut app, 32).unwrap();
    assert_eq!(app.brightness, Brightness::High);
    native::capture(native::screen(), "components-display");
    pointer.click(native::parent(native::label(
        native::screen(),
        symbol::LEFT,
    )));
    ui.drain(&mut app, 32).unwrap();
    assert_eq!(app.nav.current(), &Page::Root);
    pointer.click(native::parent(native::label(native::screen(), "Reset")));
    ui.drain(&mut app, 32).unwrap();
    assert!(app.confirm_reset);
    native::capture(native::top(), "components-dialog");
    pointer.click(native::parent(native::label(native::top(), "Cancel")));
    ui.drain(&mut app, 32).unwrap();
    assert!(!app.confirm_reset);
    assert_eq!(app.brightness, Brightness::High);
    pointer.click(native::parent(native::label(native::screen(), "Reset")));
    ui.drain(&mut app, 32).unwrap();
    pointer.click(native::parent(native::label(native::top(), "Reset")));
    ui.drain(&mut app, 32).unwrap();
    assert_eq!(app.brightness, Brightness::Medium);
    assert!(!app.toasts.is_empty());
    native::capture(native::top(), "components-toast");
    ui.post(Msg::Tick(2000)).unwrap();
    ui.drain(&mut app, 32).unwrap();
    assert!(app.toasts.is_empty());
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_obj_get_child_count(native::top()) },
        0
    );
    pointer.click(native::parent(native::label(native::screen(), "Logs")));
    ui.drain(&mut app, 32).unwrap();
    assert_eq!(app.nav.current(), &Page::Logs);
    native::capture(native::screen(), "components-logs");
}
