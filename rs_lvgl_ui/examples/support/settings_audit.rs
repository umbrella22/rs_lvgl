use super::*;
#[path = "audit.rs"]
mod native;

#[test]
fn memory_consumer_updates_and_keeps_save_reachable_by_scrolling() {
    let _lock = native::LOCK.lock().unwrap();
    let display = native::init(320, 240);
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = Settings::new();
    ui.mount(&app).unwrap();
    native::capture(native::screen(), "settings-initial");
    ui.post(Msg::SpeedChanged(85)).unwrap();
    ui.post(Msg::ToggleMute).unwrap();
    ui.post(Msg::Select(2)).unwrap();
    ui.drain(&mut app, 16).unwrap();
    let mut pointer = native::Pointer::new();
    pointer.click(native::parent(native::label(native::screen(), "Save")));
    ui.drain(&mut app, 16).unwrap();
    assert_eq!(app.speed, 85);
    assert!(app.muted);
    assert_eq!(app.selected, 2);
    assert_eq!(app.saves, 1);
    native::capture(native::screen(), "settings-scrolled");
}
