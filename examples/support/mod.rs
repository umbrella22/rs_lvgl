//! Shared desktop host for the focused component examples.
#![allow(dead_code)]

use rs_lvgl_ui::prelude::*;
use std::{
    ffi::CString,
    path::Path,
    thread,
    time::{Duration, Instant},
};

#[cfg(test)]
pub mod audit;
#[path = "../../rs_lvgl_ui/examples/support/runtime.rs"]
mod runtime;

pub const WIDTH: u32 = 640;
pub const HEIGHT: u32 = 480;

pub trait Example: App {
    fn advance(&mut self, _milliseconds: u32) -> bool {
        false
    }
    fn tick(&self, elapsed: Duration) {
        rs_lvgl::tick_inc(elapsed);
    }
}

pub fn frame<M: Clone + 'static>(
    theme: &Theme,
    title: &str,
    subtitle: &str,
    content: View<M>,
) -> View<M> {
    rs_lvgl_ui::column([
        label("rs_lvgl / examples").style(Style::muted(theme)),
        label(title).style(Style::title(theme)),
        label(subtitle).style(Style::muted(theme)),
        rs_lvgl_ui::column([content]).style(
            Style::surface(theme)
                .pad(24)
                .gap(18)
                .width(Size::Percent(100))
                .flex_grow(1),
        ),
    ])
    .style(Style::screen(theme).pad(24).gap(12))
}

pub fn column<M>() -> View<M> {
    rs_lvgl_ui::column([]).style(Style::default().gap(16).width(Size::Percent(100)))
}
pub fn row<M>() -> View<M> {
    rs_lvgl_ui::row([]).style(
        Style::default()
            .gap(12)
            .width(Size::Percent(100))
            .flex_align(FlexAlign::Start, FlexAlign::Center),
    )
}
pub fn value<M>(theme: &Theme, text: impl Into<String>) -> View<M> {
    label(text).style(Style::title(theme))
}
pub fn action<M: Clone + 'static>(theme: &Theme, text: &str, message: M) -> View<M> {
    button(text)
        .style(
            Style::accent(theme)
                .pad_left(16)
                .pad_right(16)
                .height(Size::Px(44)),
        )
        .state_style(
            State::PRESSED,
            Style::default().bg(theme.text).fg(theme.surface),
        )
        .on_click(message)
}

pub fn save(image: &Snapshot, directory: &Path, name: &str) {
    std::fs::create_dir_all(directory).expect("capture directory");
    let mut bytes = format!("P6\n{} {}\n255\n", image.width, image.height).into_bytes();
    for pixel in image.pixels.chunks_exact(4) {
        bytes.extend_from_slice(&pixel[..3]);
    }
    std::fs::write(directory.join(format!("{name}.ppm")), bytes).expect("capture pixels");
}

pub fn run<A: Example>(name: &str, mut app: A) {
    let limit = std::env::var("EXAMPLE_FRAMES").ok().map(|n| {
        n.parse::<u32>()
            .expect("EXAMPLE_FRAMES is an unsigned frame count")
    });
    let captures = std::env::var_os("EXAMPLE_CAPTURE_DIR").map(std::path::PathBuf::from);
    let dark = std::env::var("EXAMPLE_DARK").as_deref() == Ok("1");
    rs_lvgl::init();
    // SAFETY: this process owns LVGL on its main thread. The native backend
    // retains its display and mouse until the normal teardown below.
    let mouse = unsafe {
        let raw = rs_lvgl_sys::lv_sdl_window_create(WIDTH as i32, HEIGHT as i32);
        assert!(!raw.is_null(), "SDL display creation failed");
        let title = CString::new(format!("rs_lvgl / {name}")).unwrap();
        rs_lvgl_sys::lv_sdl_window_set_title(raw, title.as_ptr());
        let mouse = rs_lvgl_sys::lv_sdl_mouse_create();
        assert!(!mouse.is_null(), "SDL pointer creation failed");
        mouse
    };
    let display = rs_lvgl::Display::default();
    let mut ui = Ui::new(&display, if dark { Theme::dark() } else { Theme::light() }).expect("UI");
    ui.mount(&app).expect("mount");
    let mut previous = Instant::now();
    let mut frames = 0;
    loop {
        let now = Instant::now();
        let elapsed = now.duration_since(previous);
        previous = now;
        let milliseconds = u32::try_from(elapsed.as_millis()).expect("frame duration fits u32");
        let changed = app.advance(milliseconds);
        app.tick(elapsed);
        if changed {
            ui.update(&app).expect("update");
        }
        if runtime::frame(&mut ui, &mut app, 64)
            .expect("dispatch")
            .is_none()
        {
            // SDL deleted the display during event dispatch. No stale UI or
            // mouse pointer may be accessed; the executable exits here.
            std::mem::forget(ui);
            return;
        }
        ui.tick(milliseconds).expect("custom widget tick");
        if frames == 0 {
            if let Some(dir) = &captures {
                save(
                    &ui.snapshot().expect("snapshot"),
                    dir,
                    &format!("sdl-{name}-initial"),
                );
            }
        }
        frames += 1;
        if limit.is_some_and(|limit| frames >= limit) {
            break;
        }
        thread::sleep(Duration::from_millis(8));
    }
    if let Some(dir) = &captures {
        save(
            &ui.snapshot().expect("snapshot"),
            dir,
            &format!("sdl-{name}-final"),
        );
    }
    println!(
        "{name}: {frames} frames, {} nodes",
        ui.stats().mounted_nodes
    );
    // SAFETY: the event loop stopped with its display alive. Remove the input
    // before unmount/deinit, and finish all synchronous rendering first.
    unsafe {
        rs_lvgl_sys::lv_indev_delete(mouse);
    }
    ui.unmount();
    drop(ui);
    unsafe {
        rs_lvgl::deinit();
    }
}
