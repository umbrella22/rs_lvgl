mod common;
use rs_lvgl_ui::{
    Animation, AnimationProperty, App, Node, State, Style, StyleTransition, Theme,
    TransitionProperty, Ui, UiError,
};

fn advance(ms: u32) {
    rs_lvgl::tick_inc(core::time::Duration::from_millis(ms.into()));
    unsafe {
        rs_lvgl_sys::lv_anim_refr_now();
    }
}
struct Motion {
    animation: Option<Animation>,
    transition: bool,
}
impl App for Motion {
    type Msg = ();
    fn update(&mut self, _: ()) {}
    fn view(&self, _: &Theme) -> Node<()> {
        let mut style = Style::default()
            .outline_width(0)
            .width(rs_lvgl_ui::Size::Px(32))
            .height(rs_lvgl_ui::Size::Px(32))
            .bg(rs_lvgl_ui::Color::rgb(80, 90, 100));
        if self.transition {
            style = style.transition(StyleTransition::new(
                &[TransitionProperty::OutlineWidth],
                100,
            ));
        }
        let mut node = Node::obj()
            .style(style)
            .state_style(State::PRESSED, Style::default().outline_width(10));
        if let Some(animation) = self.animation {
            node = node.animate(animation);
        }
        node
    }
}
#[test]
fn explicit_motion_advances_without_restarting_on_equal_updates() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Motion {
        animation: Some(Animation::new(AnimationProperty::Opacity, 0, 200, 100)),
        transition: false,
    };
    ui.mount(&app).unwrap();
    let obj = common::app_root().as_ptr();
    advance(50);
    let midway = unsafe { rs_lvgl_sys::lv_obj_get_style_opa(obj, 0) };
    assert!(midway > 0 && midway < 200, "intermediate opacity {midway}");
    ui.update(&app).unwrap();
    assert_eq!(unsafe { rs_lvgl_sys::lv_obj_get_style_opa(obj, 0) }, midway);
    advance(60);
    assert_eq!(unsafe { rs_lvgl_sys::lv_obj_get_style_opa(obj, 0) }, 200);
    ui.update(&app).unwrap();
    assert_eq!(unsafe { rs_lvgl_sys::lv_obj_get_style_opa(obj, 0) }, 200);
    app.animation = Some(Animation::new(AnimationProperty::Opacity, 200, 0, 100).delay(40));
    ui.update(&app).unwrap();
    advance(20);
    assert_eq!(unsafe { rs_lvgl_sys::lv_obj_get_style_opa(obj, 0) }, 200);
    advance(60);
    let midway = unsafe { rs_lvgl_sys::lv_obj_get_style_opa(obj, 0) };
    assert!(midway > 0 && midway < 200);
    app.animation = None;
    ui.update(&app).unwrap();
    advance(200);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_obj_get_style_opa(obj, 0) },
        midway,
        "removal cancels the timer"
    );
    assert_eq!(unsafe { rs_lvgl_sys::lv_anim_count_running() }, 0);
}
#[test]
fn native_state_transition_survives_descriptors_and_cancels_on_removal() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Motion {
        animation: None,
        transition: true,
    };
    ui.mount(&app).unwrap();
    let obj = common::app_root().as_ptr();
    unsafe {
        rs_lvgl_sys::lv_refr_now(core::ptr::null_mut());
        rs_lvgl_sys::lv_obj_add_state(obj, rs_lvgl_sys::lv_state_t_LV_STATE_PRESSED);
    }
    advance(50);
    let midway = unsafe { rs_lvgl_sys::lv_obj_get_style_outline_width(obj, 0) };
    assert!(midway > 0 && midway < 10, "intermediate outline {midway}");
    ui.update(&app).unwrap();
    advance(60);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_obj_get_style_outline_width(obj, 0) },
        10
    );
    unsafe {
        rs_lvgl_sys::lv_obj_remove_state(obj, rs_lvgl_sys::lv_state_t_LV_STATE_PRESSED);
    }
    advance(20);
    assert!(unsafe { rs_lvgl_sys::lv_anim_count_running() } > 0);
    app.transition = false;
    ui.update(&app).unwrap();
    assert_eq!(unsafe { rs_lvgl_sys::lv_anim_count_running() }, 0);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_obj_get_style_outline_width(obj, 0) },
        0
    );
    advance(200);
    unsafe {
        assert!(rs_lvgl_sys::lv_obj_get_style_transition(obj, 0).is_null());
    }
}
#[test]
fn unmount_and_reinitialize_cancel_infinite_and_delayed_animations() {
    let _guard = common::lock();
    for _ in 0..3 {
        let display = common::init();
        let mut ui = Ui::new(&display, common::theme()).unwrap();
        let app = Motion {
            animation: Some(
                Animation::new(AnimationProperty::Rotation, 0, 900, 40)
                    .repeat(u32::MAX)
                    .reverse(40),
            ),
            transition: true,
        };
        ui.mount(&app).unwrap();
        unsafe {
            rs_lvgl_sys::lv_refr_now(core::ptr::null_mut());
            rs_lvgl_sys::lv_obj_add_state(
                common::app_root().as_ptr(),
                rs_lvgl_sys::lv_state_t_LV_STATE_PRESSED,
            );
        }
        advance(20);
        assert!(unsafe { rs_lvgl_sys::lv_anim_count_running() } >= 2);
        ui.unmount();
        assert_eq!(unsafe { rs_lvgl_sys::lv_anim_count_running() }, 0);
        advance(500);
        drop(ui);
    }
}
#[test]
fn invalid_animation_time_and_value_fail_before_mutation() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    for animation in [
        Animation::new(AnimationProperty::Opacity, 0, 256, 100),
        Animation::new(AnimationProperty::X, 0, 2, u32::MAX),
    ] {
        assert_eq!(
            ui.mount(&Motion {
                animation: Some(animation),
                transition: false
            }),
            Err(UiError::InvalidProperty)
        );
        assert_eq!(ui.stats().mounted_nodes, 0);
    }
}
