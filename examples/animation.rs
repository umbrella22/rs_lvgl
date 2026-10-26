//! Run: `cargo run -p rs_lvgl_ui --example animation --features simulator`
use rs_lvgl_ui::prelude::*;
mod support;

#[derive(Clone)]
enum Msg {
    Toggle,
}
struct Motion {
    running: bool,
}
impl Default for Motion {
    fn default() -> Self {
        Self { running: true }
    }
}
impl App for Motion {
    type Msg = Msg;
    fn update(&mut self, _: Msg) {
        self.running = !self.running;
    }
    fn view(&self, theme: &Theme) -> View<Msg> {
        let mut dot = obj().style(
            Style::default()
                .bg(theme.accent)
                .radius(14)
                .width(Size::Px(28))
                .height(Size::Px(28))
                .align(Align::LeftMid)
                .floating(true),
        );
        if self.running {
            dot = dot.animate(
                Animation::new(AnimationProperty::X, 0, 400, 1200)
                    .easing(Easing::EaseInOut)
                    .reverse(1200)
                    .repeat(u32::MAX),
            );
        }
        support::frame(
            theme,
            "Native animation",
            "LVGL animates a mounted node; messages control playback.",
            support::column()
                .child(
                    obj()
                        .style(
                            Style::default()
                                .width(Size::Percent(100))
                                .height(Size::Px(100)),
                        )
                        .child(dot),
                )
                .child(support::action(
                    theme,
                    if self.running { "Pause" } else { "Resume" },
                    Msg::Toggle,
                )),
        )
    }
}
impl support::Example for Motion {}
fn main() {
    support::run("animation", Motion::default());
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_motion_and_pause() {
        support::audit::check("animation", Motion::default, |ui, app| {
            let dot = support::audit::native::find(support::audit::native::root(), &|obj| unsafe {
                rs_lvgl_sys::lv_obj_get_width(obj) == 28
                    && rs_lvgl_sys::lv_obj_get_height(obj) == 28
            })
            .unwrap();
            let before = support::audit::native::rect(dot)[0];
            support::audit::native::settle(ui);
            assert_ne!(before, support::audit::native::rect(dot)[0]);
            support::audit::click(ui, app, "Pause");
            assert!(!app.running);
        });
    }
}
