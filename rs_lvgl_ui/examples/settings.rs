//! Static settings screen built with the declarative layer.
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
use rs_lvgl_ui::{App, Key, Node, Size, Style, Theme, Ui};

#[derive(Clone, Debug)]
enum Msg {
    SpeedChanged(i32),
    ToggleMute,
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
    type Msg = Msg;

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::SpeedChanged(value) => self.speed = value,
            Msg::ToggleMute => self.muted = !self.muted,
            Msg::Select(index) => self.selected = index,
            Msg::Save => self.saves += 1,
        }
    }

    fn view(&self, theme: &Theme) -> Node<Msg> {
        let modes = ["Auto", "Manual", "Night"];

        Node::column()
            .style(Style::screen(theme).gap(12).pad(16))
            .child(Node::label("Fan settings").style(Style::title(theme)))
            .child(
                Node::row()
                    .style(Style::surface(theme).width(Size::Percent(100)).gap(12))
                    .child(
                        Node::label("Speed")
                            .key(Key::Named("speed-label"))
                            .style(Style::body(theme)),
                    )
                    .child(
                        Node::slider(0, 100, self.speed)
                            .key(Key::Named("speed"))
                            .style(Style::default().height(Size::Px(16)).flex_grow(1))
                            .on_value_changed(Msg::SpeedChanged),
                    ),
            )
            .child(
                Node::row()
                    .style(Style::surface(theme).width(Size::Percent(100)).gap(12))
                    .child(
                        Node::label(if self.muted { "Muted" } else { "Sound on" })
                            .key(Key::Named("mute-label"))
                            .style(Style::body(theme)),
                    )
                    .child(
                        Node::switch(self.muted)
                            .key(Key::Named("mute"))
                            .on_value_changed(|_| Msg::ToggleMute),
                    ),
            )
            .child(
                Node::column()
                    .key(Key::Named("modes"))
                    .style(Style::surface(theme).width(Size::Percent(100)).gap(8))
                    .child(Node::label("Mode").style(Style::muted(theme)))
                    .children(modes.iter().enumerate().map(|(index, mode)| {
                        Node::button(*mode)
                            .key(Key::Id(index as u64))
                            .style(if index == self.selected {
                                Style::accent(theme)
                            } else {
                                Style::body(theme).bg(theme.surface).shadow_width(0)
                            })
                            .on_click(Msg::Select(index))
                    })),
            )
            .child(
                Node::row()
                    .style(Style::default().width(Size::Percent(100)).gap(12))
                    .child(
                        Node::label(format!("Speed {}%", self.speed))
                            .key(Key::Named("summary"))
                            .style(Style::body(theme).flex_grow(1)),
                    )
                    .child(
                        Node::button("Save")
                            .key(Key::Named("save"))
                            .style(Style::accent(theme))
                            .on_click(Msg::Save),
                    ),
            )
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

    ui.post(Msg::SpeedChanged(85)).expect("post speed");
    ui.post(Msg::ToggleMute).expect("post mute");
    ui.post(Msg::Select(2)).expect("post select");
    ui.post(Msg::Save).expect("post save");
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
