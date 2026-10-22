//! Run: `cargo run -p rs_lvgl_ui --example sdl --features simulator`
use rs_lvgl_ui::{App, Node, Theme};
mod support;

#[derive(Clone)]
enum Msg {
    Increment,
    Decrement,
}
#[derive(Default)]
struct Counter {
    value: i32,
}
impl App for Counter {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Increment => self.value += 1,
            Msg::Decrement => self.value -= 1,
        }
    }
    fn view(&self, theme: &Theme) -> Node<Msg> {
        support::frame(
            theme,
            "Native SDL window",
            "SDL mouse events become typed application messages.",
            support::column()
                .child(support::value(theme, format!("Count: {}", self.value)))
                .child(
                    support::row()
                        .child(support::action(theme, "Subtract", Msg::Decrement))
                        .child(support::action(theme, "Add", Msg::Increment)),
                ),
        )
    }
}
impl support::Example for Counter {}
fn main() {
    support::run("sdl", Counter::default());
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_pointer_reaches_both_actions() {
        support::audit::check("sdl", Counter::default, |ui, app| {
            support::audit::click(ui, app, "Add");
            assert_eq!(app.value, 1);
            support::audit::click(ui, app, "Subtract");
            assert_eq!(app.value, 0);
            support::audit::click(ui, app, "Subtract");
            assert_eq!(app.value, -1);
        });
    }
}
