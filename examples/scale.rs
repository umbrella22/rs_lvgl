//! Run: `cargo run -p rs_lvgl_ui --example scale --features simulator`
use rs_lvgl_ui::{App, Color, Node, Size, Style, Theme};
mod support;

#[derive(Clone)]
enum Msg {
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
    type Msg = Msg;
    fn update(&mut self, msg: Msg) {
        self.value = match msg {
            Msg::Value(value) => value,
            Msg::Reset => 50,
        };
    }
    fn view(&self, theme: &Theme) -> Node<Msg> {
        support::frame(
            theme,
            "Scale and slider",
            "A numbered scale gives context to a continuous value.",
            support::column()
                .child(support::value(theme, format!("Position {}", self.value)))
                .child(
                    Node::scale(0, 100, 21, 5).style(
                        Style::default()
                            .width(Size::Percent(100))
                            .height(Size::Px(64))
                            .pad_left(20)
                            .pad_right(20)
                            .pad_top(10)
                            .radius(6)
                            .bg(Color::rgb(233, 238, 243))
                            .bg_opacity(255)
                            .fg(Color::rgb(25, 35, 47)),
                    ),
                )
                .child(
                    Node::slider(0, 100, self.value)
                        .style(
                            Style::default()
                                .width(Size::Percent(100))
                                .height(Size::Px(18)),
                        )
                        .on_value_changed(Msg::Value),
                )
                .child(support::action(theme, "Center", Msg::Reset)),
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
