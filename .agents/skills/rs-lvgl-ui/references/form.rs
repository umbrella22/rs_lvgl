//! Controlled form using rs_lvgl_ui 0.7.0.
//! rs_lvgl (alloc); rs_lvgl_ui (alloc, widget-slider, widget-switch, widget-textarea).
use rs_lvgl_ui::prelude::*;

#[derive(Clone)]
enum FanSettingsAction {
    SpeedChanged(i32),
    EnabledChanged(bool),
    NameChanged(String),
}

struct FanSettings {
    speed: i32,
    enabled: bool,
    name: String,
}

fn fields(settings: &FanSettings) -> View<FanSettingsAction> {
    typed::column([])
        .child(
            typed::slider(
                0,
                100,
                binding(settings.speed, FanSettingsAction::SpeedChanged),
            )
            .key(Key::Named("speed")),
        )
        .child(
            typed::switch(binding(settings.enabled, FanSettingsAction::EnabledChanged))
                .key(Key::Named("enabled")),
        )
        .child(
            typed::textarea(binding(settings.name.as_str(), |text: &str| {
                FanSettingsAction::NameChanged(text.into())
            }))
            .placeholder("Device name")
            .key(Key::Named("name")),
        )
        .child_if(settings.enabled, || {
            typed::label("Fan enabled").key(Key::Named("hint"))
        })
        // Reserve ink space inside this nested container for the slider knob.
        .padding(12)
        .gap(12)
        .into_view()
}

impl App for FanSettings {
    type Action = FanSettingsAction;
    fn update(&mut self, action: FanSettingsAction) {
        match action {
            // Application policy: reject speeds above 50. Native correction
            // still requires reconciliation when the model remains unchanged.
            FanSettingsAction::SpeedChanged(value) if value <= 50 => self.speed = value,
            FanSettingsAction::SpeedChanged(_) => {}
            FanSettingsAction::EnabledChanged(value) => self.enabled = value,
            FanSettingsAction::NameChanged(value) => self.name = value,
        }
    }
    fn view(&self, theme: &Theme) -> View<FanSettingsAction> {
        column([
            typed::label("Fan settings")
                .font(theme.title_font)
                .text_color(theme.text)
                .into_view(),
            fields(self),
        ])
        .padding(16)
        .gap(12)
    }
}

fn main() {
    rs_lvgl::init();
    let display = rs_lvgl::Display::register::<_, { 320 * 240 }>(320, 240, |_| {}).unwrap();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut settings = FanSettings {
        speed: 50,
        enabled: false,
        name: "Desk".into(),
    };
    ui.mount(&settings).unwrap();
    ui.post(FanSettingsAction::SpeedChanged(80)).unwrap();
    ui.post(FanSettingsAction::EnabledChanged(true)).unwrap();
    ui.post(FanSettingsAction::NameChanged("Bedroom".into()))
        .unwrap();
    assert_eq!(ui.drain(&mut settings, 8).unwrap().processed, 3);
    assert_eq!(settings.speed, 50);
    assert!(settings.enabled);
    assert_eq!(settings.name, "Bedroom");
    ui.unmount();
    drop(ui);
    // SAFETY: all UI data is released and this memory display flushes synchronously.
    unsafe {
        display.delete();
        rs_lvgl::deinit();
    }
}
