//! Run: `cargo run -p rs_lvgl_ui --example button_click --features simulator`
use rs_lvgl_ui::{App, Node, Theme};
mod support;

#[derive(Clone)]
enum Msg {
    Increment,
}
#[derive(Default)]
struct Counter {
    count: u32,
}
impl App for Counter {
    type Msg = Msg;
    fn update(&mut self, _: Msg) {
        self.count += 1;
    }
    fn view(&self, theme: &Theme) -> Node<Msg> {
        support::frame(
            theme,
            "Button clicks",
            "A click sends a message and updates Rust state.",
            support::column()
                .child(support::value(theme, format!("{} clicks", self.count)))
                .child(support::action(theme, "Add one", Msg::Increment)),
        )
    }
}
impl support::Example for Counter {}
fn main() {
    support::run("button_click", Counter::default());
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn click_updates_rendered_count() {
        support::audit::check("button_click", Counter::default, |ui, app| {
            support::audit::click(ui, app, "Add one");
            assert_eq!(app.count, 1);
            support::audit::native::label(support::audit::native::screen(), "1 clicks");
        });
    }
}
