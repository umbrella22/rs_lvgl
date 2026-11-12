//! UI naming, units, layout and widget parts using rs_lvgl_ui 0.8.0.
//! Features: alloc, widget-slider, widget-switch, widget-bar.
use rs_lvgl_ui::prelude::*;

#[derive(Clone)]
enum FanAction {
    Speed(i32),
    Enabled(bool),
}
struct Fan {
    speed: i32,
    enabled: bool,
}
impl App for Fan {
    type Action = FanAction;
    fn update(&mut self, action: FanAction) {
        match action {
            FanAction::Speed(speed) => self.speed = speed,
            FanAction::Enabled(enabled) => self.enabled = enabled,
        }
    }
    fn view(&self, theme: &Theme) -> View<FanAction> {
        typed::column([])
            .child(
                typed::label("Fan")
                    .text_overflow(TextOverflow::Ellipsis)
                    .font(theme.title_font)
                    .text_color(theme.text),
            )
            .child(
                typed::row([])
                    .child(
                        typed::slider(0, 100, binding(self.speed, FanAction::Speed))
                            .height(px(16))
                            .grow(1)
                            .thumb_style(style().background(theme.accent))
                            .progress_style(style().background(theme.accent)),
                    )
                    .child(typed::switch(binding(self.enabled, FanAction::Enabled)))
                    .width(percent(100))
                    .gap(16)
                    .justify_content(FlexAlign::SpaceBetween)
                    .align_items(ItemAlign::Center)
                    .align_content(FlexAlign::Center)
                    .padding(16),
            )
            .child(typed::progress_bar(0, 100, self.speed))
            .child(
                typed::container().width(px(64)).height(px(64)).child(
                    typed::container()
                        .width(px(24))
                        .height(px(24))
                        .align(Align::Center)
                        .background(theme.accent)
                        .transform_scale_x(Scale::percent(150))
                        .transform_scale_y(Scale::percent(150))
                        .transform_pivot_x(12)
                        .transform_pivot_y(12)
                        .transform_rotation(Angle::degrees(45)),
                ),
            )
            .scroll_direction(ScrollDirection::VERTICAL)
            .width(percent(100))
            .height(percent(100))
            .background(theme.background)
            .text_color(theme.text)
            .padding(16)
            .gap(12)
            .into_view()
    }
}

fn main() {
    rs_lvgl::init();
    let display = rs_lvgl::Display::register::<_, { 320 * 240 }>(320, 240, |_| {}).unwrap();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut fan = Fan {
        speed: 50,
        enabled: true,
    };
    ui.mount(&fan).unwrap();
    ui.post(FanAction::Speed(75)).unwrap();
    ui.post(FanAction::Enabled(false)).unwrap();
    assert_eq!(ui.drain(&mut fan, 8).unwrap().processed, 2);
    assert_eq!(fan.speed, 75);
    assert!(!fan.enabled);
    ui.unmount();
    drop(ui);
    // SAFETY: no retained objects remain and the memory display flushes synchronously.
    unsafe {
        display.delete();
        rs_lvgl::deinit();
    }
}
