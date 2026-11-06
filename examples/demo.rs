//! Run: `cargo run -p rs_lvgl_ui --example demo --features simulator`
use rs_lvgl_ui::prelude::*;
mod support;

#[derive(Clone)]
enum WatchAction {
    NextDay,
}
struct Watch {
    steps: u32,
    day: u8,
}
impl Default for Watch {
    fn default() -> Self {
        Self {
            steps: 8400,
            day: 22,
        }
    }
}
impl App for Watch {
    type Action = WatchAction;
    fn update(&mut self, _: WatchAction) {
        self.day = self.day % 28 + 1;
        self.steps = (self.steps + 1700) % 10000;
    }
    fn view(&self, theme: &Theme) -> View<WatchAction> {
        support::frame(
            theme,
            "Trail watch",
            "A small watch face built from labels, rows and a bar.",
            support::column()
                .child(support::value(theme, "09:41"))
                .child(label(format!("October {} / Morning walk", self.day)))
                .child(label(format!("{} / 10,000 steps", self.steps)))
                .child(
                    progress_bar(0, 10000, self.steps as i32)
                        .width(percent(100))
                        .height(px(16)),
                )
                .child(support::action(theme, "Next day", WatchAction::NextDay)),
        )
    }
}
impl support::Example for Watch {}
fn main() {
    support::run("demo", Watch::default());
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn watch_face_renders_new_day() {
        support::audit::check("demo", Watch::default, |ui, app| {
            support::audit::click(ui, app, "Next day");
            assert_eq!(app.day, 23);
            assert_eq!(app.steps, 100);
            support::audit::native::label(
                support::audit::native::screen(),
                "October 23 / Morning walk",
            );
        });
    }
}
