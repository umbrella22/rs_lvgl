//! Run: `cargo run -p rs_lvgl_ui --example demo --features simulator`
use rs_lvgl_ui::{App, Node, Size, Style, Theme};
mod support;

#[derive(Clone)]
enum Msg {
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
    type Msg = Msg;
    fn update(&mut self, _: Msg) {
        self.day = self.day % 28 + 1;
        self.steps = (self.steps + 1700) % 10000;
    }
    fn view(&self, theme: &Theme) -> Node<Msg> {
        support::frame(
            theme,
            "Trail watch",
            "A small watch face built from labels, rows and a bar.",
            support::column()
                .child(support::value(theme, "09:41"))
                .child(Node::label(format!("October {} / Morning walk", self.day)))
                .child(Node::label(format!("{} / 10,000 steps", self.steps)))
                .child(
                    Node::bar(0, 10000, self.steps as i32).style(
                        Style::default()
                            .width(Size::Percent(100))
                            .height(Size::Px(16)),
                    ),
                )
                .child(support::action(theme, "Next day", Msg::NextDay)),
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
