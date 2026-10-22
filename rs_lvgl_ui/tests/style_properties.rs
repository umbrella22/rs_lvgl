mod common;
use rs_lvgl_ui::{
    App, BlendMode, Color, GradientDirection, Node, Part, State, Style, TextDecoration, Theme, Ui,
    UiError,
};

struct Styled {
    style: Style,
    state: Option<Style>,
}
impl App for Styled {
    type Msg = ();
    fn update(&mut self, _: ()) {}
    fn view(&self, _: &Theme) -> Node<()> {
        let mut node = Node::obj()
            .style(self.style)
            .child(Node::label("Inherited"));
        if let Some(style) = self.state {
            node = node
                .state_style(State::PRESSED, style)
                .part_style(Part::Scrollbar, style);
        }
        node
    }
}

#[test]
fn properties_merge_inherit_and_reset_on_the_same_native_object() {
    let _lock = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Styled {
        style: Style::default(),
        state: None,
    };
    ui.mount(&app).unwrap();
    let obj = common::app_root().as_ptr();
    unsafe {
        let baseline_shadow = rs_lvgl_sys::lv_obj_get_style_shadow_width(obj, 0);
        let baseline_scale = rs_lvgl_sys::lv_obj_get_style_transform_scale_x(obj, 0);
        let baseline_recolor = rs_lvgl_sys::lv_obj_get_style_image_recolor_opa(obj, 0);
        app.style = Style::default()
            .shadow_width(12)
            .shadow_color(Color::rgb(12, 34, 56))
            .shadow_offset_x(-7)
            .shadow_offset_y(9)
            .shadow_spread(-2)
            .shadow_opacity(137)
            .outline_width(3)
            .outline_pad(-1)
            .outline_opacity(91)
            .outline_color(Color::rgb(20, 30, 40))
            .bg_grad_color(Color::rgb(70, 80, 90))
            .bg_grad_dir(GradientDirection::Vertical)
            .bg_main_stop(17)
            .bg_grad_stop(220)
            .bg_main_opacity(200)
            .bg_grad_opacity(100)
            .text_letter_space(-1)
            .text_line_space(6)
            .text_decoration(TextDecoration::UNDERLINE | TextDecoration::STRIKETHROUGH)
            .image_recolor(Color::rgb(100, 110, 120))
            .image_recolor_opacity(180)
            .transform_scale_x(384)
            .transform_scale_y(192)
            .transform_rotation(-450)
            .transform_pivot_x(8)
            .transform_pivot_y(-4)
            .transform_width(2)
            .transform_height(3)
            .blend_mode(BlendMode::Multiply);
        ui.update(&app).unwrap();
        assert_eq!(common::app_root().as_ptr(), obj);
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_shadow_width(obj, 0), 12);
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_shadow_offset_x(obj, 0), -7);
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_shadow_spread(obj, 0), -2);
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_shadow_opa(obj, 0), 137);
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_outline_width(obj, 0), 3);
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_outline_pad(obj, 0), -1);
        assert_eq!(
            rs_lvgl_sys::lv_obj_get_style_bg_grad_dir(obj, 0),
            rs_lvgl_sys::lv_grad_dir_t_LV_GRAD_DIR_VER
        );
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_bg_main_stop(obj, 0), 17);
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_bg_grad_stop(obj, 0), 220);
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_bg_main_opa(obj, 0), 200);
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_bg_grad_opa(obj, 0), 100);
        let child = common::child(common::app_root(), 0).as_ptr();
        assert_eq!(
            rs_lvgl_sys::lv_obj_get_style_text_letter_space(child, 0),
            -1
        );
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_text_line_space(child, 0), 6);
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_text_decor(child, 0), 3);
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_image_recolor_opa(obj, 0), 180);
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_transform_scale_x(obj, 0), 384);
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_transform_scale_y(obj, 0), 192);
        assert_eq!(
            rs_lvgl_sys::lv_obj_get_style_transform_rotation(obj, 0),
            -450
        );
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_transform_pivot_y(obj, 0), -4);
        assert_eq!(
            rs_lvgl_sys::lv_obj_get_style_blend_mode(obj, 0),
            rs_lvgl_sys::lv_blend_mode_t_LV_BLEND_MODE_MULTIPLY
        );
        app.style = Style::default();
        ui.update(&app).unwrap();
        assert_eq!(
            rs_lvgl_sys::lv_obj_get_style_shadow_width(obj, 0),
            baseline_shadow
        );
        assert_eq!(
            rs_lvgl_sys::lv_obj_get_style_transform_scale_x(obj, 0),
            baseline_scale
        );
        assert_eq!(
            rs_lvgl_sys::lv_obj_get_style_image_recolor_opa(obj, 0),
            baseline_recolor
        );
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_text_letter_space(child, 0), 0);
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_text_decor(child, 0), 0);
    }
}

#[test]
fn state_and_part_removal_restore_their_base_styles() {
    let _lock = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Styled {
        style: Style::default().outline_width(2),
        state: Some(Style::default().outline_width(9).shadow_width(5)),
    };
    ui.mount(&app).unwrap();
    let obj = common::app_root().as_ptr();
    unsafe {
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_outline_width(obj, 0), 2);
        assert_eq!(
            rs_lvgl_sys::lv_obj_get_style_outline_width(
                obj,
                rs_lvgl_sys::lv_part_t_LV_PART_SCROLLBAR
            ),
            9
        );
        rs_lvgl_sys::lv_obj_add_state(obj, rs_lvgl_sys::lv_state_t_LV_STATE_PRESSED);
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_outline_width(obj, 0), 9);
        app.state = None;
        ui.update(&app).unwrap();
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_outline_width(obj, 0), 2);
        assert_eq!(rs_lvgl_sys::lv_obj_get_style_shadow_width(obj, 0), 0);
        assert_eq!(
            rs_lvgl_sys::lv_obj_get_style_outline_width(
                obj,
                rs_lvgl_sys::lv_part_t_LV_PART_SCROLLBAR
            ),
            0
        );
    }
}

#[test]
fn invalid_width_is_rejected_before_mutating_the_mounted_tree() {
    let _lock = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Styled {
        style: Style::default().shadow_width(7),
        state: None,
    };
    ui.mount(&app).unwrap();
    let obj = common::app_root();
    app.style = Style::default().shadow_width(-1);
    assert_eq!(ui.update(&app), Err(UiError::InvalidProperty));
    assert_eq!(common::app_root(), obj);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_obj_get_style_shadow_width(obj.as_ptr(), 0) },
        7
    );
}
