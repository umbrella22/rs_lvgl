//! Run: `cargo run -p rs_lvgl_ui --example bar --features simulator`
use rs_lvgl_ui::prelude::*;
mod support;

#[derive(Clone)]
enum Msg {
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
    type Msg = Msg;
    fn update(&mut self, msg: Msg) {
        self.value = match msg {
            Msg::Advance => (self.value + 10) % 110,
            Msg::Reset => 0,
        };
    }
    fn view(&self, theme: &Theme) -> View<Msg> {
        support::frame(
            theme,
            "Progress bar",
            "The label and bar share the same value.",
            support::column()
                .child(support::value(theme, format!("{}% complete", self.value)))
                .child(
                    bar(0, 100, self.value).style(
                        Style::default()
                            .width(Size::Percent(100))
                            .height(Size::Px(24)),
                    ),
                )
                .child(
                    support::row()
                        .child(support::action(theme, "Advance", Msg::Advance))
                        .child(support::action(theme, "Reset", Msg::Reset)),
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
