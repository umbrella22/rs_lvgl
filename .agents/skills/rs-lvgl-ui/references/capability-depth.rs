use core::ptr::NonNull;
use rs_lvgl::NativeObject;
use rs_lvgl_sys as sys;
use rs_lvgl_ui::prelude::*;
use std::{cell::Cell, ffi::CStr, num::NonZeroU32, rc::Rc};

struct Gauge(Rc<Cell<usize>>);
struct GaugeProps(i32);
impl CustomWidget for Gauge {
    type Props = GaugeProps;
    fn validate(props: &GaugeProps) -> Result<(), UiError> {
        if (0..=100).contains(&props.0) {
            Ok(())
        } else {
            Err(UiError::InvalidProperty)
        }
    }
    fn create(&mut self, parent: &mut Obj<'_>, _: &GaugeProps) -> Result<CustomObject, UiError> {
        // SAFETY: one live direct child is returned to the mounted owner.
        unsafe {
            let pointer = NonNull::new(sys::lv_arc_create(parent.raw().as_ptr()))
                .ok_or(UiError::Allocation)?;
            CustomObject::new(parent, pointer)
        }
    }
    fn apply_props(
        &mut self,
        object: &mut CustomObject,
        props: &GaugeProps,
    ) -> Result<(), UiError> {
        // SAFETY: this hook borrows its live arc exclusively on the UI thread.
        unsafe {
            sys::lv_arc_set_value(object.raw().as_ptr(), props.0);
        }
        Ok(())
    }
    fn delete(&mut self, _: &mut CustomObject) {
        self.0.set(self.0.get() + 1);
    }
}
#[derive(Clone)]
enum Action {
    Range(SliderRange),
    Text(String),
}
struct Page {
    value: i32,
    range: SliderRange,
    text: String,
    history: ChartHistory,
    deletes: Rc<Cell<usize>>,
}
impl App for Page {
    type Action = Action;
    fn update(&mut self, action: Action) {
        match action {
            Action::Range(range) => self.range = range,
            Action::Text(text) => self.text = text,
        }
    }
    fn view(&self, _: &Theme) -> View<Action> {
        column([
            typed::range_slider(0, 100, binding(self.range, Action::Range))
                .key(Key::Id(1))
                .into_view(),
            typed::textarea(binding(self.text.as_str(), |s: &str| {
                Action::Text(s.into())
            }))
            .accepted_chars("0123456789")
            .input_max_length(NonZeroU32::new(2))
            .cursor_position(Some(1))
            .cursor_click_position(false)
            .key(Key::Id(2))
            .into_view(),
            chart(ChartSpec::new(
                ChartType::Line,
                ChartAxes::new(ChartRange::new(0, 100), ChartRange::new(0, 100)),
                vec![ChartSeries::new(
                    7,
                    Color::rgb(20, 150, 200),
                    ChartData::History(self.history.snapshot()),
                )],
            ))
            .width(px(160))
            .height(px(70))
            .key(Key::Id(3)),
            custom(Gauge(self.deletes.clone()), GaugeProps(self.value)).key(Key::Id(4)),
        ])
        .flex_wrap(true)
        .flex_reverse(true)
        .width(px(220))
        .height(px(200))
    }
}
unsafe fn child(parent: *mut sys::lv_obj_t, index: i32) -> *mut sys::lv_obj_t {
    // SAFETY: callers supply the live mounted parent and a checked child index.
    unsafe {
        let p = sys::lv_obj_get_child(parent, index);
        assert!(!p.is_null());
        p
    }
}
fn main() {
    rs_lvgl::init();
    let flushes = Rc::new(Cell::new(0));
    let output = flushes.clone();
    let display = rs_lvgl::Display::register::<_, { 240 * 8 }>(240, 220, move |_| {
        output.set(output.get() + 1)
    })
    .unwrap();
    let deletes = Rc::new(Cell::new(0));
    let mut page = Page {
        value: 30,
        range: SliderRange::new(20, 70),
        text: "A中Z".into(),
        history: ChartHistory::new(1, NonZeroU32::new(8).unwrap(), ChartSampleKind::Y),
        deletes: deletes.clone(),
    };
    page.history.push_y(20).unwrap();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    ui.mount(&page).unwrap();
    // SAFETY: the UI owns this complete live tree until unmount below.
    unsafe {
        let root = child(child(sys::lv_screen_active(), 0), 0);
        let slider = child(root, 0);
        let textarea = child(root, 1);
        let plot = child(root, 2);
        let gauge = child(root, 3);
        assert_eq!(sys::lv_slider_get_left_value(slider), 20);
        assert_eq!(sys::lv_slider_get_value(slider), 70);
        assert_eq!(
            CStr::from_ptr(sys::lv_textarea_get_text(textarea))
                .to_str()
                .unwrap(),
            "A中Z"
        );
        assert_eq!(sys::lv_textarea_get_cursor_pos(textarea), 1);
        assert!(!sys::lv_textarea_get_cursor_click_pos(textarea));
        assert_eq!(sys::lv_arc_get_value(gauge), 30);
        page.value = 65;
        page.range = SliderRange::new(25, 80);
        page.text = "model中".into();
        page.history.push_y(55).unwrap();
        ui.update(&page).unwrap();
        assert_eq!(child(root, 3), gauge);
        assert_eq!(sys::lv_arc_get_value(gauge), 65);
        assert_eq!(sys::lv_slider_get_left_value(slider), 25);
        assert_eq!(sys::lv_slider_get_value(slider), 80);
        assert_eq!(
            CStr::from_ptr(sys::lv_textarea_get_text(textarea))
                .to_str()
                .unwrap(),
            "model中"
        );
        let series = sys::lv_chart_get_series_next(plot, core::ptr::null());
        assert!(!series.is_null());
        let values = core::slice::from_raw_parts(sys::lv_chart_get_series_y_array(plot, series), 8);
        assert_eq!(&values[..2], &[20, 55]);
        sys::lv_refr_now(sys::lv_obj_get_display(root));
        assert!(flushes.get() > 0);
    }
    ui.tick(16).unwrap();
    ui.unmount();
    assert_eq!(deletes.get(), 1);
    drop(ui);
    // SAFETY: all UI/callbacks are gone; the memory-display flush is synchronous.
    unsafe {
        display.delete();
        rs_lvgl::deinit();
    }
    println!("CAPABILITY_DEPTH:PASS");
}
