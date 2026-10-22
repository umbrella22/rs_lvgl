use super::*;
#[path = "coverage.rs"]
mod coverage;
#[path = "../support/audit.rs"]
mod fixture;

fn mount() -> (rs_lvgl::Display, Studio, Ui<Msg>) {
    let display = fixture::init(WIDTH as u32, HEIGHT as u32);
    let app = Studio::default();
    let mut ui = Ui::new(&display, palette(false)).unwrap();
    apply_theme(&mut ui, &app);
    ui.mount(&app).unwrap();
    (display, app, ui)
}
fn click(pointer: &mut fixture::Pointer, ui: &mut Ui<Msg>, app: &mut Studio, label: &str) {
    pointer.click(fixture::parent(fixture::label(fixture::screen(), label)));
    ui.drain(app, 128)
        .unwrap_or_else(|error| panic!("{label} on page {}: {error}", app.page));
}
fn page(pointer: &mut fixture::Pointer, ui: &mut Ui<Msg>, app: &mut Studio, index: usize) {
    click(
        pointer,
        ui,
        app,
        &format!("{:02}   {}", index + 1, PAGES[index].0),
    );
    assert_eq!(
        app.page, index,
        "sidebar must navigate through native input"
    );
    fixture::settle(ui);
}
fn assert_in_display(obj: fixture::Obj, name: &str) {
    let [x1, y1, x2, y2] = fixture::rect(obj);
    assert!(
        x1 >= 0 && y1 >= 0 && x2 < WIDTH && y2 < HEIGHT && x2 >= x1 && y2 >= y1,
        "{name} outside display: {x1},{y1}..{x2},{y2}"
    );
}

#[test]
fn every_category_mounts_real_widgets_and_tears_down_three_times() {
    let _lock = fixture::LOCK.lock().unwrap();
    for pass in 0..3 {
        let (display, mut app, mut ui) = mount();
        let mut pointer = fixture::Pointer::new();
        for index in 0..PAGES.len() {
            page(&mut pointer, &mut ui, &mut app, index);
            if pass == 0 {
                fixture::capture(fixture::screen(), &format!("studio-{index}"));
            }
            for (name, class) in coverage::expected(index) {
                let obj = fixture::find(fixture::screen(), &|obj| unsafe {
                    rs_lvgl_sys::lv_obj_check_type(obj, class)
                })
                .unwrap_or_else(|| panic!("page {index} is missing native {name}"));
                // Pinyin's helper object is intentionally hidden; its keyboard
                // and candidates are exercised separately with actual input.
                if name != "ime_pinyin" {
                    assert_in_display(obj, name);
                }
            }
        }
        page(&mut pointer, &mut ui, &mut app, 0);
        click(&mut pointer, &mut ui, &mut app, "Dark mode");
        assert!(app.dark);
        apply_theme(&mut ui, &app);
        ui.update(&app).unwrap();
        fixture::settle(&mut ui);
        if pass == 0 {
            fixture::capture(fixture::screen(), "studio-dark");
        }
        ui.unmount();
        drop(ui);
        drop(pointer);
        unsafe {
            display.delete();
        }
        assert!(unsafe { rs_lvgl_sys::lv_display_get_default() }.is_null());
    }
}

#[test]
fn controls_keyboard_navigation_and_modal_actions_change_application_state() {
    let _lock = fixture::LOCK.lock().unwrap();
    let (display, mut app, mut ui) = mount();
    let mut pointer = fixture::Pointer::new();
    page(&mut pointer, &mut ui, &mut app, 1);
    let slider = fixture::class(fixture::screen(), coverage::expected(1)[0].1);
    let [x1, y1, x2, y2] = fixture::rect(slider);
    pointer.read(x1 + (x2 - x1) * 3 / 4, (y1 + y2) / 2, true);
    pointer.read(x1 + (x2 - x1) * 3 / 4, (y1 + y2) / 2, false);
    ui.drain(&mut app, 128).unwrap();
    assert!(
        (70..=80).contains(&app.value),
        "slider value: {}",
        app.value
    );
    let previous = app.value;
    click(&mut pointer, &mut ui, &mut app, "+");
    assert_eq!(app.value, previous + 1);
    let switch = fixture::class(fixture::screen(), coverage::expected(1)[2].1);
    pointer.click(switch);
    ui.drain(&mut app, 128).unwrap();
    assert!(!app.enabled);
    click(&mut pointer, &mut ui, &mut app, "Unavailable");
    assert_eq!(app.clicks, 0, "disabled actions must remain disabled");

    page(&mut pointer, &mut ui, &mut app, 2);
    let keyboard = fixture::class(fixture::screen(), &raw const fixture::lv_keyboard_class);
    let textarea = fixture::class(fixture::screen(), &raw const fixture::lv_textarea_class);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_keyboard_get_textarea(keyboard) },
        textarea
    );
    // Select a real native keyboard key and dispatch the same value event
    // the keyboard's button matrix emits on activation.
    let key = (0..100)
        .find(|id| unsafe {
            let text = rs_lvgl_sys::lv_buttonmatrix_get_button_text(keyboard, *id);
            !text.is_null() && std::ffi::CStr::from_ptr(text).to_bytes() == b"a"
        })
        .unwrap();
    unsafe {
        rs_lvgl_sys::lv_buttonmatrix_set_selected_button(keyboard, key);
        rs_lvgl_sys::lv_obj_send_event(
            keyboard,
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            core::ptr::null_mut(),
        );
    }
    ui.drain(&mut app, 128).unwrap();
    assert_eq!(app.text, "Hello, Rust!a");

    // Both keyboards coexist. Candidate submission must target only the
    // Chinese field and survive reconciliation through the queued text events.
    let ime_class = coverage::expected(2)[2].1;
    let ime = fixture::class(fixture::screen(), ime_class);
    let candidate = unsafe {
        let kb = rs_lvgl_sys::lv_ime_pinyin_get_kb(ime);
        assert_ne!(rs_lvgl_sys::lv_keyboard_get_textarea(kb), textarea);
        for key in [b"n", b"i"] {
            let id = (0..100)
                .find(|id| {
                    let text = rs_lvgl_sys::lv_buttonmatrix_get_button_text(kb, *id);
                    !text.is_null() && std::ffi::CStr::from_ptr(text).to_bytes() == key
                })
                .expect("Pinyin key");
            rs_lvgl_sys::lv_buttonmatrix_set_selected_button(kb, id);
            rs_lvgl_sys::lv_obj_send_event(
                kb,
                rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
                core::ptr::null_mut(),
            );
        }
        let panel = rs_lvgl_sys::lv_ime_pinyin_get_cand_panel(ime);
        assert!(!rs_lvgl_sys::lv_obj_is_hidden(panel));
        let candidate =
            std::ffi::CStr::from_ptr(rs_lvgl_sys::lv_buttonmatrix_get_button_text(panel, 1))
                .to_str()
                .unwrap()
                .to_owned();
        assert!(!candidate.is_ascii());
        rs_lvgl_sys::lv_buttonmatrix_set_selected_button(panel, 1);
        rs_lvgl_sys::lv_obj_send_event(
            panel,
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            core::ptr::null_mut(),
        );
        candidate
    };
    ui.drain(&mut app, 128).unwrap();
    assert_eq!(app.chinese, format!("你好{candidate}"));
    assert_eq!(app.text, "Hello, Rust!a", "Pinyin changed the ASCII field");

    page(&mut pointer, &mut ui, &mut app, 4);
    let cells: Vec<_> = (1..=4)
        .map(|i| {
            let obj = fixture::parent(fixture::label(fixture::screen(), &format!("Cell {i}")));
            assert_in_display(obj, "grid cell");
            let [x1, y1, x2, y2] = fixture::rect(obj);
            let [px1, py1, px2, py2] = fixture::rect(fixture::parent(obj));
            assert!(
                x1 >= px1 && y1 >= py1 && x2 <= px2 && y2 <= py2,
                "grid cell clipped by its container"
            );
            [x1, y1, x2, y2]
        })
        .collect();
    assert!(
        cells[0][3] < cells[2][1] && cells[0][2] < cells[1][0],
        "grid cells overlap"
    );
    click(&mut pointer, &mut ui, &mut app, "Next tile");
    assert_eq!(app.tile, 1);
    page(&mut pointer, &mut ui, &mut app, 7);
    click(&mut pointer, &mut ui, &mut app, "Open device");
    assert_eq!(app.nav.depth(), 1);
    click(&mut pointer, &mut ui, &mut app, "Back to workspace");
    assert_eq!(app.nav.depth(), 0);
    let file = fixture::parent(fixture::label(fixture::screen(), "studio-motion.json"));
    pointer.click(file);
    ui.drain(&mut app, 128).unwrap();
    assert_eq!(app.file, 2);

    let keyboard = fixture::keys::Keyboard::new(&display);
    ui.bind_keypad(&keyboard.device).unwrap();
    click(&mut pointer, &mut ui, &mut app, "Open dialog");
    assert!(app.dialog);
    assert_in_display(fixture::label(fixture::top(), "Ready to build?"), "dialog");
    pointer.click(fixture::parent(fixture::label(fixture::top(), "Let's go")));
    ui.drain(&mut app, 128).unwrap();
    assert!(!app.dialog);
    click(&mut pointer, &mut ui, &mut app, "Show toast");
    assert_eq!(app.toasts.len(), 1);
    assert!(app.toasts.advance(3201));
    ui.update(&app).unwrap();
    assert!(app.toasts.is_empty());

    page(&mut pointer, &mut ui, &mut app, 8);
    pointer.click(fixture::parent(fixture::label(
        fixture::screen(),
        "New workspace",
    )));
    ui.drain(&mut app, 128).unwrap();
    assert!(app.clicks > 0, "raw child click must reach Rust");
    pointer.click(fixture::parent(fixture::label(
        fixture::screen(),
        "Display  >",
    )));
    ui.drain(&mut app, 128).unwrap();
    assert_in_display(
        fixture::label(fixture::screen(), "1280 x 860\nRGB color\nNative rendering"),
        "native menu page",
    );
    ui.unmount();
    drop(ui);
    drop(keyboard);
    drop(pointer);
    unsafe {
        display.delete();
    }
}

#[test]
fn action_focus_outlines_render_on_all_four_sides() {
    let _lock = fixture::LOCK.lock().unwrap();
    let (display, mut app, mut ui) = mount();
    let mut pointer = fixture::Pointer::new();
    for (index, caption) in [(0, "Dark mode"), (7, "Open dialog")] {
        page(&mut pointer, &mut ui, &mut app, index);
        let obj = fixture::parent(fixture::label(fixture::screen(), caption));
        unsafe {
            rs_lvgl_sys::lv_obj_remove_state(obj, rs_lvgl_sys::lv_state_t_LV_STATE_FOCUSED);
        }
        fixture::settle(&mut ui);
        let before = fixture::capture(fixture::screen(), &format!("studio-focus-{index}-before"));
        unsafe {
            rs_lvgl_sys::lv_obj_add_state(obj, rs_lvgl_sys::lv_state_t_LV_STATE_FOCUSED);
        }
        fixture::settle(&mut ui);
        let after = fixture::capture(fixture::screen(), &format!("studio-focus-{index}-after"));
        fixture::outline_changed(&before, &after, obj);
    }
    ui.unmount();
    drop(ui);
    drop(pointer);
    unsafe {
        display.delete();
    }
}
