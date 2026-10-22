//! Tests use each example's actual App and native rendered objects.
use super::*;
#[path = "../../rs_lvgl_ui/examples/support/audit.rs"]
pub mod native;

unsafe extern "C" {
    static lv_button_class: rs_lvgl_sys::lv_obj_class_t;
}

pub fn click<A: App>(ui: &mut Ui<A::Msg>, app: &mut A, text: &str) {
    let mut pointer = native::Pointer::new();
    pointer.click(native::parent(native::label(native::screen(), text)));
    ui.drain(app, 64).unwrap();
}

fn layout(obj: native::Obj) {
    let [x1, y1, x2, y2] = native::rect(obj);
    assert!(x2 >= x1 && y2 >= y1, "example content has no visible area");
    assert!(
        x1 >= 0 && y1 >= 0 && x2 < WIDTH as i32 && y2 < HEIGHT as i32,
        "example content clipped: {x1},{y1}..{x2},{y2}"
    );
    // SAFETY: this traversal runs while the mounted UI owns every object.
    unsafe {
        if rs_lvgl_sys::lv_obj_check_type(obj, &raw const lv_button_class) {
            assert_eq!(
                rs_lvgl_sys::lv_obj_get_style_shadow_width(obj, 0),
                0,
                "unexpected button shadow"
            );
        }
        for i in 0..rs_lvgl_sys::lv_obj_get_child_count(obj) {
            layout(native::child(obj, i as i32));
        }
    }
}

pub fn check<A: Example>(
    name: &str,
    make: impl Fn() -> A,
    mut interact: impl FnMut(&mut Ui<A::Msg>, &mut A),
) {
    let _lock = native::LOCK.lock().unwrap();
    for cycle in 0..3 {
        let display = native::init(WIDTH, HEIGHT);
        let mut app = make();
        let theme = if cycle == 2 {
            Theme::dark()
        } else {
            Theme::light()
        };
        let mut ui = Ui::new(&display, theme).unwrap();
        ui.mount(&app).unwrap();
        native::settle(&mut ui);
        layout(native::root());
        let suffix = if cycle == 2 { "dark" } else { "light" };
        native::capture(native::screen(), &format!("root-{name}-{suffix}-initial"));
        interact(&mut ui, &mut app);
        native::settle(&mut ui);
        layout(native::root());
        native::capture(native::screen(), &format!("root-{name}-{suffix}-updated"));
        ui.unmount();
        drop(ui);
        // SAFETY: no UI/input survives and memory rendering is synchronous.
        unsafe {
            display.delete();
            rs_lvgl::deinit();
        }
    }
}
