//! Run: `cargo run -p rs_lvgl_ui --example arc --features simulator`
use rs_lvgl_ui::prelude::*;
mod support;

#[derive(Clone)]
enum GaugeAction {
    Value(i32),
    Step,
}
struct Gauge {
    value: i32,
}
impl Default for Gauge {
    fn default() -> Self {
        Self { value: 64 }
    }
}
impl App for Gauge {
    type Action = GaugeAction;
    fn update(&mut self, action: GaugeAction) {
        self.value = match action {
            GaugeAction::Value(value) => value,
            GaugeAction::Step => (self.value + 10) % 101,
        };
    }
    fn view(&self, theme: &Theme) -> View<GaugeAction> {
        support::frame(
            theme,
            "Arc control",
            "Drag the dial or advance the value with a button.",
            support::column()
                .child(
                    support::row()
                        .child(
                            arc(0, 100, self.value)
                                .width(px(176))
                                .height(px(176))
                                .padding(12)
                                .arc_color(theme.accent)
                                .native_part_style(
                                    NativePart::Knob,
                                    style()
                                        .background(theme.accent)
                                        .background_opacity(255)
                                        .radius(12)
                                        .padding(6),
                                )
                                .clickable(true)
                                .on_value_changed(GaugeAction::Value),
                        )
                        .child(support::value(theme, format!("{}% output", self.value))),
                )
                .child(support::action(theme, "Increase", GaugeAction::Step)),
        )
    }
}
impl support::Example for Gauge {}
fn main() {
    support::run("arc", Gauge::default());
}
#[cfg(test)]
mod tests {
    use super::*;
    use support::audit::native;

    // Bindgen omits class variables; the pinned native header exports this class.
    unsafe extern "C" {
        static lv_arc_class: rs_lvgl_sys::lv_obj_class_t;
    }

    #[test]
    fn gauge_value_is_controlled() {
        support::audit::check("arc", Gauge::default, |ui, app| {
            support::audit::click(ui, app, "Increase");
            assert_eq!(app.value, 74);
            support::audit::native::label(support::audit::native::screen(), "74% output");
            let arc = native::class(native::screen(), &raw const lv_arc_class);
            let [x1, y1, x2, _] = native::rect(arc);
            let mut pointer = native::Pointer::new();
            for _ in 0..10 {
                rs_lvgl::tick_inc(std::time::Duration::from_millis(30));
                pointer.read((x1 + x2) / 2, y1 + 18, true);
                ui.drain(app, 64).unwrap();
            }
            pointer.read((x1 + x2) / 2, y1 + 18, false);
            ui.drain(app, 64).unwrap();
            assert!(app.value < 74, "dragging the dial must update its value");
        });
    }
}
