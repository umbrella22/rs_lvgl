//! Memory-display example for rs_lvgl 0.2.
//! Dependencies: rs_lvgl 0.2 (alloc), rs_lvgl_ui 0.2 (alloc).
//! No SDL or optional widget is needed.
use rs_lvgl_ui::prelude::*;

#[derive(Clone)]
enum Msg {
    Increment,
}

struct Counter {
    value: u32,
}

impl App for Counter {
    type Msg = Msg;

    fn update(&mut self, _: Msg) {
        self.value += 1;
    }

    fn view(&self, theme: &Theme) -> View<Msg> {
        column([
            label(format!("Count: {}", self.value)),
            button("Add one")
                .key(Key::Named("increment"))
                .style(Style::accent(theme))
                .on_click(Msg::Increment),
        ])
        .style(Style::default().gap(12))
    }
}

fn main() {
    rs_lvgl::init();
    let display =
        rs_lvgl::Display::register::<_, { 160 * 120 }>(160, 120, |_| {}).expect("memory display");
    let mut app = Counter { value: 0 };
    let mut ui = Ui::new(&display, Theme::dark()).expect("UI");
    ui.mount(&app).expect("mount");
    // Real inputs enqueue messages; an external state change requests a render.
    app.update(Msg::Increment);
    ui.update(&app).expect("render changed state");
    rs_lvgl::tick_inc(core::time::Duration::from_millis(20));
    ui.timer_handler();
    ui.tick(20).expect("advance custom widgets");
    ui.drain(&mut app, 32).expect("drain messages");
    assert_eq!(app.value, 1);
    ui.unmount();
    drop(ui);
    // SAFETY: no UI, input, alias or pending transfer remains. The memory-display
    // callback completes synchronously; this process has no other LVGL users.
    unsafe {
        display.delete();
        rs_lvgl::deinit();
    }
}
