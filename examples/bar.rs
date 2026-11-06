//! Run: `cargo run -p rs_lvgl_ui --example bar --features simulator`
use rs_lvgl_ui::prelude::*;
mod support;

#[derive(Clone)]
enum ProgressAction {
    Advance,
    Reset,
}
struct Progress {
    value: i32,
}
impl Default for Progress {
    fn default() -> Self {
        Self { value: 40 }
    }
}
impl App for Progress {
    type Action = ProgressAction;
    fn update(&mut self, action: ProgressAction) {
        self.value = match action {
            ProgressAction::Advance => (self.value + 10) % 110,
            ProgressAction::Reset => 0,
        };
    }
    fn view(&self, theme: &Theme) -> View<ProgressAction> {
        support::frame(
            theme,
            "Progress bar",
            "The label and bar share the same value.",
            support::column()
                .child(support::value(theme, format!("{}% complete", self.value)))
                .child(
                    progress_bar(0, 100, self.value)
                        .width(percent(100))
                        .height(px(24)),
                )
                .child(
                    support::row()
                        .child(support::action(theme, "Advance", ProgressAction::Advance))
                        .child(support::action(theme, "Reset", ProgressAction::Reset)),
                ),
        )
    }
}
impl support::Example for Progress {}
fn main() {
    support::run("bar", Progress::default());
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn progress_follows_button_messages() {
        support::audit::check("bar", Progress::default, |ui, app| {
            support::audit::click(ui, app, "Advance");
            assert_eq!(app.value, 50);
            support::audit::native::label(support::audit::native::screen(), "50% complete");
        });
    }
}
