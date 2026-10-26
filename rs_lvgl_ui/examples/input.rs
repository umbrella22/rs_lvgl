//! Native touch/mouse and keyboard example.
//! Run with `cargo run -p rs_lvgl_ui --example input --features simulator,input-keypad`.
//! Set INPUT_FRAMES for a bounded run; Tab changes focus and Enter activates.

use core::ptr::NonNull;
use rs_lvgl_ui::prelude::*;
use std::time::{Duration, Instant};

#[derive(Clone)]
enum Msg {
    Add,
    Reset,
    Key(u32),
    Scroll(ScrollOffset),
}

struct InputDemo {
    count: u32,
    last: String,
}

impl App for InputDemo {
    type Msg = Msg;
    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Add => {
                self.count += 1;
                self.last = "Activated Add".into();
            }
            Msg::Reset => {
                self.count = 0;
                self.last = "Activated Reset".into();
            }
            Msg::Key(key) => self.last = format!("Key code: {key}"),
            Msg::Scroll(offset) => self.last = format!("Scroll: {}, {} px", offset.x, offset.y),
        }
        println!("count={} {}", self.count, self.last);
    }
    fn view(&self, theme: &Theme) -> View<Msg> {
        let focus = Style::default()
            .outline_width(2)
            .outline_pad(3)
            .outline_color(theme.accent)
            .outline_opacity(255);
        column([
            label("Touch + keys").style(Style::title(theme)),
            label("Click Add, or use Tab and Enter. Drag the list to scroll.")
                .style(Style::body(theme)),
            label(format!("Count: {}", self.count)).style(Style::title(theme)),
            row([])
                // Reserve the outline width, gap and one antialiasing pixel.
                .style(Style::default().pad(6).gap(12))
                .child(
                    button("Add +1")
                        .key(Key::Named("add"))
                        .style(Style::default().shadow_width(0).outline_width(0))
                        .state_style(State::FOCUSED, focus)
                        .state_style(State::FOCUS_KEY, focus)
                        .on_click(Msg::Add)
                        .on_key(Msg::Key),
                )
                .child(
                    button("Reset")
                        .key(Key::Named("reset"))
                        .style(Style::default().shadow_width(0).outline_width(0))
                        .state_style(State::FOCUSED, focus)
                        .state_style(State::FOCUS_KEY, focus)
                        .on_click(Msg::Reset)
                        .on_key(Msg::Key),
                ),
            column(
                (1..=8).map(|n| label(format!("Scrollable item {n}")).style(Style::body(theme))),
            )
            .style(
                Style::surface(theme)
                    .width(Size::Percent(100))
                    .height(Size::Px(80))
                    .pad(8)
                    .gap(8)
                    .outline_width(0),
            )
            .state_style(State::FOCUSED, focus)
            .state_style(State::FOCUS_KEY, focus)
            .scroll_dir(ScrollDir::VER)
            .focusable(true)
            .scrollbar_mode(ScrollbarMode::Auto)
            .scroll_momentum(true)
            .scroll_elastic(true)
            .on_scroll(Msg::Scroll),
            label(&self.last).style(Style::body(theme)),
        ])
        .style(Style::screen(theme).pad(20).gap(12))
    }
}

fn main() {
    let frames: Option<u64> = std::env::var("INPUT_FRAMES")
        .ok()
        .and_then(|value| value.parse().ok());
    // SAFETY: all native calls and UI work stay on this thread. The platform
    // creates devices and tears them down after dropping the mounted UI.
    let (mouse, keyboard) = unsafe {
        rs_lvgl::init();
        let display =
            NonNull::new(rs_lvgl_sys::lv_sdl_window_create(640, 420)).expect("SDL display");
        rs_lvgl_sys::lv_sdl_window_set_title(display.as_ptr(), c"LVGL input".as_ptr());
        let mouse = NonNull::new(rs_lvgl_sys::lv_sdl_mouse_create()).expect("SDL mouse");
        let keyboard = NonNull::new(rs_lvgl_sys::lv_sdl_keyboard_create()).expect("SDL keyboard");
        (mouse, keyboard)
    };
    let display = rs_lvgl::Display::default();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = InputDemo {
        count: 0,
        last: "Ready".into(),
    };
    ui.mount(&app).unwrap();
    // SAFETY: SDL returned a live keypad belonging to this display.
    unsafe { ui.bind_input(keyboard).unwrap() };
    ui.focus(Key::Named("add")).unwrap();
    let mut last = Instant::now();
    let mut frame = 0;
    loop {
        let now = Instant::now();
        rs_lvgl::tick_inc(now.duration_since(last));
        last = now;
        ui.timer_handler();
        // The native SDL close handler deletes the display. Do not access
        // the UI's stale tree after that unsafe platform teardown.
        if unsafe { rs_lvgl_sys::lv_display_get_default() }.is_null() {
            core::mem::forget(ui);
            unsafe {
                rs_lvgl_sys::lv_indev_delete(mouse.as_ptr());
                rs_lvgl_sys::lv_indev_delete(keyboard.as_ptr());
            }
            return;
        }
        ui.drain(&mut app, 64).unwrap();
        frame += 1;
        if frames.is_some_and(|limit| frame >= limit) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(ui);
    unsafe {
        rs_lvgl_sys::lv_indev_delete(mouse.as_ptr());
        rs_lvgl_sys::lv_indev_delete(keyboard.as_ptr());
        display.delete();
    }
}

#[cfg(test)]
#[path = "support/input_audit.rs"]
mod audit;
