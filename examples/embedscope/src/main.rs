//! A real LVGL memory display, optionally inspectable over loopback TCP.
use rs_lvgl_ui::prelude::*;
use std::time::{Duration, Instant};

#[cfg(feature = "embedscope")]
mod diagnostics;

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;

#[derive(Clone)]
enum SettingsAction {
    Increment,
    Brightness(i32),
}

struct Settings {
    count: u32,
    brightness: i32,
    preview: i32,
    #[cfg(feature = "embedscope")]
    case: BrightnessCase,
    #[cfg(feature = "embedscope")]
    decision: Option<diagnostics::Decision>,
    #[cfg(feature = "embedscope")]
    cancelled_brightness: bool,
}

fn preview_rgb(brightness: i32) -> [u8; 3] {
    [32, (brightness * 3 + 24) as u8, 96]
}

impl App for Settings {
    type Action = SettingsAction;

    fn update(&mut self, action: SettingsAction) {
        match action {
            SettingsAction::Increment => self.count += 1,
            SettingsAction::Brightness(value) => {
                // LVGL can emit VALUE_CHANGED while handling PRESS_LOST. The
                // cancelled tap still reconciles the controlled native value.
                #[cfg(feature = "embedscope")]
                if self.cancelled_brightness {
                    return;
                }
                if !(0..=65).contains(&value) {
                    #[cfg(feature = "embedscope")]
                    {
                        self.decision = Some(diagnostics::Decision::Rejected);
                    }
                } else if self.brightness == value {
                    #[cfg(feature = "embedscope")]
                    {
                        self.decision
                            .get_or_insert(diagnostics::Decision::Unchanged);
                    }
                } else {
                    self.brightness = value;
                    #[cfg(feature = "embedscope")]
                    {
                        self.decision = Some(diagnostics::Decision::Accepted(preview_rgb(value)));
                    }
                    #[cfg(feature = "embedscope")]
                    if self.case == BrightnessCase::HoldPreview {
                        return;
                    }
                    self.preview = value;
                }
            }
        }
    }

    fn view(&self, theme: &Theme) -> View<SettingsAction> {
        let [r, g, b] = preview_rgb(self.preview);
        column([
            label("Display settings"),
            label(format!("Count: {}", self.count)),
            button("Add one")
                .key(Key::Named("increment"))
                .style(Style::accent(theme).width(px(224)).height(px(56)))
                .on_click(SettingsAction::Increment),
            label("Brightness"),
            slider(0, 100, self.brightness)
                .key(Key::Named("brightness"))
                .width(px(224))
                .height(px(40))
                .on_value_changed(SettingsAction::Brightness),
            container()
                .key(Key::Named("preview"))
                .width(px(224))
                .height(px(16))
                .background(Color::rgb(r, g, b)),
        ])
        .style(Style::surface(theme))
        .width(percent(100))
        .height(percent(100))
        .padding(16)
        .gap(8)
        .scrollable(false)
    }
}

#[cfg(feature = "embedscope")]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum BrightnessCase {
    #[default]
    Normal,
    IgnoreInput,
    HoldPreview,
}

struct Options {
    run_for: Option<Duration>,
    exit_on_stdin_close: bool,
    #[cfg(feature = "embedscope")]
    listen: std::net::SocketAddr,
    #[cfg(feature = "embedscope")]
    case: BrightnessCase,
}

impl Options {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut result = Self {
            run_for: None,
            exit_on_stdin_close: false,
            #[cfg(feature = "embedscope")]
            listen: "127.0.0.1:9338".parse()?,
            #[cfg(feature = "embedscope")]
            case: BrightnessCase::Normal,
        };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--exit-on-stdin-close" => result.exit_on_stdin_close = true,
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
                #[cfg(feature = "embedscope")]
                "--brightness-case" => {
                    result.case = match args.next().as_deref() {
                        Some("normal") => BrightnessCase::Normal,
                        Some("ignore-input") => BrightnessCase::IgnoreInput,
                        Some("hold-preview") => BrightnessCase::HoldPreview,
                        _ => return Err("expected normal, ignore-input, or hold-preview".into()),
                    };
                }
                _ => return Err(format!("unknown option: {arg}").into()),
            }
        }
        Ok(result)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::parse()?;
    let mut stdin_reader = if options.exit_on_stdin_close {
        Some(std::thread::Builder::new().spawn(|| {
            std::io::copy(&mut std::io::stdin().lock(), &mut std::io::sink()).map(|_| ())
        })?)
    } else {
        None
    };
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
    let mut app = Settings {
        count: 0,
        brightness: 30,
        preview: 30,
        #[cfg(feature = "embedscope")]
        case: options.case,
        #[cfg(feature = "embedscope")]
        decision: None,
        #[cfg(feature = "embedscope")]
        cancelled_brightness: false,
    };
    let mut ui = Ui::new(&display, Theme::dark()).expect("UI");
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        ui.mount(&app)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        #[cfg(feature = "embedscope")]
        {
            // SAFETY: this is the sole display, on its UI thread, and the
            // RGB888 flush is synchronous. Publish a complete first capture
            // before opening the listener, independent of timer phase.
            unsafe { rs_lvgl::sys::lv_refr_now(std::ptr::null_mut()) };
            pixels.borrow_mut().finish();
        }
        // Declared after the UI: transport/input/tree are dropped before unmount.
        #[cfg(feature = "embedscope")]
        let mut inspector =
            diagnostics::Inspector::new(options.listen, &display, pixels, options.case)?;
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
            {
                inspector.poll()?;
                app.cancelled_brightness = inspector.take_cancelled_brightness();
            }
            let _stats = ui
                .drain(&mut app, 32)
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            ui.timer_handler();
            #[cfg(feature = "embedscope")]
            inspector.rendered(_stats.reconciled, app.count, app.decision.take())?;
            if let Some(reader) = stdin_reader.take_if(|reader| reader.is_finished()) {
                reader
                    .join()
                    .map_err(|_| std::io::Error::other("stdin reader panicked"))??;
                break;
            }
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
