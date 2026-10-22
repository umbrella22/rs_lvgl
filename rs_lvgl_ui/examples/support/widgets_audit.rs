use super::*;
#[path = "audit.rs"]
mod native;

#[test]
fn light_theme_updates_native_tabs_and_retained_calendar() {
    let _lock = native::LOCK.lock().unwrap();
    let display = native::init(WIDTH as u32, HEIGHT as u32);
    let mut app = Demo::new();
    unsafe {
        rs_lvgl_sys::lv_theme_default_init(
            rs_lvgl_sys::lv_display_get_default(),
            raw_color(PALETTE[0]),
            raw_color(PALETTE[1]),
            true,
            &raw const rs_lvgl_sys::lv_font_montserrat_14,
        );
    }
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    ui.mount(&app).unwrap();
    let tabs = native::class(native::screen(), &raw const native::lv_tabview_class);
    let calendar = native::class(native::screen(), &raw const native::lv_calendar_class);
    native::settle(&mut ui);
    native::capture(native::screen(), "widgets-dark");
    let [x1, _, x2, _] = native::rect(calendar);
    assert_eq!(
        x2 - x1 + 1,
        unsafe { rs_lvgl_sys::lv_obj_get_content_width(native::parent(calendar)) },
        "raw calendar must preserve the full form width"
    );
    let mut pointer = native::Pointer::new();
    pointer.click(native::parent(native::label(native::screen(), "Light")));
    ui.drain(&mut app, 64).unwrap();
    assert!(!app.dark);
    apply_theme(&mut ui, &app);
    ui.update(&app).unwrap();
    native::settle(&mut ui);
    let light = native::capture(native::screen(), "widgets-light");
    let bar = unsafe { rs_lvgl_sys::lv_tabview_get_tab_bar(tabs) };
    let [x1, y1, _, _] = native::rect(bar);
    let p = native::pixel(&light, x1 + 180, y1 + 10);
    assert!(
        p[0] > 160 && p[1] > 160 && p[2] > 160,
        "native tab bar stayed dark: {p:?}"
    );
    assert_eq!(
        native::class(native::screen(), &raw const native::lv_calendar_class),
        calendar
    );
    let color = unsafe { rs_lvgl_sys::lv_obj_get_style_bg_color(calendar, 0) };
    assert_eq!(
        (color.red, color.green, color.blue),
        (255, 255, 255),
        "retained calendar kept its old local style"
    );
    app.accent = 2;
    apply_theme(&mut ui, &app);
    ui.update(&app).unwrap();
    native::settle(&mut ui);
    let orange = native::capture(native::screen(), "widgets-light-orange");
    let selected = native::pixel(&orange, x1 + 20, y1 + 10);
    assert!(
        selected[0] > selected[2].saturating_add(20),
        "native selected tab did not follow the orange accent"
    );
    app.dark = true;
    apply_theme(&mut ui, &app);
    ui.update(&app).unwrap();
    native::settle(&mut ui);
    let dark = native::capture(native::screen(), "widgets-dark-restored");
    assert!(native::pixel(&dark, x1 + 180, y1 + 10)[0] < 100);
}

#[test]
fn native_text_keyboard_disabled_controls_and_all_tabs_work() {
    use std::ffi::CStr;
    let _lock = native::LOCK.lock().unwrap();
    let display = native::init(WIDTH as u32, HEIGHT as u32);
    let mut app = Demo::new();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    apply_theme(&mut ui, &app);
    ui.mount(&app).unwrap();
    let mut pointer = native::Pointer::new();
    pointer.click(native::parent(native::label(native::screen(), "Invite")));
    ui.drain(&mut app, 64).unwrap();
    assert_eq!(app.purchases, 0);
    assert_eq!(app.logouts, 0);
    pointer.click(native::parent(native::label(native::screen(), "Log out")));
    ui.drain(&mut app, 64).unwrap();
    assert_eq!(app.logouts, 1);
    let text = native::class(native::screen(), &raw const native::lv_textarea_class);
    pointer.click(text);
    ui.drain(&mut app, 64).unwrap();
    assert_eq!(app.keyboard_for, Some(1));
    let keyboard = native::class(native::screen(), &raw const native::lv_keyboard_class);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_keyboard_get_textarea(keyboard) },
        text
    );
    for letter in ["a", "u", "d", "i", "t"] {
        let id = (0..100)
            .find(|id| unsafe {
                let label = rs_lvgl_sys::lv_buttonmatrix_get_button_text(keyboard, *id);
                !label.is_null() && CStr::from_ptr(label).to_bytes() == letter.as_bytes()
            })
            .expect("native keyboard key");
        unsafe {
            rs_lvgl_sys::lv_buttonmatrix_set_selected_button(keyboard, id);
            rs_lvgl_sys::lv_obj_send_event(
                keyboard,
                rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
                core::ptr::null_mut(),
            );
        }
        ui.drain(&mut app, 64).unwrap();
    }
    assert_eq!(app.username, "audit");
    native::capture(native::screen(), "widgets-keyboard");
    unsafe {
        rs_lvgl_sys::lv_obj_send_event(
            keyboard,
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_READY,
            core::ptr::null_mut(),
        );
    }
    ui.drain(&mut app, 64).unwrap();
    assert_eq!(app.keyboard_for, None);
    for (tab, name) in [(Tab::Analytics, "Analytics"), (Tab::Shop, "Shop")] {
        pointer.click(native::parent(native::label(native::screen(), name)));
        ui.drain(&mut app, 64).unwrap();
        assert_eq!(app.tab, tab);
        native::settle(&mut ui);
        native::capture(
            native::screen(),
            &format!("widgets-{}", name.to_lowercase()),
        );
        let tabs = native::class(native::screen(), &raw const native::lv_tabview_class);
        let content = unsafe { rs_lvgl_sys::lv_tabview_get_content(tabs) };
        let page = native::child(content, app.tab_index() as i32);
        for (label, index, disabled) in [
            ("Item purchased", 0, false),
            ("New message", 3, true),
            ("Milestone reached", 4, true),
        ] {
            let checkbox = native::find(page, &|obj| unsafe {
                rs_lvgl_sys::lv_obj_check_type(obj, &raw const native::lv_checkbox_class)
                    && CStr::from_ptr(rs_lvgl_sys::lv_checkbox_get_text(obj)).to_bytes()
                        == label.as_bytes()
            })
            .unwrap();
            let before = app.notifications[index];
            pointer.click(checkbox);
            ui.drain(&mut app, 64).unwrap();
            assert_eq!(
                app.notifications[index],
                if disabled { before } else { !before }
            );
        }
        native::capture(
            native::screen(),
            &format!("widgets-{}-scrolled", name.to_lowercase()),
        );
        if tab == Tab::Shop {
            pointer.click(native::parent(native::label(page, "Buy")));
            ui.drain(&mut app, 64).unwrap();
            assert_eq!(app.purchases, 1);
            assert_eq!(app.last_buy, Some(0));
        }
    }
}
