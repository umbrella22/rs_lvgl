//! Run: `cargo run -p rs_lvgl_ui --example app --features simulator`
use rs_lvgl_ui::prelude::*;
mod support;

#[derive(Clone)]
enum SettingsAction {
    Enabled(bool),
    Save,
}
struct Settings {
    enabled: bool,
    saves: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            saves: 0,
        }
    }
}
impl App for Settings {
    type Action = SettingsAction;
    fn update(&mut self, action: SettingsAction) {
        match action {
            SettingsAction::Enabled(value) => self.enabled = value,
            SettingsAction::Save => self.saves += 1,
        }
    }
    fn view(&self, theme: &Theme) -> View<SettingsAction> {
        support::frame(
            theme,
            "Device settings",
            "Compose controls around one application state.",
            support::column()
                .child(support::row().child(label("Notifications")).child(
                    switch(self.enabled).on_value_changed(|n| SettingsAction::Enabled(n != 0)),
                ))
                .child(support::action(
                    theme,
                    "Save settings",
                    SettingsAction::Save,
                ))
                .child(label(if self.saves == 0 {
                    "No changes saved".into()
                } else {
                    format!("Saved {} times", self.saves)
                })),
        )
    }
}
impl support::Example for Settings {}
fn main() {
    support::run("app", Settings::default());
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saves_application_state() {
        support::audit::check("app", Settings::default, |ui, app| {
            support::audit::click(ui, app, "Save settings");
            assert_eq!(app.saves, 1);
            ui.post(SettingsAction::Enabled(false)).unwrap();
            ui.drain(app, 8).unwrap();
            assert!(!app.enabled);
            support::audit::native::label(support::audit::native::screen(), "Saved 1 times");
        });
    }
}
