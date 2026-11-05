//! Controlled bindings keep update authoritative, including rejected input.
#![cfg(all(
    feature = "widget-slider",
    feature = "widget-switch",
    feature = "widget-textarea"
))]
mod common;
use common::{app_root, child, init, lock};
use rs_lvgl_ui::prelude::*;
use std::{cell::Cell, ffi::CStr, rc::Rc};

#[derive(Clone, Debug, PartialEq)]
enum FieldAction {
    Speed(i32),
    Enabled(bool),
    Name(String),
}
#[derive(Clone, Debug, PartialEq)]
enum FormAction {
    Field(FieldAction),
}
struct Form {
    speed: i32,
    enabled: bool,
    name: String,
    accept: bool,
    received: Vec<FormAction>,
}
impl App for Form {
    type Action = FormAction;
    fn update(&mut self, action: FormAction) {
        if self.accept {
            match &action {
                FormAction::Field(FieldAction::Speed(value)) => self.speed = *value,
                FormAction::Field(FieldAction::Enabled(value)) => self.enabled = *value,
                FormAction::Field(FieldAction::Name(value)) => self.name = value.clone(),
            }
        }
        self.received.push(action);
    }
    fn view(&self, _: &Theme) -> View<FormAction> {
        typed::column([])
            .child(typed::slider(0, 100, binding(self.speed, FieldAction::Speed)).key(Key::Id(1)))
            .child(typed::switch(binding(self.enabled, FieldAction::Enabled)).key(Key::Id(2)))
            .child(
                typed::textarea(binding(self.name.as_str(), |text: &str| {
                    FieldAction::Name(text.into())
                }))
                .placeholder("Name")
                .key(Key::Id(3)),
            )
            .map(FormAction::Field)
            .into_view()
    }
}
fn send_value(node: *mut rs_lvgl_sys::lv_obj_t) {
    // SAFETY: called only for live controls in the mounted test tree.
    unsafe {
        rs_lvgl_sys::lv_obj_send_event(
            node,
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            core::ptr::null_mut(),
        );
    }
}

#[test]
fn rejected_native_input_restores_the_unchanged_model_without_echo() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut form = Form {
        speed: 50,
        enabled: false,
        name: "model".into(),
        accept: false,
        received: vec![],
    };
    ui.mount(&form).unwrap();
    let slider = child(app_root(), 0).as_ptr();
    let switch = child(app_root(), 1).as_ptr();
    let textarea = child(app_root(), 2).as_ptr();
    // SAFETY: these native objects are live and have the declared concrete kinds.
    unsafe {
        rs_lvgl_sys::lv_slider_set_value(slider, 80, false);
        rs_lvgl_sys::lv_obj_add_state(switch, rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED);
    }
    send_value(slider);
    send_value(switch);
    // The textarea setter itself emits VALUE_CHANGED.
    unsafe { rs_lvgl_sys::lv_textarea_set_text(textarea, c"native".as_ptr()) };
    assert!(
        form.received.is_empty(),
        "native callbacks only queue actions"
    );
    let drained = ui.drain(&mut form, 8).unwrap();
    assert_eq!(drained.processed, 3);
    assert!(drained.reconciled);
    assert_eq!(
        (form.speed, form.enabled, form.name.as_str()),
        (50, false, "model")
    );
    assert_eq!(
        form.received,
        [
            FormAction::Field(FieldAction::Speed(80)),
            FormAction::Field(FieldAction::Enabled(true)),
            FormAction::Field(FieldAction::Name("native".into())),
        ]
    );
    // SAFETY: reconciliation keeps these keyed objects alive.
    unsafe {
        assert_eq!(rs_lvgl_sys::lv_slider_get_value(slider), 50);
        assert!(!rs_lvgl_sys::lv_obj_has_state(
            switch,
            rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED
        ));
        assert_eq!(
            CStr::from_ptr(rs_lvgl_sys::lv_textarea_get_text(textarea)).to_bytes(),
            b"model"
        );
    }
    assert_eq!(
        ui.drain(&mut form, 8).unwrap().processed,
        0,
        "no correction echo"
    );

    form.accept = true;
    unsafe { rs_lvgl_sys::lv_slider_set_value(slider, 25, false) };
    send_value(slider);
    assert_eq!(ui.drain(&mut form, 8).unwrap().processed, 1);
    assert_eq!(form.speed, 25);
    assert_eq!(
        child(app_root(), 0).as_ptr(),
        slider,
        "binding retains identity"
    );
    assert_eq!(ui.drain(&mut form, 8).unwrap().processed, 0);
}

struct DropProbe(Rc<Cell<u32>>);
impl Drop for DropProbe {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}
#[test]
fn explicit_handler_replacement_drops_binding_capture_once() {
    let drops = Rc::new(Cell::new(0));
    let probe = DropProbe(drops.clone());
    let node = typed::switch(binding(false, move |_: bool| {
        let _ = &probe;
    }));
    assert_eq!(drops.get(), 0);
    let node = node.padding(4).on_toggle(|_| ());
    assert_eq!(
        drops.get(),
        1,
        "replacement releases the single value handler"
    );
    drop(node);
    assert_eq!(drops.get(), 1);
}

#[test]
fn typed_markers_do_not_enlarge_descriptors() {
    assert_eq!(
        core::mem::size_of::<View<()>>(),
        core::mem::size_of::<rs_lvgl_ui::Node<(), typed::kind::Slider>>()
    );
    assert_eq!(
        core::mem::align_of::<View<()>>(),
        core::mem::align_of::<rs_lvgl_ui::Node<(), typed::kind::Textarea>>()
    );
}
