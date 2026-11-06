//! Run: `cargo run -p rs_lvgl_ui --example sdl_demo --features simulator`
use rs_lvgl_ui::components::panel::Panel;
use rs_lvgl_ui::prelude::*;
mod support;

#[derive(Clone)]
enum MixerAction {
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
    type Action = MixerAction;
    fn update(&mut self, action: MixerAction) {
        match action {
            MixerAction::Volume(value) => self.volume = value,
            MixerAction::Enabled(value) => self.enabled = value,
            MixerAction::Reset => *self = Self::default(),
        }
    }
    fn view(&self, theme: &Theme) -> View<MixerAction> {
        support::frame(
            theme,
            "Component dashboard",
            "A panel composes stateful controls without raw widgets.",
            Panel::new("Audio output")
                .style(style().height(percent(100)))
                .child(
                    support::row()
                        .child(label(format!("Volume {}%", self.volume)))
                        .child(
                            switch(self.enabled).on_value_changed(|n| MixerAction::Enabled(n != 0)),
                        ),
                )
                .child(
                    slider(0, 100, self.volume)
                        .width(percent(100))
                        .height(px(18))
                        .on_value_changed(MixerAction::Volume),
                )
                .child(
                    progress_bar(0, 100, if self.enabled { self.volume } else { 0 })
                        .width(percent(100))
                        .height(px(16)),
                )
                .child(support::action(
                    theme,
                    "Restore defaults",
                    MixerAction::Reset,
                ))
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
            ui.post(MixerAction::Volume(20)).unwrap();
            ui.post(MixerAction::Enabled(false)).unwrap();
            ui.drain(app, 8).unwrap();
            support::audit::click(ui, app, "Restore defaults");
            assert_eq!(app.volume, 65);
            assert!(app.enabled);
        });
    }
}
