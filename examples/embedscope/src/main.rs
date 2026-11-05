//! A real LVGL memory display, optionally inspectable over loopback TCP.
use rs_lvgl_ui::prelude::*;
use std::time::{Duration, Instant};

#[cfg(feature = "embedscope")]
mod diagnostics;

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;

#[derive(Clone)]
enum CounterAction {
    Increment,
}

#[derive(Default)]
struct Counter(u32);

impl App for Counter {
    // This independent example uses published 0.3.1, whose App still names its
    // associated type Msg. The unreleased workspace API uses Action instead.
    type Msg = CounterAction;

    fn update(&mut self, _: CounterAction) {
        self.0 += 1;
    }

    fn view(&self, theme: &Theme) -> View<CounterAction> {
        column([
            label("rs_lvgl + EmbedScope"),
            label(format!("Count: {}", self.0)),
            button("Add one")
                .key(Key::Named("increment"))
                .style(
                    Style::accent(theme)
                        .width(Size::Px(224))
                        .height(Size::Px(56)),
                )
                .on_click(CounterAction::Increment),
        ])
        .style(
            Style::surface(theme)
                .width(Size::Px(WIDTH as i16))
                .height(Size::Px(HEIGHT as i16))
                .pad(24)
                .gap(16),
        )
    }
}

struct Options {
    run_for: Option<Duration>,
    #[cfg(feature = "embedscope")]
    listen: std::net::SocketAddr,
}

impl Options {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut result = Self {
            run_for: None,
            #[cfg(feature = "embedscope")]
            listen: "127.0.0.1:9338".parse()?,
        };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--run-for-ms" => {
                    let ms: u64 = args.next().ok_or("missing duration")?.parse()?;
                    result.run_for = Some(Duration::from_millis(ms));
                }
                #[cfg(feature = "embedscope")]
                "--listen" => {
                    result.listen = args.next().ok_or("missing listen address")?.parse()?;
                    if !result.listen.ip().is_loopback() {
                        return Err("the example accepts loopback addresses only".into());
                    }
                }
                _ => return Err(format!("unknown option: {arg}").into()),
            }
        }
        Ok(result)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::parse()?;
    rs_lvgl::init();
    #[cfg(feature = "embedscope")]
    let pixels = std::rc::Rc::new(std::cell::RefCell::new(diagnostics::Frame::new()));
    #[cfg(feature = "embedscope")]
    let output = pixels.clone();
    let display =
        rs_lvgl::Display::register::<_, { WIDTH as usize * 16 }>(WIDTH, HEIGHT, move |_refresh| {
            #[cfg(feature = "embedscope")]
            output.borrow_mut().flush(_refresh);
        })
        .expect("memory display");
    let mut app = Counter::default();
    let mut ui = Ui::new(&display, Theme::dark()).expect("UI");
    ui.mount(&app).expect("mount");
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        // Declared after the UI: transport/input/tree are dropped before unmount.
        #[cfg(feature = "embedscope")]
        let mut inspector = diagnostics::Inspector::new(options.listen, &display, pixels)?;
        #[cfg(feature = "embedscope")]
        ui.set_diagnostic_sink(Some(inspector.sink()));
        #[cfg(not(feature = "embedscope"))]
        println!("EmbedScope disabled; native memory display running");
        let started = Instant::now();
        let mut last_tick = Instant::now();
        loop {
            let now = Instant::now();
            rs_lvgl::tick_inc(now.duration_since(last_tick));
            last_tick = now;
            #[cfg(feature = "embedscope")]
            inspector.poll()?;
            ui.timer_handler();
            let stats = ui.drain(&mut app, 32).expect("drain");
            #[cfg(feature = "embedscope")]
            inspector.rendered(stats.processed != 0, app.0)?;
            #[cfg(not(feature = "embedscope"))]
            let _ = stats;
            if options
                .run_for
                .is_some_and(|limit| started.elapsed() >= limit)
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    })();
    #[cfg(feature = "embedscope")]
    ui.set_diagnostic_sink(None);
    ui.unmount();
    drop(ui);
    // SAFETY: inspector/input/tree and UI are gone; the memory-display flush
    // completes synchronously, and this application owns the sole LVGL instance.
    unsafe {
        display.delete();
        rs_lvgl::deinit();
    }
    result
}
