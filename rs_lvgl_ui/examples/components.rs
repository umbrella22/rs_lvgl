//! Composite components: stack navigation, lists, a panel, a dialog and
//! toasts.
//!
//! Run with:
//!
//! ```text
//! cargo run -p rs_lvgl_ui --example components --features simulator
//! ```
//!
//! Open the settings pages, toggle options, open the log panel, or press
//! "Reset" to see a confirmation dialog. Saving shows a toast.
//!
//! Set `COMPONENTS_FRAMES=<n>` for a bounded, non-interactive smoke run.

use core::mem;
use rs_lvgl_ui::prelude::*;
use std::thread::sleep;
use std::time::{Duration, Instant};

use rs_lvgl::Display;
use rs_lvgl_ui::components::dialog::Dialog;
use rs_lvgl_ui::components::layers;
use rs_lvgl_ui::components::list::{self, ListItem};
use rs_lvgl_ui::components::nav::{self, NavMsg, NavState};
use rs_lvgl_ui::components::panel::{self, Panel};
use rs_lvgl_ui::components::toast::{self, ToastKind, Toasts};

#[path = "support/runtime.rs"]
mod runtime;

const WIDTH: i32 = 480;
const HEIGHT: i32 = 320;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Root,
    Display,
    Logs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Brightness {
    Low,
    Medium,
    High,
}

const BRIGHTNESS: [(Brightness, &str); 3] = [
    (Brightness::Low, "Low"),
    (Brightness::Medium, "Medium"),
    (Brightness::High, "High"),
];

#[derive(Clone, Debug)]
enum Msg {
    Nav(NavMsg<Page>),
    ToggleWifi,
    Brightness(Brightness),
    Save,
    AskReset,
    CancelReset,
    Reset,
    DismissToast(u64),
    Tick(u32),
    Quit,
}

struct Demo {
    nav: NavState<Page>,
    wifi: bool,
    brightness: Brightness,
    confirm_reset: bool,
    toasts: Toasts,
    log: Vec<String>,
    quit: bool,
}

impl Demo {
    fn new() -> Self {
        Self {
            nav: NavState::new(Page::Root),
            wifi: true,
            brightness: Brightness::Medium,
            confirm_reset: false,
            toasts: Toasts::with_max_visible(3),
            log: vec!["boot ok".into()],
            quit: false,
        }
    }

    fn root_page(&self, theme: &Theme) -> View<Msg> {
        list::list(
            theme,
            [
                list::section(theme, "Network").key(Key::Named("net")),
                ListItem::new("Wi-Fi")
                    .icon(symbol::WIFI)
                    .trailing(switch(self.wifi).on_value_changed(|_| Msg::ToggleWifi))
                    .view(theme)
                    .key(Key::Named("wifi")),
                list::section(theme, "Device").key(Key::Named("device")),
                ListItem::new("Display")
                    .icon(symbol::IMAGE)
                    .detail(name(self.brightness))
                    .chevron(true)
                    .on_click(Msg::Nav(NavMsg::Push(Page::Display)))
                    .view(theme)
                    .key(Key::Named("display")),
                nav::link(theme, "Logs", Msg::Nav(NavMsg::Push(Page::Logs)))
                    .key(Key::Named("logs")),
                ListItem::new("Save")
                    .icon(symbol::SAVE)
                    .on_click(Msg::Save)
                    .view(theme)
                    .key(Key::Named("save")),
                ListItem::new("Reset")
                    .icon(symbol::TRASH)
                    .on_click(Msg::AskReset)
                    .view(theme)
                    .key(Key::Named("reset")),
                ListItem::new("Quit")
                    .icon(symbol::CLOSE)
                    .on_click(Msg::Quit)
                    .view(theme)
                    .key(Key::Named("quit")),
            ],
        )
    }

    fn display_page(&self, theme: &Theme) -> View<Msg> {
        list::list(
            theme,
            BRIGHTNESS.iter().map(|(level, label)| {
                ListItem::new(*label)
                    .selected(*level == self.brightness)
                    .on_click(Msg::Brightness(*level))
                    .view(theme)
                    .key(Key::Named(label))
            }),
        )
    }

    fn logs_page(&self, theme: &Theme) -> View<Msg> {
        Panel::new("Events")
            .action(panel::icon_button(theme, symbol::SAVE, Msg::Save))
            .children(
                self.log
                    .iter()
                    .enumerate()
                    .map(|(index, line)| label(line.clone()).key(Key::Id(index as u64))),
            )
            .style(Style::default().flex_grow(1).height(Size::Percent(100)))
            .view(theme)
    }

    fn record(&mut self, line: impl Into<String>) {
        self.log.push(line.into());
    }
}

fn name(level: Brightness) -> &'static str {
    BRIGHTNESS
        .iter()
        .find(|(candidate, _)| *candidate == level)
        .map(|(_, label)| *label)
        .unwrap_or("?")
}

impl App for Demo {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Nav(msg) => self.nav.update(msg),
            Msg::ToggleWifi => {
                self.wifi = !self.wifi;
                self.record(if self.wifi { "wifi on" } else { "wifi off" });
            }
            Msg::Brightness(level) => {
                self.brightness = level;
                self.record(format!("brightness {}", name(level)));
            }
            Msg::Save => {
                self.toasts
                    .push_kind(ToastKind::Success, "Settings saved", 2000);
                self.record("saved");
            }
            Msg::AskReset => self.confirm_reset = true,
            Msg::CancelReset => self.confirm_reset = false,
            Msg::Reset => {
                self.confirm_reset = false;
                self.wifi = true;
                self.brightness = Brightness::Medium;
                self.nav.update(NavMsg::Root);
                self.toasts.push("Settings reset", 2000);
                self.record("reset");
            }
            Msg::DismissToast(id) => self.toasts.dismiss(id),
            Msg::Tick(elapsed) => {
                self.toasts.advance(elapsed);
            }
            Msg::Quit => self.quit = true,
        }
    }

    fn view(&self, theme: &Theme) -> View<Msg> {
        let (title, page) = match self.nav.current() {
            Page::Root => ("Settings", self.root_page(theme)),
            Page::Display => ("Display", self.display_page(theme)),
            Page::Logs => ("Logs", self.logs_page(theme)),
        };
        nav::view(&self.nav, theme, title, page, Msg::Nav(NavMsg::Back))
    }

    fn overlay(&self, theme: &Theme) -> Option<View<Msg>> {
        if !self.confirm_reset && self.toasts.is_empty() {
            return None;
        }
        let mut children = Vec::new();
        if self.confirm_reset {
            children.push(
                Dialog::new("Reset settings?")
                    .text("Wi-Fi and display settings return to their defaults.")
                    .button("Cancel", Msg::CancelReset)
                    .primary("Reset", Msg::Reset)
                    .on_dismiss(Msg::CancelReset)
                    .view(theme),
            );
        }
        children.push(toast::view(&self.toasts, theme, Msg::DismissToast));
        Some(layers(children))
    }
}

fn main() {
    let frame_limit: Option<u32> = std::env::var("COMPONENTS_FRAMES")
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

    let display = Display::default();
    let mut ui = Ui::new(&display, Theme::dark()).expect("ui creation");
    let mut app = Demo::new();
    ui.mount(&app).expect("mount");

    let mut last = Instant::now();
    let mut frames = 0u32;
    loop {
        let now = Instant::now();
        let elapsed = now.duration_since(last);
        rs_lvgl::tick_inc(elapsed);
        last = now;

        // Toasts count down in application state. Posting a tick only while
        // toasts are visible keeps an idle loop from re-rendering.
        if !app.toasts.is_empty() {
            let _ = ui.post(Msg::Tick(elapsed.as_millis() as u32));
        }
        match runtime::frame(&mut ui, &mut app, 32) {
            Ok(Some(_)) => {}
            Ok(None) => {
                mem::forget(ui);
                return;
            }
            Err(error) => {
                eprintln!("ui error: {error}");
                break;
            }
        }
        if app.quit {
            break;
        }

        frames += 1;
        if frame_limit.is_some_and(|limit| frames >= limit) {
            break;
        }

        sleep(Duration::from_millis(5));
    }

    ui.unmount();
    drop(ui);
    // SAFETY: the loop stopped, LVGL handled all events, and the UI is
    // unmounted. The SDL backend's display is deleted by `deinit`.
    unsafe { rs_lvgl::deinit() };
}

#[cfg(test)]
#[path = "support/components_audit.rs"]
mod audit;
