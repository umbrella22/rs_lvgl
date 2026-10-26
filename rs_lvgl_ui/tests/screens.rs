mod common;
use rs_lvgl_ui::prelude::*;
struct Page {
    invalid: bool,
    overlay: bool,
}
impl App for Page {
    type Msg = ();
    fn update(&mut self, _: ()) {}
    fn view(&self, _: &Theme) -> View<()> {
        button("Page").style(Style::default().width(Size::Px(if self.invalid { -1 } else { 40 })))
    }
    fn overlay(&self, _: &Theme) -> Option<View<()>> {
        self.overlay.then(|| button("Overlay"))
    }
}
fn advance(screens: &mut Screens<()>, ms: u32) {
    unsafe {
        rs_lvgl_sys::lv_tick_inc(ms);
        rs_lvgl_sys::lv_anim_refr_now();
    }
    screens.timer_handler();
}
#[test]
fn transition_retains_then_releases_outgoing_page_and_restores_original() {
    let _guard = common::lock();
    let display = common::init();
    let original = common::screen();
    let mut screens = Screens::new(&display, common::theme()).unwrap();
    let page = Page {
        invalid: false,
        overlay: true,
    };
    screens.show(&page, ScreenTransition::immediate()).unwrap();
    let first = common::screen();
    assert_ne!(first, original);
    screens
        .show(&page, ScreenTransition::new(ScreenEffect::MoveLeft, 100))
        .unwrap();
    let second = common::screen();
    assert_ne!(first, second);
    assert_eq!(screens.retiring_pages(), 1);
    advance(&mut screens, 50);
    unsafe {
        let x = rs_lvgl_sys::lv_obj_get_x(second.as_ptr());
        assert!(x > 0 && x < 64, "intermediate x={x}");
        let top = rs_lvgl_sys::lv_layer_top();
        assert_eq!(rs_lvgl_sys::lv_obj_get_child_count(top), 2);
        assert!(rs_lvgl_sys::lv_obj_is_hidden(
            rs_lvgl_sys::lv_obj_get_child(top, 0)
        ));
    }
    advance(&mut screens, 100);
    assert_eq!(screens.retiring_pages(), 0);
    unsafe {
        assert_eq!(rs_lvgl_sys::lv_obj_get_x(second.as_ptr()), 0);
    }
    screens.unmount();
    assert_eq!(common::screen(), original);
    unsafe {
        assert_eq!(
            rs_lvgl_sys::lv_obj_get_child_count(rs_lvgl_sys::lv_layer_top()),
            0
        );
        assert_eq!(rs_lvgl_sys::lv_anim_count_running(), 0);
        rs_lvgl::deinit();
    }
    drop(screens);
}
#[test]
fn interrupted_transition_and_failed_show_preserve_a_valid_page() {
    let _guard = common::lock();
    for _ in 0..3 {
        let display = common::init();
        let original = common::screen();
        let mut screens = Screens::new(&display, common::theme()).unwrap();
        for _ in 0..8 {
            screens
                .show(
                    &Page {
                        invalid: false,
                        overlay: true,
                    },
                    ScreenTransition::new(ScreenEffect::FadeIn, 100),
                )
                .unwrap();
            advance(&mut screens, 20);
            assert!(screens.retiring_pages() <= 1);
        }
        let active = common::screen();
        assert_eq!(
            screens.show(
                &Page {
                    invalid: true,
                    overlay: true
                },
                ScreenTransition::immediate()
            ),
            Err(UiError::InvalidProperty)
        );
        assert_eq!(common::screen(), active);
        assert_eq!(
            screens.show(
                &Page {
                    invalid: false,
                    overlay: false
                },
                ScreenTransition::new(ScreenEffect::FadeOut, u32::MAX)
            ),
            Err(UiError::InvalidProperty)
        );
        screens.unmount();
        assert_eq!(common::screen(), original);
        assert_eq!(unsafe { rs_lvgl_sys::lv_anim_count_running() }, 0);
    }
}
#[test]
fn screen_creation_respects_supplied_display_and_restores_default() {
    let _guard = common::lock();
    let display = common::init();
    let original = common::screen();
    let other = rs_lvgl::Display::register::<_, { 32 * 32 }>(32, 32, |_| {}).unwrap();
    let other_raw =
        unsafe { rs_lvgl_sys::lv_obj_get_display(other.get_scr_act().unwrap().raw().as_ptr()) };
    unsafe {
        rs_lvgl_sys::lv_display_set_default(other_raw);
    }
    let mut screens = Screens::new(&display, common::theme()).unwrap();
    screens
        .show(
            &Page {
                invalid: false,
                overlay: false,
            },
            ScreenTransition::immediate(),
        )
        .unwrap();
    unsafe {
        assert_eq!(rs_lvgl_sys::lv_display_get_default(), other_raw);
        assert_ne!(
            rs_lvgl_sys::lv_display_get_screen_active(rs_lvgl_sys::lv_obj_get_display(
                original.as_ptr()
            )),
            original.as_ptr()
        );
    }
    screens.unmount();
}
use rs_lvgl::NativeObject;

#[cfg(feature = "input-keypad")]
#[test]
fn switching_detaches_old_group_and_suspends_keypad_input_until_completion() {
    use rs_lvgl::input_device::{key, Keypad, KeypadInputData};
    use std::{cell::Cell, rc::Rc};
    struct Clicks(usize);
    impl App for Clicks {
        type Msg = ();
        fn update(&mut self, _: ()) {
            self.0 += 1;
        }
        fn view(&self, _: &Theme) -> View<()> {
            button("Click").on_click(())
        }
    }
    let _guard = common::lock();
    let display = common::init();
    let sample = Rc::new(Cell::new(false));
    let source = sample.clone();
    let device = Keypad::register(
        move || {
            if source.get() {
                KeypadInputData::pressed(key::ENTER)
            } else {
                KeypadInputData::released(key::ENTER)
            }
        },
        &display,
    )
    .unwrap();
    let mut app = Clicks(0);
    let mut screens = Screens::new(&display, common::theme()).unwrap();
    screens.show(&app, ScreenTransition::immediate()).unwrap();
    screens.current_mut().unwrap().bind_keypad(&device).unwrap();
    screens
        .show(&app, ScreenTransition::new(ScreenEffect::MoveLeft, 100))
        .unwrap();
    assert!(unsafe { rs_lvgl_sys::lv_indev_get_group(device.raw().unwrap().as_ptr()) }.is_null());
    screens.current_mut().unwrap().bind_keypad(&device).unwrap();
    let send = || {
        for pressed in [true, false] {
            sample.set(pressed);
            unsafe {
                rs_lvgl_sys::lv_indev_read(device.raw().unwrap().as_ptr());
            }
        }
    };
    send();
    assert_eq!(
        screens
            .current_mut()
            .unwrap()
            .drain(&mut app, 8)
            .unwrap()
            .processed,
        0
    );
    advance(&mut screens, 150);
    send();
    assert_eq!(
        screens
            .current_mut()
            .unwrap()
            .drain(&mut app, 8)
            .unwrap()
            .processed,
        1
    );
    assert_eq!(app.0, 1);
    screens.unmount();
}

#[test]
fn one_controller_per_display_and_unmount_releases_the_claim() {
    let _guard = common::lock();
    let display = common::init();
    let mut first = Screens::<()>::new(&display, common::theme()).unwrap();
    let page = Page {
        invalid: false,
        overlay: false,
    };
    first.show(&page, ScreenTransition::immediate()).unwrap();
    assert!(matches!(
        Screens::<()>::new(&display, common::theme()),
        Err(UiError::InvalidProperty)
    ));
    first.unmount();
    let mut second = Screens::<()>::new(&display, common::theme()).unwrap();
    assert_eq!(
        first.show(&page, ScreenTransition::immediate()),
        Err(UiError::InvalidProperty)
    );
    second.unmount();
    first.show(&page, ScreenTransition::immediate()).unwrap();
    first.unmount();
    // A controller that never mounts must release its display registration too.
    drop(Screens::<()>::new(&display, common::theme()).unwrap());
    let mut third = Screens::<()>::new(&display, common::theme()).unwrap();
    third.unmount();
    unsafe {
        rs_lvgl::deinit();
    }
    drop(first);
    drop(second);
    drop(third);
}
