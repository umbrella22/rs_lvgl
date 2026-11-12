//! Settings screen using the rs_lvgl_ui 0.8.0 typed construction and binding APIs.
//!
//! Run with:
//!
//! ```text
//! cargo run -p rs_lvgl_ui --example settings
//! ```
//!
//! The example registers an off-screen memory display, mounts one settings
//! screen, feeds a few messages through the queue and prints the reconcile
//! counters. It contains no `rs_lvgl::sys` calls and no raw pointers.

use rs_lvgl::Display;
use rs_lvgl_ui::prelude::*;

#[derive(Clone, Debug)]
enum FanSettingsAction {
    SpeedChanged(i32),
    MutedChanged(bool),
    Select(usize),
    Save,
}

struct Settings {
    speed: i32,
    muted: bool,
    selected: usize,
    saves: u32,
}

impl Settings {
    fn new() -> Self {
        Self {
            speed: 40,
            muted: false,
            selected: 0,
            saves: 0,
        }
    }
}

impl App for Settings {
    type Action = FanSettingsAction;

    fn update(&mut self, action: FanSettingsAction) {
        match action {
            FanSettingsAction::SpeedChanged(value) => self.speed = value,
            FanSettingsAction::MutedChanged(value) => self.muted = value,
            FanSettingsAction::Select(index) => self.selected = index,
            FanSettingsAction::Save => self.saves += 1,
        }
    }

    fn view(&self, theme: &Theme) -> View<FanSettingsAction> {
        let modes = ["Auto", "Manual", "Night"];

        column([
            label("Fan settings")
                .font(theme.title_font)
                .text_color(theme.text),
            row([
                label("Speed")
                    .key(Key::Named("speed-label"))
                    .font(theme.body_font)
                    .text_color(theme.text),
                typed::slider(0, 100, binding(self.speed, FanSettingsAction::SpeedChanged))
                    .key(Key::Named("speed"))
                    .height(px(16))
                    .grow(1)
                    .into_view(),
            ])
            .style(Style::surface(theme))
            .width(percent(100))
            .gap(12),
            row([
                label(if self.muted { "Muted" } else { "Sound on" })
                    .key(Key::Named("mute-label"))
                    .font(theme.body_font)
                    .text_color(theme.text),
                typed::switch(binding(self.muted, FanSettingsAction::MutedChanged))
                    .key(Key::Named("mute"))
                    .into_view(),
            ])
            .style(Style::surface(theme))
            .width(percent(100))
            .gap(12),
            column([label("Mode").font(theme.body_font).text_color(theme.muted)])
                .key(Key::Named("modes"))
                .style(Style::surface(theme))
                .width(percent(100))
                .gap(8)
                .children(modes.iter().enumerate().map(|(index, mode)| {
                    button(*mode)
                        .key(Key::Id(index as u64))
                        .style(if index == self.selected {
                            Style::accent(theme)
                        } else {
                            Style::body(theme).background(theme.surface).shadow_width(0)
                        })
                        .on_click(FanSettingsAction::Select(index))
                })),
            row([
                label(format!("Speed {}%", self.speed))
                    .key(Key::Named("summary"))
                    .font(theme.body_font)
                    .text_color(theme.text)
                    .grow(1),
                button("Save")
                    .key(Key::Named("save"))
                    .style(Style::accent(theme))
                    .on_click(FanSettingsAction::Save),
            ])
            .width(percent(100))
            // The native focus outline extends outside the button's layout box.
            .padding(6)
            .gap(12),
        ])
        .style(Style::screen(theme))
        .gap(12)
        .padding(16)
        .child_if(self.saves > 0, || {
            label(format!("Saved {} times", self.saves)).key(Key::Named("saved"))
        })
    }
}

fn main() {
    rs_lvgl::init();

    let display = Display::register::<_, { 320 * 240 }>(320, 240, |_| {})
        .expect("memory display registration");
    let mut ui = Ui::new(&display, Theme::dark()).expect("ui creation");
    let mut app = Settings::new();

    ui.mount(&app).expect("initial mount");
    let mounted = ui.stats();

    ui.post(FanSettingsAction::SpeedChanged(85))
        .expect("post speed");
    ui.post(FanSettingsAction::MutedChanged(true))
        .expect("post mute");
    ui.post(FanSettingsAction::Select(2)).expect("post select");
    ui.post(FanSettingsAction::Save).expect("post save");
    let drained = ui.drain(&mut app, 16).expect("drain");

    let updated = ui.stats();
    println!(
        "mounted: {} nodes, reconciled {} times",
        mounted.mounted_nodes, updated.reconciled_updates
    );
    println!(
        "drain: processed {}, dropped {}, reconciled {}",
        drained.processed, drained.dropped, drained.reconciled
    );
    println!(
        "created {}, deleted {}, moved {}",
        updated.created_nodes, updated.deleted_nodes, updated.moved_nodes
    );
    println!(
        "state: speed={} muted={} selected={} saves={}",
        app.speed, app.muted, app.selected, app.saves
    );

    assert_eq!(app.speed, 85);
    assert!(app.muted);
    assert_eq!(app.selected, 2);
    assert_eq!(app.saves, 1);
    assert_eq!(drained.processed, 4);
    assert!(drained.reconciled);

    ui.unmount();
    assert_eq!(ui.stats().mounted_nodes, 0);
    drop(ui);
    // SAFETY: the UI is unmounted and no rendering, callback or handle remains.
    unsafe { display.delete() };
    unsafe { rs_lvgl::deinit() };
}

#[cfg(test)]
#[path = "support/settings_audit.rs"]
mod audit;
