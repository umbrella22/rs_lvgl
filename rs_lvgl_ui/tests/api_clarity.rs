//! Public semantic units, layout fields and widget part selectors reach native state.
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
fn scales_round_at_the_native_boundary_and_ratios_preserve_old_precision() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    ui.mount(&Page(|_| {
        container().children(
            [
                Scale::percent(1),
                Scale::percent(50),
                Scale::percent(75),
                Scale::percent(100),
                Scale::percent(150),
                Scale::ratio(1, 512).unwrap(),
                Scale::ratio(1, 513).unwrap(),
                Scale::ratio(65535, 256).unwrap(),
            ]
            .map(|scale| container().transform_scale_x(scale)),
        )
    }))
    .unwrap();
    // SAFETY: every pointer belongs to this live tree on the UI thread.
    unsafe {
        for (index, expected) in [3, 128, 192, 256, 384, 1, 0, 65535].into_iter().enumerate() {
            assert_eq!(
                sys::lv_obj_get_style_transform_scale_x(
                    common::child(common::app_root(), index as i32).as_ptr(),
                    0
                ),
                expected
            );
        }
    }
    assert_eq!(Scale::ratio(1, 0), Err(UiError::InvalidProperty));
    assert_eq!(Scale::ratio(65536, 256), Err(UiError::InvalidProperty));
    assert_eq!(Scale::ratio(u32::MAX, 1), Err(UiError::InvalidProperty));
    assert_eq!(Scale::ratio(u32::MAX, u32::MAX), Ok(Scale::percent(100)));
}

#[test]
fn angle_precision_and_invalid_unit_updates_preserve_the_tree() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Page(|_| container().transform_rotation(Angle::degrees(-90)));
    ui.mount(&app).unwrap();
    let root = common::app_root();
    // SAFETY: the root remains live across all valid/rejected updates below.
    assert_eq!(
        unsafe { sys::lv_obj_get_style_transform_rotation(root.as_ptr(), 0) },
        -900
    );
    for (view, expected) in [
        (
            (|_: &Theme| container().transform_rotation(Angle::tenths(455)))
                as fn(&Theme) -> View<()>,
            455,
        ),
        (
            |_| container().transform_rotation(Angle::tenths(-32768)),
            -32768,
        ),
        (
            |_| container().transform_rotation(Angle::tenths(32767)),
            32767,
        ),
    ] {
        app.0 = view;
        ui.update(&app).unwrap();
        assert_eq!(
            unsafe { sys::lv_obj_get_style_transform_rotation(root.as_ptr(), 0) },
            expected
        );
    }
    for invalid in [
        (|_: &Theme| container().transform_rotation(Angle::tenths(32768)))
            as fn(&Theme) -> View<()>,
        |_| container().transform_rotation(Angle::tenths(-32769)),
        |_| container().transform_rotation(Angle::degrees(i16::MAX)),
        |_| container().transform_rotation(Angle::degrees(i16::MIN)),
        |_| container().transform_scale_x(Scale::percent(25600)),
        |_| {
            container().state_style(
                State::PRESSED,
                style().transform_scale_y(Scale::percent(u16::MAX)),
            )
        },
    ] {
        app.0 = invalid;
        assert_eq!(ui.update(&app), Err(UiError::InvalidProperty));
        assert_eq!(common::app_root(), root);
        assert_eq!(
            unsafe { sys::lv_obj_get_style_transform_rotation(root.as_ptr(), 0) },
            32767
        );
    }
    app.0 = |_| container();
    ui.update(&app).unwrap();
    assert_eq!(
        unsafe { sys::lv_obj_get_style_transform_rotation(root.as_ptr(), 0) },
        0
    );
    assert_eq!(
        unsafe { sys::lv_obj_get_style_transform_scale_x(root.as_ptr(), 0) },
        256
    );
}

#[cfg(feature = "widget-image")]
#[test]
fn image_scale_uses_the_same_units_and_rejects_zero_before_mutation() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    static PIXELS: [u8; 2] = [0, 0];
    let mut app = Page(|_| {
        typed::image_scaled(ImageSource::rgb565(1, 1, &PIXELS), Scale::percent(50)).into_view()
    });
    ui.mount(&app).unwrap();
    let root = common::app_root();
    assert_eq!(unsafe { sys::lv_image_get_scale(root.as_ptr()) }, 128);
    for invalid in [
        (|_: &Theme| image_scaled(ImageSource::rgb565(1, 1, &PIXELS), Scale::percent(0)))
            as fn(&Theme) -> View<()>,
        |_| image_scaled(ImageSource::rgb565(1, 1, &PIXELS), Scale::percent(25600)),
    ] {
        app.0 = invalid;
        assert_eq!(ui.update(&app), Err(UiError::InvalidProperty));
        assert_eq!(common::app_root(), root);
        assert_eq!(unsafe { sys::lv_image_get_scale(root.as_ptr()) }, 128);
    }
}

#[test]
fn typed_animation_endpoints_reach_native_values_and_invalid_updates_do_not_restart() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Page(|_| {
        container()
            .animate(Animation::rotation(
                Angle::degrees(-45),
                Angle::tenths(455),
                100,
            ))
            .animate(Animation::scale_x(
                Scale::percent(50),
                Scale::percent(150),
                100,
            ))
            .animate(Animation::scale_y(
                Scale::percent(100),
                Scale::percent(75),
                100,
            ))
    });
    ui.mount(&app).unwrap();
    let root = common::app_root();
    assert_eq!(
        unsafe { sys::lv_obj_get_style_transform_rotation(root.as_ptr(), 0) },
        -450
    );
    for invalid in [
        (|_: &Theme| {
            container().animate(Animation::rotation(
                Angle::degrees(0),
                Angle::tenths(i32::MAX),
                100,
            ))
        }) as fn(&Theme) -> View<()>,
        |_| {
            container().animate(Animation::scale_x(
                Scale::percent(100),
                Scale::percent(25600),
                100,
            ))
        },
    ] {
        app.0 = invalid;
        assert_eq!(ui.update(&app), Err(UiError::InvalidProperty));
        assert_eq!(common::app_root(), root);
    }
    rs_lvgl::tick_inc(core::time::Duration::from_millis(120));
    unsafe {
        sys::lv_anim_refr_now();
        assert_eq!(
            sys::lv_obj_get_style_transform_rotation(root.as_ptr(), 0),
            455
        );
        assert_eq!(
            sys::lv_obj_get_style_transform_scale_x(root.as_ptr(), 0),
            384
        );
        assert_eq!(
            sys::lv_obj_get_style_transform_scale_y(root.as_ptr(), 0),
            192
        );
    }
}

#[test]
fn label_overflow_builder_applies_and_replaces_native_mode() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Page(|_| {
        typed::label("long label")
            .text_overflow(TextOverflow::Ellipsis)
            .into_view()
    });
    ui.mount(&app).unwrap();
    let root = common::app_root();
    assert_eq!(
        unsafe { sys::lv_label_get_long_mode(root.as_ptr()) },
        sys::lv_label_long_mode_t_LV_LABEL_LONG_MODE_DOTS
    );
    app.0 = |_| label("long label").text_overflow(TextOverflow::ScrollLoop);
    ui.update(&app).unwrap();
    assert_eq!(
        unsafe { sys::lv_label_get_long_mode(root.as_ptr()) },
        sys::lv_label_long_mode_t_LV_LABEL_LONG_MODE_SCROLL_CIRCULAR
    );
}

#[test]
fn flex_properties_merge_independently_and_place_children_in_native_coordinates() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    fn items() -> View<()> {
        row([
            container().width(px(10)).height(px(10)),
            container().width(px(10)).height(px(20)),
        ])
        .width(px(60))
        .height(px(60))
    }
    let mut app = Page(|_| {
        items()
            .justify_content(FlexAlign::End)
            .align_items(ItemAlign::Start)
            .style(style().align_content(FlexAlign::Center))
            .style(style().align_items(ItemAlign::End))
    });
    ui.mount(&app).unwrap();
    let root = common::app_root();
    let first = common::child(root, 0);
    let second = common::child(root, 1);
    // SAFETY: all objects remain mounted; layout and getters are synchronous.
    unsafe {
        sys::lv_obj_update_layout(root.as_ptr());
        assert_eq!(
            sys::lv_obj_get_style_flex_main_place(root.as_ptr(), 0),
            sys::lv_flex_align_t_LV_FLEX_ALIGN_END
        );
        assert_eq!(
            sys::lv_obj_get_style_flex_cross_place(root.as_ptr(), 0),
            sys::lv_flex_align_t_LV_FLEX_ALIGN_END
        );
        assert_eq!(
            sys::lv_obj_get_style_flex_track_place(root.as_ptr(), 0),
            sys::lv_flex_align_t_LV_FLEX_ALIGN_CENTER
        );
        assert_eq!(
            (
                sys::lv_obj_get_x(first.as_ptr()),
                sys::lv_obj_get_y(first.as_ptr())
            ),
            (40, 30)
        );
        assert_eq!(
            (
                sys::lv_obj_get_x(second.as_ptr()),
                sys::lv_obj_get_y(second.as_ptr())
            ),
            (50, 20)
        );
    }
    app.0 = |_| {
        items()
            .justify_content(FlexAlign::End)
            .align_content(FlexAlign::Center)
    };
    ui.update(&app).unwrap();
    unsafe {
        sys::lv_obj_update_layout(root.as_ptr());
        assert_eq!(
            sys::lv_obj_get_style_flex_cross_place(root.as_ptr(), 0),
            sys::lv_flex_align_t_LV_FLEX_ALIGN_START
        );
        assert_eq!(
            (
                sys::lv_obj_get_y(first.as_ptr()),
                sys::lv_obj_get_y(second.as_ptr())
            ),
            (20, 20)
        );
        assert_eq!(sys::lv_obj_get_x(first.as_ptr()), 40);
    }
    app.0 = |_| items();
    ui.update(&app).unwrap();
    unsafe {
        sys::lv_obj_update_layout(root.as_ptr());
        assert_eq!(
            (
                sys::lv_obj_get_x(first.as_ptr()),
                sys::lv_obj_get_y(first.as_ptr())
            ),
            (0, 0)
        );
        assert_eq!(
            sys::lv_obj_get_style_flex_track_place(root.as_ptr(), 0),
            sys::lv_flex_align_t_LV_FLEX_ALIGN_START
        );
    }
    assert_eq!(
        style()
            .justify_content(FlexAlign::End)
            .justify_content_value(),
        FlexAlign::End
    );
    assert_eq!(
        style().align_items(ItemAlign::Center).align_items_value(),
        ItemAlign::Center
    );
    assert_eq!(
        style()
            .align_content(FlexAlign::SpaceAround)
            .align_content_value(),
        FlexAlign::SpaceAround
    );
}

#[test]
fn floating_excludes_layout_and_parent_scroll_and_clears_on_update() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    fn content(floating: bool) -> View<()> {
        column([
            container().width(px(10)).height(px(20)),
            container()
                .key(Key::Named("badge"))
                .width(px(10))
                .height(px(10))
                .floating(floating)
                .align(Align::TopRight)
                .offset(-2, 4),
            container().width(px(10)).height(px(100)),
        ])
        .width(px(60))
        .height(px(60))
        .scrollable(true)
    }
    let mut app = Page(|_| content(true));
    ui.mount(&app).unwrap();
    let root = common::app_root();
    let badge = common::child(root, 1);
    let last = common::child(root, 2);
    unsafe {
        sys::lv_obj_update_layout(root.as_ptr());
        assert!(sys::lv_obj_has_flag(
            badge.as_ptr(),
            sys::lv_obj_flag_t_LV_OBJ_FLAG_FLOATING
        ));
        assert!(!sys::lv_obj_is_layout_positioned(badge.as_ptr()));
        assert_eq!(sys::lv_obj_get_y(last.as_ptr()), 20);
        let mut badge_before = core::mem::zeroed();
        let mut last_before = core::mem::zeroed();
        sys::lv_obj_get_coords(badge.as_ptr(), &mut badge_before);
        sys::lv_obj_get_coords(last.as_ptr(), &mut last_before);
        sys::lv_obj_scroll_to_y(root.as_ptr(), 20, false);
        let mut badge_after = core::mem::zeroed();
        let mut last_after = core::mem::zeroed();
        sys::lv_obj_get_coords(badge.as_ptr(), &mut badge_after);
        sys::lv_obj_get_coords(last.as_ptr(), &mut last_after);
        assert_eq!(sys::lv_obj_get_scroll_y(root.as_ptr()), 20);
        assert_eq!(
            (badge_before.x1, badge_before.y1),
            (badge_after.x1, badge_after.y1)
        );
        assert_eq!(last_after.y1, last_before.y1 - 20);
        sys::lv_obj_scroll_to_y(root.as_ptr(), 0, false);
    }
    app.0 = |_| content(false);
    ui.update(&app).unwrap();
    assert_eq!(common::child(root, 1), badge);
    unsafe {
        sys::lv_obj_update_layout(root.as_ptr());
        assert!(!sys::lv_obj_has_flag(
            badge.as_ptr(),
            sys::lv_obj_flag_t_LV_OBJ_FLAG_FLOATING
        ));
        assert!(sys::lv_obj_is_layout_positioned(badge.as_ptr()));
        assert_eq!(sys::lv_obj_get_y(last.as_ptr()), 30);
    }
}

#[cfg(feature = "widget-slider")]
#[test]
fn semantic_and_native_part_styles_share_selector_replacement_states_and_restoration() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Page(|_| slider(0, 100, 50));
    ui.mount(&app).unwrap();
    let root = common::app_root();
    let knob = sys::lv_part_t_LV_PART_KNOB;
    let progress = sys::lv_part_t_LV_PART_INDICATOR;
    let baseline_radius = unsafe { sys::lv_obj_get_style_radius(root.as_ptr(), knob) };
    let baseline_color = unsafe { sys::lv_obj_get_style_bg_color(root.as_ptr(), knob) };
    app.0 = |_| {
        typed::slider(0, 100, 50)
            .thumb_style(style().background(Color::rgb(10, 20, 30)).radius(3))
            .native_part_style(NativePart::Knob, style().background(Color::rgb(40, 50, 60)))
            .thumb_style(style().radius(8))
            .thumb_state_style(State::PRESSED, style().outline_width(13))
            .progress_style(style().outline_width(7))
            .progress_state_style(State::PRESSED, style().outline_width(11))
            .into_view()
    };
    ui.update(&app).unwrap();
    unsafe {
        let color = sys::lv_obj_get_style_bg_color(root.as_ptr(), knob);
        // Existing selector semantics: a later patch replaces the whole earlier
        // patch for that selector. The final radius-only patch restores its color.
        assert_eq!(
            (color.red, color.green, color.blue),
            (
                baseline_color.red,
                baseline_color.green,
                baseline_color.blue
            )
        );
        assert_eq!(sys::lv_obj_get_style_radius(root.as_ptr(), knob), 8);
        assert_eq!(
            sys::lv_obj_get_style_outline_width(root.as_ptr(), progress),
            7
        );
        sys::lv_obj_add_state(root.as_ptr(), sys::lv_state_t_LV_STATE_PRESSED);
        assert_eq!(sys::lv_obj_get_style_outline_width(root.as_ptr(), knob), 13);
        assert_eq!(
            sys::lv_obj_get_style_outline_width(root.as_ptr(), progress),
            11
        );
    }
    app.0 = |_| slider(0, 100, 50).thumb_style(style().radius(5));
    ui.update(&app).unwrap();
    assert_eq!(common::app_root(), root);
    unsafe {
        assert_eq!(sys::lv_obj_get_style_radius(root.as_ptr(), knob), 5);
        assert_eq!(sys::lv_obj_get_style_outline_width(root.as_ptr(), knob), 0);
        assert_eq!(
            sys::lv_obj_get_style_outline_width(root.as_ptr(), progress),
            0
        );
        let color = sys::lv_obj_get_style_bg_color(root.as_ptr(), knob);
        assert_eq!(
            (color.red, color.green, color.blue),
            (
                baseline_color.red,
                baseline_color.green,
                baseline_color.blue
            )
        );
    }
    app.0 = |_| slider(0, 100, 50);
    ui.update(&app).unwrap();
    assert_eq!(
        unsafe { sys::lv_obj_get_style_radius(root.as_ptr(), knob) },
        baseline_radius
    );
}

#[cfg(all(feature = "widget-switch", feature = "widget-bar"))]
#[test]
fn switch_thumb_and_progress_bar_use_their_actual_native_parts() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    ui.mount(&Page(|_| {
        container()
            .child(
                typed::switch(false)
                    .thumb_style(style().outline_width(9))
                    .thumb_state_style(State::CHECKED, style().outline_width(15)),
            )
            .child(
                typed::progress_bar(0, 100, 50)
                    .progress_style(style().outline_width(6))
                    .progress_state_style(State::PRESSED, style().outline_width(12)),
            )
    }))
    .unwrap();
    unsafe {
        let switch = common::child(common::app_root(), 0).as_ptr();
        let bar = common::child(common::app_root(), 1).as_ptr();
        assert_eq!(
            sys::lv_obj_get_style_outline_width(switch, sys::lv_part_t_LV_PART_KNOB),
            9
        );
        assert_eq!(
            sys::lv_obj_get_style_outline_width(bar, sys::lv_part_t_LV_PART_INDICATOR),
            6
        );
        sys::lv_obj_add_state(switch, sys::lv_state_t_LV_STATE_CHECKED);
        sys::lv_obj_add_state(bar, sys::lv_state_t_LV_STATE_PRESSED);
        assert_eq!(
            sys::lv_obj_get_style_outline_width(switch, sys::lv_part_t_LV_PART_KNOB),
            15
        );
        assert_eq!(
            sys::lv_obj_get_style_outline_width(bar, sys::lv_part_t_LV_PART_INDICATOR),
            12
        );
    }
}

#[test]
fn erased_semantic_misuse_follows_diagnostics_and_strict_policy() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    #[cfg(feature = "diagnostics")]
    let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    #[cfg(feature = "diagnostics")]
    {
        let captured = events.clone();
        ui.set_diagnostic_sink(Some(std::rc::Rc::new(move |event| {
            captured.borrow_mut().push(event)
        })));
    }
    let mut app = Page(|_| button("valid"));
    ui.mount(&app).unwrap();
    let root = common::app_root();
    for (view, property) in [
        (
            (|_: &Theme| button("valid").thumb_style(style().outline_width(47)))
                as fn(&Theme) -> View<()>,
            rs_lvgl_ui::Property::ThumbStyle,
        ),
        (
            |_| button("valid").progress_state_style(State::PRESSED, style().outline_width(47)),
            rs_lvgl_ui::Property::ProgressStyle,
        ),
        (
            |_| button("valid").text_overflow(TextOverflow::Clip),
            rs_lvgl_ui::Property::TextOverflow,
        ),
    ] {
        app.0 = view;
        #[cfg(feature = "diagnostics")]
        events.borrow_mut().clear();
        #[cfg(feature = "strict-properties")]
        assert_eq!(ui.update(&app), Err(UiError::InvalidProperty));
        #[cfg(not(feature = "strict-properties"))]
        ui.update(&app).unwrap();
        assert_eq!(common::app_root(), root);
        assert_ne!(
            unsafe {
                sys::lv_obj_get_style_outline_width(root.as_ptr(), sys::lv_part_t_LV_PART_KNOB)
            },
            47
        );
        #[cfg(all(feature = "diagnostics", not(feature = "strict-properties")))]
        assert!(events.borrow().iter().any(|event| matches!(event, rs_lvgl_ui::DiagnosticEvent::IgnoredProperty(p) if *p == property)));
        #[cfg(all(feature = "diagnostics", feature = "strict-properties"))]
        assert!(events.borrow().iter().any(|event| matches!(event, rs_lvgl_ui::DiagnosticEvent::Error { property: p, .. } if *p == property)));
        #[cfg(not(feature = "diagnostics"))]
        let _ = property;
    }
}
