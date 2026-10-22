//! Run: `cargo run -p rs_lvgl_ui --example sdl_demo --features simulator`
use rs_lvgl_ui::{components::panel::Panel, App, Node, Size, Style, Theme};
mod support;

#[derive(Clone)]
enum Msg {
    Volume(i32),
    Enabled(bool),
    Reset,
}
struct Mixer {
    volume: i32,
    enabled: bool,
}
impl Default for Mixer {
    fn default() -> Self {
        Self {
            volume: 65,
            enabled: true,
        }
    }
}
impl App for Mixer {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Volume(value) => self.volume = value,
            Msg::Enabled(value) => self.enabled = value,
            Msg::Reset => *self = Self::default(),
        }
    }
    fn view(&self, theme: &Theme) -> Node<Msg> {
        support::frame(
            theme,
            "Component dashboard",
            "A panel composes stateful controls without raw widgets.",
            Panel::new("Audio output")
                .style(Style::default().height(Size::Percent(100)))
                .child(
                    support::row()
                        .child(Node::label(format!("Volume {}%", self.volume)))
                        .child(
                            Node::switch(self.enabled).on_value_changed(|n| Msg::Enabled(n != 0)),
                        ),
                )
                .child(
                    Node::slider(0, 100, self.volume)
                        .style(
                            Style::default()
                                .width(Size::Percent(100))
                                .height(Size::Px(18)),
                        )
                        .on_value_changed(Msg::Volume),
                )
                .child(
                    Node::bar(0, 100, if self.enabled { self.volume } else { 0 }).style(
                        Style::default()
                            .width(Size::Percent(100))
                            .height(Size::Px(16)),
                    ),
                )
                .child(support::action(theme, "Restore defaults", Msg::Reset))
                .view(theme),
        )
    }
}
impl support::Example for Mixer {}
fn main() {
    support::run("sdl_demo", Mixer::default());
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn panel_restores_shared_state() {
        support::audit::check("sdl_demo", Mixer::default, |ui, app| {
            ui.post(Msg::Volume(20)).unwrap();
            ui.post(Msg::Enabled(false)).unwrap();
            ui.drain(app, 8).unwrap();
            support::audit::click(ui, app, "Restore defaults");
            assert_eq!(app.volume, 65);
            assert!(app.enabled);
        });
    }
}
