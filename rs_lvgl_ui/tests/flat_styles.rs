//! Flat construction must preserve native precedence, inheritance and updates.
mod common;

use rs_lvgl_sys as sys;
use rs_lvgl_ui::prelude::*;

struct Page(fn(&Theme) -> View<()>);
impl App for Page {
    type Action = ();
    fn update(&mut self, _: ()) {}
    fn view(&self, theme: &Theme) -> View<()> {
        (self.0)(theme)
    }
}

#[test]
fn flat_modifiers_and_reusable_patches_keep_explicit_zero_and_call_order() {
    let _lock = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let app = Page(|_| {
        let patch = style().padding(9).gap(12).grow(4).shadow_width(6);
        obj().children([
            column([])
                .padding(9)
                .gap(12)
                .grow(4)
                .shadow_width(6)
                .style(style().padding_left(0).grow(0).shadow_width(0))
                .padding_top(3),
            column([])
                .style(patch)
                .padding_left(0)
                .grow(0)
                .shadow_width(0)
                .padding_top(3),
            obj().padding(9).style(style().padding(0)).padding_left(2),
            obj()
                .background(Color::rgb(12, 34, 56))
                .style(style().background_opacity(80)),
            obj()
                .background_opacity(80)
                .background(Color::rgb(12, 34, 56)),
        ])
    });
    ui.mount(&app).unwrap();
    // SAFETY: each child belongs to the mounted UI and is used on this thread.
    unsafe {
        for index in 0..2 {
            let child = common::child(common::app_root(), index).as_ptr();
            assert_eq!(sys::lv_obj_get_style_pad_left(child, 0), 0);
            assert_eq!(sys::lv_obj_get_style_pad_right(child, 0), 9);
            assert_eq!(sys::lv_obj_get_style_pad_top(child, 0), 3);
            assert_eq!(sys::lv_obj_get_style_pad_bottom(child, 0), 9);
            assert_eq!(sys::lv_obj_get_style_pad_row(child, 0), 12);
            assert_eq!(sys::lv_obj_get_style_pad_column(child, 0), 0);
            assert_eq!(sys::lv_obj_get_style_flex_grow(child, 0), 0);
            assert_eq!(sys::lv_obj_get_style_shadow_width(child, 0), 0);
        }
        let zero = common::child(common::app_root(), 2).as_ptr();
        assert_eq!(sys::lv_obj_get_style_pad_left(zero, 0), 2);
        assert_eq!(sys::lv_obj_get_style_pad_right(zero, 0), 0);
        assert_eq!(sys::lv_obj_get_style_pad_top(zero, 0), 0);
        assert_eq!(sys::lv_obj_get_style_pad_bottom(zero, 0), 0);
        for (index, opacity) in [(3, 80), (4, 255)] {
            let child = common::child(common::app_root(), index).as_ptr();
            let color = sys::lv_obj_get_style_bg_color(child, 0);
            assert_eq!((color.red, color.green, color.blue), (12, 34, 56));
            assert_eq!(sys::lv_obj_get_style_bg_opa(child, 0), opacity);
        }
    }
}

#[test]
fn explicit_units_reach_native_layout_and_invalid_updates_preserve_it() {
    let _lock = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Page(|_| {
        column([
            obj().width(percent(50)).height(px(10)),
            label("x")
                .width(content())
                .height(content())
                .min_width(px(8))
                .max_width(px(40))
                .min_height(px(6))
                .max_height(px(32)),
        ])
        .width(px(60))
        .height(px(60))
    });
    ui.mount(&app).unwrap();
    let root = common::app_root();
    let half = common::child(root, 0);
    let text = common::child(root, 1);
    // SAFETY: all pointers belong to the live mounted tree. lv_pct is pure.
    unsafe {
        sys::lv_obj_update_layout(root.as_ptr());
        assert_eq!(sys::lv_obj_get_width(root.as_ptr()), 60);
        assert_eq!(sys::lv_obj_get_width(half.as_ptr()), 30);
        assert_eq!(sys::lv_obj_get_height(half.as_ptr()), 10);
        assert_eq!(
            sys::lv_obj_get_style_width(half.as_ptr(), 0),
            sys::lv_pct(50)
        );
        // LV_SIZE_CONTENT is a native coordinate macro, not a pixel count.
        assert_eq!(sys::lv_obj_get_style_width(text.as_ptr(), 0), 0x3fff_ffff);
        assert_eq!(sys::lv_obj_get_style_height(text.as_ptr(), 0), 0x3fff_ffff);
        assert_eq!(sys::lv_obj_get_style_min_width(text.as_ptr(), 0), 8);
        assert_eq!(sys::lv_obj_get_style_max_width(text.as_ptr(), 0), 40);
        assert_eq!(sys::lv_obj_get_style_min_height(text.as_ptr(), 0), 6);
        assert_eq!(sys::lv_obj_get_style_max_height(text.as_ptr(), 0), 32);
    }
    app.0 = |_| column([]).width(px(-1));
    assert_eq!(ui.update(&app), Err(UiError::InvalidProperty));
    assert_eq!(common::app_root(), root);
    assert_eq!(common::child(root, 0), half);
    // SAFETY: rejected validation left the original tree alive and untouched.
    assert_eq!(unsafe { sys::lv_obj_get_width(root.as_ptr()) }, 60);
}

#[test]
fn unset_text_inherits_and_theme_changes_rebuild_explicit_inputs() {
    let _lock = common::lock();
    let display = common::init();
    let mut theme = common::theme();
    let mut ui = Ui::new(&display, theme).unwrap();
    let mut app = Page(|theme| {
        column([
            label("Inherited").style(style()),
            label("Explicit").text_color(Color::rgb(1, 2, 3)),
        ])
        .text_color(theme.text)
        .font(theme.body_font)
    });
    ui.mount(&app).unwrap();
    let root = common::app_root();
    let inherited = common::child(root, 0);
    let explicit = common::child(root, 1);
    for _ in 0..2 {
        // SAFETY: mounted pointers remain valid across scalar style updates.
        unsafe {
            let color = sys::lv_obj_get_style_text_color(inherited.as_ptr(), 0);
            assert_eq!(
                (color.red, color.green, color.blue),
                (theme.text.r, theme.text.g, theme.text.b)
            );
            assert_eq!(
                sys::lv_obj_get_style_text_font(inherited.as_ptr(), 0),
                core::ptr::addr_of!(sys::lv_font_montserrat_14)
            );
            let color = sys::lv_obj_get_style_text_color(explicit.as_ptr(), 0);
            assert_eq!((color.red, color.green, color.blue), (1, 2, 3));
        }
        theme = Theme::light();
        ui.set_theme(theme);
        assert!(ui.drain(&mut app, 8).unwrap().reconciled);
        assert_eq!(common::app_root(), root);
        assert_eq!(common::child(root, 0), inherited);
    }
    assert!(!ui.drain(&mut app, 8).unwrap().reconciled);
}

#[test]
fn removing_state_and_part_patches_restores_flat_base_on_the_same_object() {
    let _lock = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Page(|_| {
        obj()
            .outline_width(2)
            .state_style(State::PRESSED, style().outline_width(9).shadow_width(5))
            .part_style(Part::Scrollbar, style().outline_width(7))
    });
    ui.mount(&app).unwrap();
    let root = common::app_root();
    let raw = root.as_ptr();
    // SAFETY: this test holds the native lock and the object is mounted.
    unsafe {
        assert_eq!(sys::lv_obj_get_style_outline_width(raw, 0), 2);
        assert_eq!(
            sys::lv_obj_get_style_outline_width(raw, sys::lv_part_t_LV_PART_SCROLLBAR),
            7
        );
        sys::lv_obj_add_state(raw, sys::lv_state_t_LV_STATE_PRESSED);
        assert_eq!(sys::lv_obj_get_style_outline_width(raw, 0), 9);
        assert_eq!(sys::lv_obj_get_style_shadow_width(raw, 0), 5);
    }
    app.0 = |_| obj().outline_width(2);
    ui.update(&app).unwrap();
    assert_eq!(common::app_root(), root);
    // SAFETY: the update preserves object identity while removing overrides.
    unsafe {
        assert_eq!(sys::lv_obj_get_style_outline_width(raw, 0), 2);
        assert_eq!(sys::lv_obj_get_style_shadow_width(raw, 0), 0);
        assert_eq!(
            sys::lv_obj_get_style_outline_width(raw, sys::lv_part_t_LV_PART_SCROLLBAR),
            0
        );
    }
}
