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
    ui.post(FanSettingsAction::SpeedChanged(85)).unwrap();
    ui.post(FanSettingsAction::MutedChanged(true)).unwrap();
    ui.post(FanSettingsAction::Select(2)).unwrap();
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

#[test]
fn controls_and_save_focus_keep_their_complete_drawing_bounds() {
    let _lock = native::LOCK.lock().unwrap();
    for theme in [Theme::light(), Theme::dark()] {
        let display = native::init(320, 240);
        let mut ui = Ui::new(&display, theme).unwrap();
        ui.mount(&Settings::new()).unwrap();
        let slider = native::class(native::screen(), &raw const native::lv_slider_class);
        native::slider_knob_unclipped(slider, "settings-slider");
        let save = native::parent(native::label(native::screen(), "Save"));
        // SAFETY: the mounted button stays live and scrolling is synchronous.
        unsafe {
            rs_lvgl_sys::lv_obj_scroll_to_view_recursive(save, false);
            rs_lvgl_sys::lv_obj_remove_state(save, rs_lvgl_sys::lv_state_t_LV_STATE_FOCUS_KEY);
        }
        native::settle(&mut ui);
        let before = native::capture(native::screen(), "settings-save-unfocused");
        // SAFETY: only the live button's native focus appearance is under test.
        unsafe { rs_lvgl_sys::lv_obj_add_state(save, rs_lvgl_sys::lv_state_t_LV_STATE_FOCUS_KEY) };
        native::settle(&mut ui);
        let after = native::capture(native::screen(), "settings-save-focused");
        native::outline_changed(&before, &after, save);
        ui.unmount();
        drop(ui);
        // SAFETY: no UI or asynchronous display work remains on this memory display.
        unsafe { display.delete() };
    }
}
