mod common;
use rs_lvgl_ui::prelude::*;
struct TestApp(usize);
impl App for TestApp {
    type Msg = ();
    fn update(&mut self, _: ()) {}
    fn view(&self, _: &Theme) -> View<()> {
        match self.0 {
            0 => qrcode(QrCode::new(b"a\0b".to_vec(), 58)),
            1 => qrcode(QrCode::new(b"changed".to_vec(), 58)),
            2 => barcode(Barcode::new("A B", 20)),
            3 => barcode(Barcode::new("AB", 20)),
            4 => gif(Gif::new(&include_bytes!("assets/two-colors.gif")[..])),
            5 => {
                let mut animation = Gif::new(&include_bytes!("assets/two-colors.gif")[..]);
                animation.paused = true;
                gif(animation)
            }
            6 => lottie(Lottie::new(
                include_str!("assets/moving-square.json"),
                32,
                32,
            )),
            7 => lottie(Lottie::new("{}", 32, 32)),
            8 => column([])
                .child(textarea("").keyboard_id(1).key(Key::Named("text")))
                .child(pinyin_keyboard(1, Font::cjk_16()).key(Key::Named("ime"))),
            _ => column([]).child(pinyin_keyboard(1, Font::cjk_16()).key(Key::Named("ime"))),
        }
    }
}
fn advance(ms: u32) {
    unsafe {
        rs_lvgl_sys::lv_tick_inc(ms);
        rs_lvgl_sys::lv_anim_refr_now();
        rs_lvgl_sys::lv_timer_handler();
    }
}
#[test]
fn qr_binary_payload_and_barcode_spaces_produce_real_updated_pixels() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    ui.mount(&TestApp(0)).unwrap();
    let qr = common::app_root();
    let first = ui.snapshot().unwrap();
    assert!(first.pixels.as_chunks::<4>().0.contains(&[0, 0, 0, 255]));
    assert!(first
        .pixels
        .as_chunks::<4>()
        .0
        .contains(&[255, 255, 255, 255]));
    ui.update(&TestApp(1)).unwrap();
    assert_eq!(common::app_root(), qr);
    assert_ne!(ui.snapshot().unwrap().pixels, first.pixels);
    ui.update(&TestApp(2)).unwrap();
    let with_space = ui.snapshot().unwrap();
    assert!(unsafe { rs_lvgl_sys::lv_barcode_is_render_valid(common::app_root().as_ptr()) });
    ui.update(&TestApp(3)).unwrap();
    let without_space = ui.snapshot().unwrap();
    assert_ne!(with_space, without_space, "Raw encoding preserves spaces");
}
#[test]
fn gif_owns_source_advances_frames_and_pauses_without_restarting() {
    let _guard = common::lock();
    for _ in 0..3 {
        let display = common::init();
        let mut ui = Ui::new(&display, common::theme()).unwrap();
        ui.mount(&TestApp(4)).unwrap();
        let gif = common::app_root();
        let first = ui.snapshot().unwrap();
        advance(80);
        let second = ui.snapshot().unwrap();
        assert_ne!(first.pixels, second.pixels);
        ui.update(&TestApp(5)).unwrap();
        assert_eq!(gif, common::app_root());
        let index = unsafe { rs_lvgl_sys::lv_gif_get_current_frame_index(gif.as_ptr()) };
        advance(300);
        assert_eq!(
            unsafe { rs_lvgl_sys::lv_gif_get_current_frame_index(gif.as_ptr()) },
            index
        );
        ui.unmount();
        advance(500);
    }
}
#[test]
fn lottie_renders_moves_rejects_invalid_source_and_reinitializes() {
    let _guard = common::lock();
    for _ in 0..3 {
        let display = common::init();
        let mut ui = Ui::new(&display, common::theme()).unwrap();
        ui.mount(&TestApp(6)).unwrap();
        let obj = common::app_root();
        unsafe {
            rs_lvgl_sys::lv_refr_now(core::ptr::null_mut());
            rs_lvgl_sys::lv_anim_refr_now();
        }
        let first = ui.snapshot().unwrap();
        assert_eq!((first.width, first.height), (32, 32));
        assert!(first
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[0] > 200 && p[3] > 200));
        advance(500);
        let second = ui.snapshot().unwrap();
        assert_ne!(first.pixels, second.pixels);
        ui.update(&TestApp(6)).unwrap();
        assert_eq!(common::app_root(), obj);
        assert_eq!(ui.update(&TestApp(7)), Err(UiError::InvalidProperty));
        ui.unmount();
        assert_eq!(unsafe { rs_lvgl_sys::lv_anim_count_running() }, 0);
    }
}
#[test]
fn pinyin_commits_candidate_and_unbinds_when_textarea_disappears() {
    use std::ffi::CStr;
    let _guard = common::lock();
    for _ in 0..3 {
        let display = common::init();
        let mut ui = Ui::new(&display, common::theme()).unwrap();
        ui.mount(&TestApp(8)).unwrap();
        let root = common::app_root();
        let textarea = common::child(root, 0);
        let ime_root = common::child(root, 1);
        let ime = common::child(ime_root, 0);
        unsafe {
            let kb = rs_lvgl_sys::lv_ime_pinyin_get_kb(ime.as_ptr());
            assert_eq!(rs_lvgl_sys::lv_keyboard_get_textarea(kb), textarea.as_ptr());
            for key in ["n", "i"] {
                let mut found = false;
                for id in 0..100 {
                    let text = rs_lvgl_sys::lv_buttonmatrix_get_button_text(kb, id);
                    if !text.is_null() && CStr::from_ptr(text).to_bytes() == key.as_bytes() {
                        rs_lvgl_sys::lv_buttonmatrix_set_selected_button(kb, id);
                        rs_lvgl_sys::lv_obj_send_event(
                            kb,
                            rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
                            core::ptr::null_mut(),
                        );
                        found = true;
                        break;
                    }
                }
                assert!(found);
            }
            let panel = rs_lvgl_sys::lv_ime_pinyin_get_cand_panel(ime.as_ptr());
            assert!(!rs_lvgl_sys::lv_obj_is_hidden(panel));
            let candidate = CStr::from_ptr(rs_lvgl_sys::lv_buttonmatrix_get_button_text(panel, 1))
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
            assert_eq!(
                CStr::from_ptr(rs_lvgl_sys::lv_textarea_get_text(textarea.as_ptr()))
                    .to_str()
                    .unwrap(),
                candidate
            );
        }
        ui.update(&TestApp(9)).unwrap();
        assert_eq!(common::child(common::app_root(), 0), ime_root);
        unsafe {
            let kb = rs_lvgl_sys::lv_ime_pinyin_get_kb(ime.as_ptr());
            assert!(rs_lvgl_sys::lv_keyboard_get_textarea(kb).is_null());
        }
        ui.unmount();
        advance(100);
    }
}

#[test]
fn vendored_cjk_fonts_resolve_chinese_glyphs_and_float_points_are_exact() {
    let _guard = common::lock();
    let display = common::init();
    unsafe {
        for font in [
            core::ptr::addr_of!(rs_lvgl_sys::lv_font_source_han_sans_sc_14_cjk),
            core::ptr::addr_of!(rs_lvgl_sys::lv_font_source_han_sans_sc_16_cjk),
        ] {
            let mut glyph = core::mem::zeroed();
            assert!(rs_lvgl_sys::lv_font_get_glyph_dsc(
                font,
                &mut glyph,
                '中' as u32,
                0
            ));
            assert_eq!(glyph.is_placeholder(), 0);
        }
    }
    #[cfg(feature = "widget-line")]
    {
        struct Line;
        impl App for Line {
            type Msg = ();
            fn update(&mut self, _: ()) {}
            fn view(&self, _: &Theme) -> View<()> {
                line([(16_777_217, 0), (0, 1)])
            }
        }
        let mut ui = Ui::new(&display, common::theme()).unwrap();
        assert_eq!(ui.mount(&Line), Err(UiError::InvalidProperty));
    }
}

#[test]
fn failed_later_sibling_releases_retained_native_users_before_retry() {
    struct Siblings(bool);
    impl App for Siblings {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            column([])
                .child(
                    gif(Gif::new(&include_bytes!("assets/two-colors.gif")[..]))
                        .key(Key::Named("gif")),
                )
                .child(
                    lottie(Lottie::new(
                        if self.0 {
                            "{}"
                        } else {
                            include_str!("assets/moving-square.json")
                        },
                        32,
                        32,
                    ))
                    .key(Key::Named("motion")),
                )
        }
    }
    let _guard = common::lock();
    for _ in 0..3 {
        let display = common::init();
        let mut ui = Ui::new(&display, common::theme()).unwrap();
        ui.mount(&Siblings(false)).unwrap();
        assert_eq!(ui.update(&Siblings(true)), Err(UiError::InvalidProperty));
        assert_eq!(ui.stats().mounted_nodes, 0);
        assert_eq!(unsafe { rs_lvgl_sys::lv_anim_count_running() }, 0);
        advance(100);
        ui.mount(&Siblings(false)).unwrap();
        assert_eq!(ui.stats().mounted_nodes, 3);
        ui.unmount();
    }
}
