//! Run: `cargo run -p rs_lvgl_ui --example rust_timer --features simulator,rust_timer`
use rs_lvgl::timer::{update_clock, LvClock};
use rs_lvgl_ui::prelude::*;
use std::time::Duration;
mod support;

#[derive(Clone)]
enum Msg {
    Toggle,
}
#[derive(Default)]
struct Clock {
    milliseconds: u32,
    tick_ms: u32,
    paused: bool,
}
impl LvClock for Clock {
    fn since_init(&self) -> Duration {
        Duration::from_millis(self.tick_ms as u64)
    }
}
impl App for Clock {
    type Msg = Msg;
    fn update(&mut self, _: Msg) {
        self.paused = !self.paused;
    }
    fn view(&self, theme: &Theme) -> View<Msg> {
        support::frame(
            theme,
            "Rust clock",
            "A Rust monotonic clock drives LVGL and the displayed timer.",
            support::column()
                .child(support::value(
                    theme,
                    format!("{:.1} seconds", self.milliseconds as f64 / 1000.0),
                ))
                .child(
                    bar(0, 10000, (self.milliseconds % 10001) as i32).style(
                        Style::default()
                            .width(Size::Percent(100))
                            .height(Size::Px(24)),
                    ),
                )
                .child(support::action(
                    theme,
                    if self.paused { "Resume" } else { "Pause" },
                    Msg::Toggle,
                )),
        )
    }
}
impl support::Example for Clock {
    fn advance(&mut self, milliseconds: u32) -> bool {
        self.tick_ms = self
            .tick_ms
            .checked_add(milliseconds)
            .expect("native clock fits u32");
        if self.paused {
            return false;
        }
        self.milliseconds = self
            .milliseconds
            .checked_add(milliseconds)
            .expect("clock duration fits u32");
        true
    }
    fn tick(&self, _: Duration) {
        update_clock(self).expect("clock fits native tick");
    }
}
fn main() {
    support::run("rust_timer", Clock::default());
}
#[cfg(test)]
mod tests {
    use super::*;
    use support::Example;
    #[test]
    fn rust_clock_drives_native_tick_and_pause() {
        support::audit::check("rust_timer", Clock::default, |ui, app| {
            app.advance(1250);
            app.tick(Duration::ZERO);
            assert_eq!(unsafe { rs_lvgl_sys::lv_tick_get() }, 1250);
            ui.update(app).unwrap();
            support::audit::click(ui, app, "Pause");
            assert!(app.paused);
            app.advance(1000);
            assert_eq!(app.milliseconds, 1250);
            app.tick(Duration::ZERO);
            assert_eq!(unsafe { rs_lvgl_sys::lv_tick_get() }, 2250);
        });
    }
}
