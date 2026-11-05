//! Run: `cargo run -p rs_lvgl_ui --example scale --features simulator`
use rs_lvgl_ui::prelude::*;
mod support;

#[derive(Clone)]
enum ScaleAction {
    Value(i32),
    Reset,
}
struct Ruler {
    value: i32,
}
impl Default for Ruler {
    fn default() -> Self {
        Self { value: 35 }
    }
}
impl App for Ruler {
    type Action = ScaleAction;
    fn update(&mut self, action: ScaleAction) {
        self.value = match action {
            ScaleAction::Value(value) => value,
            ScaleAction::Reset => 50,
        };
    }
    fn view(&self, theme: &Theme) -> View<ScaleAction> {
        support::frame(
            theme,
            "Scale and slider",
            "A numbered scale gives context to a continuous value.",
            support::column()
                .child(support::value(theme, format!("Position {}", self.value)))
                .child(
                    scale(0, 100, 21, 5)
                        .width(percent(100))
                        .height(px(64))
                        .padding_left(20)
                        .padding_right(20)
                        .padding_top(10)
                        .radius(6)
                        .background(Color::rgb(233, 238, 243))
                        .background_opacity(255)
                        .text_color(Color::rgb(25, 35, 47)),
                )
                .child(
                    slider(0, 100, self.value)
                        .width(percent(100))
                        .height(px(18))
                        .on_value_changed(ScaleAction::Value),
                )
                .child(support::action(theme, "Center", ScaleAction::Reset)),
        )
    }
}
impl support::Example for Ruler {}
fn main() {
    support::run("scale", Ruler::default());
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn center_updates_value_and_slider() {
        support::audit::check("scale", Ruler::default, |ui, app| {
            support::audit::click(ui, app, "Center");
            assert_eq!(app.value, 50);
            support::audit::native::label(support::audit::native::screen(), "Position 50");
        });
    }
}
