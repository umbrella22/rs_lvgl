//! Interactive gallery for the declarative layer.
//!
//! Run with:
//!
//! ```text
//! cargo run -p rs_lvgl_ui --example gallery --features simulator
//! ```
//!
//! The gallery uses LVGL's native SDL2 backend, so mouse input and window
//! events are handled in C by LVGL itself. Click the buttons, drag the slider,
//! toggle the switch, reorder the keyed list, switch the theme, or press Quit.
//!
//! Set `GALLERY_FRAMES=<n>` for a bounded, non-interactive smoke run.

use core::mem;
use rs_lvgl_ui::prelude::*;
use std::thread::sleep;
use std::time::{Duration, Instant};

use rs_lvgl::Display;

#[path = "support/runtime.rs"]
mod runtime;

const WIDTH: i32 = 480;
const HEIGHT: i32 = 320;

#[derive(Clone, Debug)]
enum Msg {
    Inc,
    Dec,
    Speed(i32),
    ToggleMute,
    MoveDown(u64),
    Add,
    ToggleTheme,
    Quit,
}

struct Gallery {
    count: i32,
    speed: i32,
    muted: bool,
    dark: bool,
    items: Vec<u64>,
    next_id: u64,
    quit: bool,
}

impl Gallery {
    fn new() -> Self {
        Self {
            count: 0,
            speed: 35,
            muted: false,
            dark: true,
            items: vec![1, 2, 3, 4],
            next_id: 5,
            quit: false,
        }
    }
}

impl App for Gallery {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Inc => self.count += 1,
            Msg::Dec => self.count -= 1,
            Msg::Speed(value) => self.speed = value,
            Msg::ToggleMute => self.muted = !self.muted,
            Msg::MoveDown(id) => {
                if let Some(index) = self.items.iter().position(|item| *item == id) {
                    let item = self.items.remove(index);
                    let target = if index + 1 < self.items.len() {
                        index + 1
                    } else {
                        0
                    };
                    self.items.insert(target, item);
                }
            }
            Msg::Add => {
                self.items.push(self.next_id);
                self.next_id += 1;
            }
            Msg::ToggleTheme => self.dark = !self.dark,
            Msg::Quit => self.quit = true,
        }
    }

    fn view(&self, theme: &Theme) -> View<Msg> {
        column([
            row([
                label("Mosaico gallery")
                    .key(Key::Named("title"))
                    .style(Style::title(theme).flex_grow(1)),
                button("Theme")
                    .key(Key::Named("theme"))
                    .style(Style::accent(theme))
                    .on_click(Msg::ToggleTheme),
                button("Quit")
                    .key(Key::Named("quit"))
                    .style(Style::accent(theme))
                    .on_click(Msg::Quit),
            ])
            .style(Style::default().width(Size::Percent(100)).gap(8)),
            row([
                button("-")
                    .key(Key::Named("dec"))
                    .style(Style::accent(theme))
                    .on_click(Msg::Dec),
                label(format!("Count {}", self.count))
                    .key(Key::Named("count"))
                    .style(Style::body(theme).flex_grow(1)),
                button("+")
                    .key(Key::Named("inc"))
                    .style(Style::accent(theme))
                    .on_click(Msg::Inc),
            ])
            .style(Style::surface(theme).width(Size::Percent(100)).gap(12)),
            row([
                label(format!("Speed {}%", self.speed))
                    .key(Key::Named("speed-label"))
                    .style(Style::body(theme)),
                slider(0, 100, self.speed)
                    .key(Key::Named("speed"))
                    .style(Style::default().height(Size::Px(16)).flex_grow(1))
                    .on_value_changed(Msg::Speed),
            ])
            .style(Style::surface(theme).width(Size::Percent(100)).gap(12)),
            row([
                label(if self.muted { "Muted" } else { "Sound on" })
                    .key(Key::Named("mute-label"))
                    .style(Style::body(theme).flex_grow(1)),
                switch(self.muted)
                    .key(Key::Named("mute"))
                    .on_value_changed(|_| Msg::ToggleMute),
            ])
            .style(Style::surface(theme).width(Size::Percent(100)).gap(12)),
            column([label("Keyed list").style(Style::muted(theme))])
                .key(Key::Named("list"))
                .style(Style::surface(theme).width(Size::Percent(100)).gap(8))
                .children(self.items.iter().map(|id| {
                    row([
                        label(format!("Item {id}"))
                            .key(Key::Named("label"))
                            .style(Style::body(theme).flex_grow(1)),
                        button("Move down")
                            .key(Key::Named("down"))
                            .style(Style::accent(theme))
                            .on_click(Msg::MoveDown(*id)),
                    ])
                    .key(Key::Id(*id))
                    .style(Style::default().width(Size::Percent(100)).gap(8))
                })),
            button("Add item")
                .key(Key::Named("add"))
                .style(Style::accent(theme))
                .on_click(Msg::Add),
        ])
        .style(Style::screen(theme).gap(12).pad(16))
    }
}

fn main() {
    let frame_limit: Option<u32> = std::env::var("GALLERY_FRAMES")
        .ok()
        .and_then(|value| value.parse().ok());

    // SAFETY: LVGL is initialized before the display is created, and the loop
    // below keeps all access on this thread.
    unsafe {
        rs_lvgl::init();
        if rs_lvgl_sys::lv_sdl_window_create(WIDTH, HEIGHT).is_null() {
            panic!("lv_sdl_window_create failed");
        }
        if rs_lvgl_sys::lv_sdl_mouse_create().is_null() {
            panic!("lv_sdl_mouse_create failed");
        }
    }
    println!("SDL window created: {WIDTH}x{HEIGHT} (LVGL native mouse input)");

    let display = Display::default();
    // `Ui::new` mounts on the active screen created by the SDL backend.
    let mut ui = Ui::new(&display, Theme::dark()).expect("ui creation");
    let mut app = Gallery::new();
    ui.mount(&app).expect("mount");

    let mut last = Instant::now();
    let mut active_dark = true;
    let mut frames = 0u32;
    'running: loop {
        let now = Instant::now();
        rs_lvgl::tick_inc(now.duration_since(last));
        last = now;

        // LVGL dispatches callbacks from the timer handler. They only enqueue
        // messages, so application state and the view are updated below.
        match runtime::frame(&mut ui, &mut app, 32) {
            Ok(Some(_)) => {}
            Ok(None) => {
                println!("SDL window closed");
                mem::forget(ui);
                return;
            }
            Err(error) => {
                eprintln!("ui error: {error}");
                break;
            }
        }
        if app.quit {
            break 'running;
        }
        if app.dark != active_dark {
            active_dark = app.dark;
            ui.set_theme(if active_dark {
                Theme::dark()
            } else {
                Theme::light()
            });
            if let Err(error) = ui.update(&app) {
                eprintln!("ui error: {error}");
                break;
            }
        }

        frames += 1;
        if frame_limit.is_some_and(|limit| frames >= limit) {
            break 'running;
        }

        sleep(Duration::from_millis(5));
    }

    let stats = ui.stats();
    println!(
        "gallery stopped: {} nodes, {} messages, created {}, deleted {}, moved {}",
        stats.mounted_nodes,
        stats.messages_processed,
        stats.created_nodes,
        stats.deleted_nodes,
        stats.moved_nodes
    );

    ui.unmount();
    drop(ui);
    // SAFETY: the loop stopped, LVGL handled all events, and the UI is
    // unmounted. The SDL backend's display is deleted by `deinit`.
    unsafe { rs_lvgl::deinit() };
}

#[cfg(test)]
#[path = "support/gallery_audit.rs"]
mod audit;
