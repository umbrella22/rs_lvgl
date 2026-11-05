//! rs_lvgl component studio: real native widgets, controlled application state.
//! Run with `--features showcase`. STUDIO_FRAMES bounds a native SDL run;
//! STUDIO_AUTORUN walks every category, STUDIO_CAPTURE_DIR exports real renders.
use core::ptr::NonNull;
use rs_lvgl_ui::components::{
    dialog::Dialog,
    nav::NavState,
    toast::{self, ToastKind, Toasts},
};
use rs_lvgl_ui::prelude::*;
#[cfg(not(target_os = "emscripten"))]
use std::{
    path::Path,
    time::{Duration, Instant},
};

#[path = "studio/native.rs"]
mod native;
#[path = "studio/pages.rs"]
mod pages;
#[path = "support/runtime.rs"]
#[cfg(not(target_os = "emscripten"))]
mod runtime;

#[cfg(target_os = "emscripten")]
#[path = "studio/browser.rs"]
pub mod browser;

const WIDTH: i32 = 1280;
const HEIGHT: i32 = 860;
const INK: Color = Color::rgb(25, 31, 44);
const ORANGE: Color = Color::rgb(225, 88, 47);
const TEAL: Color = Color::rgb(26, 145, 128);
const BLUE: Color = Color::rgb(91, 117, 218);
const WHITE: Color = Color::rgb(255, 255, 255);
const PAGES: [(&str, &str, &str); 9] = [
    (
        "Overview",
        "Make it yours.",
        "An embedded UI toolkit, made tangible. Explore every component in one place.",
    ),
    (
        "Controls",
        "A little more control.",
        "Real controls, shared state. Change a value and watch the preview respond.",
    ),
    (
        "Text & input",
        "Every input matters.",
        "Text fields, native keyboards, Chinese input and date selection.",
    ),
    (
        "Data & charts",
        "Give your data a shape.",
        "Sample data rendered with native charts, tables, scales and lines.",
    ),
    (
        "Layout",
        "Room to build.",
        "Tabs, swipeable tiles and flexible grids, composed from ordinary nodes.",
    ),
    (
        "Media & codes",
        "Small details. Big character.",
        "Images, animated media and machine-readable codes in a single view.",
    ),
    (
        "Canvas & motion",
        "Draw outside the box.",
        "Owned drawing buffers, curved type and scoped native animation.",
    ),
    (
        "Composition",
        "Put it all together.",
        "Lists, panels, navigation, files, dialogs and notifications.",
    ),
    (
        "Native extras",
        "A door to the native engine.",
        "List, menu and window samples through the raw native extension point.",
    ),
];

#[derive(Clone, Debug)]
enum StudioAction {
    Page(usize),
    Theme,
    Value(i32),
    Enabled(bool),
    Checked(bool),
    Select(i32),
    Text(String),
    Chinese(String),
    Date(i32),
    Tab(i32),
    Tile(i32),
    Step(i32),
    Click,
    Dialog(bool),
    Toast,
    Dismiss(u64),
    File(u64),
    Push,
    Back,
    Reset,
    Quit,
}

struct Studio {
    page: usize,
    dark: bool,
    value: i32,
    enabled: bool,
    checked: bool,
    selected: u32,
    text: String,
    chinese: String,
    date: String,
    tab: u32,
    tile: u32,
    clicks: u32,
    dialog: bool,
    toasts: Toasts,
    nav: NavState<&'static str>,
    file: u64,
    status: String,
    quit: bool,
}
impl Default for Studio {
    fn default() -> Self {
        Self {
            page: 0,
            dark: false,
            value: 64,
            enabled: true,
            checked: true,
            selected: 1,
            text: "Hello, Rust!".into(),
            chinese: "你好".into(),
            date: "22 October 2026".into(),
            tab: 0,
            tile: 0,
            clicks: 0,
            dialog: false,
            toasts: Toasts::new(),
            nav: NavState::new("Workspace"),
            file: 0,
            status: "Ready to explore".into(),
            quit: false,
        }
    }
}
impl App for Studio {
    type Action = StudioAction;
    fn update(&mut self, action: StudioAction) {
        match action {
            StudioAction::Page(page) => {
                self.page = page;
                self.dialog = false;
                self.status = format!("{} opened", PAGES[page].0);
            }
            StudioAction::Theme => self.dark = !self.dark,
            StudioAction::Value(value) => {
                self.value = value;
                self.status = format!("Output set to {value}%");
            }
            StudioAction::Enabled(value) => self.enabled = value,
            StudioAction::Checked(value) => self.checked = value,
            StudioAction::Select(value) => self.selected = value as u32,
            StudioAction::Text(text) => self.text = text,
            StudioAction::Chinese(text) => self.chinese = text,
            StudioAction::Date(date) => {
                if let Some(d) = CalendarDate::from_value(date) {
                    self.date = format!("{} / {} / {}", d.year, d.month, d.day);
                }
            }
            StudioAction::Tab(tab) => self.tab = tab as u32,
            StudioAction::Tile(tile) => {
                if tile >= 0 {
                    self.tile = tile as u32;
                }
            }
            StudioAction::Step(step) => self.value = (self.value + step).clamp(0, 100),
            StudioAction::Click => {
                self.clicks += 1;
                self.status = format!("Action received / {}", self.clicks);
            }
            StudioAction::Dialog(open) => self.dialog = open,
            StudioAction::Toast => {
                self.toasts
                    .push_kind(ToastKind::Success, "Your workspace is ready.", 3200);
            }
            StudioAction::Dismiss(id) => self.toasts.dismiss(id),
            StudioAction::File(id) => {
                self.file = id;
                self.status = format!("Selected sample file {id}");
            }
            StudioAction::Push => self.nav.push("Device details"),
            StudioAction::Back => {
                self.nav.back();
            }
            StudioAction::Reset => {
                let page = self.page;
                let dark = self.dark;
                *self = Self::default();
                self.page = page;
                self.dark = dark;
            }
            StudioAction::Quit => self.quit = true,
        }
    }
    fn view(&self, theme: &Theme) -> View<StudioAction> {
        let sidebar = studio_column()
            .key(Key::Named("sidebar"))
            .background(INK)
            .text_color(WHITE)
            .width(px(224))
            .height(percent(100))
            .padding(22)
            .gap(8)
            .child(styled_label("rs_lvgl", WHITE, Font::montserrat_28()).padding_top(8))
            .child(
                styled_label(
                    "COMPONENT STUDIO",
                    Color::rgb(146, 156, 176),
                    Font::montserrat_12(),
                )
                .text_letter_space(2)
                .padding_bottom(28),
            )
            .children(PAGES.iter().enumerate().map(|(i, (name, _, _))| {
                action_button(
                    &format!("{:02}   {name}", i + 1),
                    StudioAction::Page(i),
                    theme,
                )
                .key(Key::Named(name))
                .width(percent(100))
                .height(px(46))
                .padding_left(12)
                .background(if self.page == i { ORANGE } else { INK })
                .text_color(if self.page == i {
                    WHITE
                } else {
                    Color::rgb(187, 196, 212)
                })
                .text_align(TextAlign::Left)
            }))
            .child(obj().style(Style::neutral()).grow(1).clickable(false))
            .child(styled_label(
                "BUILT WITH RUST",
                Color::rgb(146, 156, 176),
                Font::montserrat_12(),
            ))
            .child(styled_label(
                "Powered by LVGL 9.6",
                Color::rgb(187, 196, 212),
                Font::montserrat_12(),
            ))
            .child(
                action_button("Exit studio", StudioAction::Quit, theme)
                    .background(Color::rgb(42, 49, 65))
                    .text_color(WHITE)
                    .width(percent(100)),
            );

        let main = studio_column()
            .width(px(WIDTH as i16 - 224))
            .height(percent(100))
            .padding(28)
            .gap(18)
            .child(
                studio_row()
                    .height(px(54))
                    .flex_align(FlexAlign::SpaceBetween, FlexAlign::Center)
                    .child(
                        styled_label("RS / COMPONENT LIBRARY", theme.muted, Font::montserrat_12())
                            .text_letter_space(2),
                    )
                    .child(studio_row().width(content()).padding(6).children([
                        action_button("Reset", StudioAction::Reset, theme),
                        action_button(
                            if self.dark { "Light mode" } else { "Dark mode" },
                            StudioAction::Theme,
                            theme,
                        ),
                    ])),
            )
            .child(
                studio_column()
                    .gap(8)
                    .height(px(70))
                    .child(styled_label(
                        PAGES[self.page].1,
                        theme.text,
                        Font::montserrat_32(),
                    ))
                    .child(text(PAGES[self.page].2, theme)),
            )
            .child(
                pages::view(self, theme)
                    .key(Key::Id(1000 + self.page as u64))
                    .width(percent(100))
                    .grow(1)
                    .scrollable(true)
                    .scroll_dir(ScrollDir::VER)
                    .scrollbar_mode(ScrollbarMode::Auto),
            )
            .child(
                studio_row()
                    .height(px(24))
                    .flex_align(FlexAlign::SpaceBetween, FlexAlign::Center)
                    .child(text(&self.status, theme))
                    .child(text("LOCAL DEMO  /  SAMPLE DATA", theme)),
            );
        studio_row()
            .style(Style::screen(theme))
            .padding(0)
            .gap(0)
            .text_color(theme.text)
            .font(theme.body_font)
            .child(sidebar)
            .child(main)
    }
    fn overlay(&self, theme: &Theme) -> Option<View<StudioAction>> {
        let mut layers = vec![toast::view(&self.toasts, theme, StudioAction::Dismiss)];
        if self.dialog {
            layers.push(
                Dialog::new("Ready to build?")
                    .text("This dialog is a real modal: focus stays inside until it closes.")
                    .button("Keep exploring", StudioAction::Dialog(false))
                    .primary("Let's go", StudioAction::Dialog(false))
                    .on_dismiss(StudioAction::Dialog(false))
                    .view(theme),
            );
        }
        Some(components::layers(layers))
    }
}

fn palette(dark: bool) -> Theme {
    let mut t = if dark { Theme::dark() } else { Theme::light() };
    t.background = if dark {
        Color::rgb(17, 22, 31)
    } else {
        Color::rgb(246, 245, 241)
    };
    t.surface = if dark { Color::rgb(30, 37, 49) } else { WHITE };
    t.accent = ORANGE;
    t.body_font = Font::montserrat_14();
    t.title_font = Font::montserrat_20();
    t
}
fn studio_row() -> View<StudioAction> {
    rs_lvgl_ui::row([])
        .style(Style::neutral())
        .width(percent(100))
        .gap(18)
        .scrollable(false)
}
fn studio_column() -> View<StudioAction> {
    rs_lvgl_ui::column([])
        .style(Style::neutral())
        .width(percent(100))
        .gap(18)
        .scrollable(false)
}
fn styled_label(value: &str, color: Color, font: Font) -> View<StudioAction> {
    rs_lvgl_ui::label(value)
        .text_color(color)
        .font(font)
        .scrollable(false)
}
fn text(value: &str, theme: &Theme) -> View<StudioAction> {
    styled_label(value, theme.muted, theme.body_font)
}
fn card(title: &str, caption: &str, theme: &Theme) -> View<StudioAction> {
    studio_column()
        .background(theme.surface)
        .radius(16)
        .padding(20)
        .gap(14)
        .border(
            if theme.background == palette(false).background {
                Color::rgb(232, 232, 226)
            } else {
                Color::rgb(51, 60, 74)
            },
            1,
        )
        .child(styled_label(title, theme.text, Font::montserrat_20()))
        .child(text(caption, theme).width(percent(100)))
}
fn action_button(value: &str, action: StudioAction, theme: &Theme) -> View<StudioAction> {
    rs_lvgl_ui::button(value)
        .focusable(true)
        .on_click(action)
        .style(Style::accent(theme))
        .background(theme.surface)
        .text_color(theme.text)
        .radius(8)
        .height(px(42))
        .padding_left(16)
        .padding_right(16)
        .shadow_width(0)
        .outline_width(0)
        .transition(StyleTransition::new(
            &[TransitionProperty::BackgroundColor],
            100,
        ))
        .state_style(
            State::PRESSED,
            style().background(ORANGE).text_color(WHITE).shadow_width(0),
        )
        .state_style(
            State::FOCUSED,
            style()
                .outline_width(2)
                .outline_pad(2)
                .outline_color(ORANGE),
        )
}
fn primary(value: &str, action: StudioAction, theme: &Theme) -> View<StudioAction> {
    action_button(value, action, theme)
        .background(ORANGE)
        .text_color(WHITE)
}

fn apply_theme(ui: &mut Ui<StudioAction>, app: &Studio) {
    // SAFETY: the display and built-in fonts are live on this single UI thread.
    unsafe {
        let native = rs_lvgl_sys::lv_theme_default_init(
            rs_lvgl_sys::lv_display_get_default(),
            rs_lvgl_sys::lv_color_t {
                red: ORANGE.r,
                green: ORANGE.g,
                blue: ORANGE.b,
            },
            rs_lvgl_sys::lv_color_t {
                red: TEAL.r,
                green: TEAL.g,
                blue: TEAL.b,
            },
            app.dark,
            &raw const rs_lvgl_sys::lv_font_montserrat_14,
        );
        rs_lvgl_sys::lv_display_set_theme(rs_lvgl_sys::lv_display_get_default(), native);
    }
    ui.set_theme(palette(app.dark));
}

#[cfg(not(target_os = "emscripten"))]
fn capture(ui: &mut Ui<StudioAction>, directory: &Path, name: &str) {
    let image = ui.snapshot().expect("studio snapshot");
    let mut ppm = format!("P6\n{} {}\n255\n", image.width, image.height).into_bytes();
    for pixel in image.pixels.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    std::fs::create_dir_all(directory).expect("capture directory");
    std::fs::write(directory.join(format!("studio-{name}.ppm")), ppm).expect("capture file");
}

#[cfg(not(target_os = "emscripten"))]
fn main() {
    let limit = std::env::var("STUDIO_FRAMES").ok().map(|v| {
        v.parse::<u64>()
            .expect("STUDIO_FRAMES must be a positive integer")
    });
    let autorun = std::env::var_os("STUDIO_AUTORUN").is_some();
    let capture_dir = std::env::var_os("STUDIO_CAPTURE_DIR").map(std::path::PathBuf::from);
    // SAFETY: native display/input creation, dispatch and destruction all stay
    // on this thread; normal teardown releases Ui and devices before Display.
    let (mouse, keyboard) = unsafe {
        rs_lvgl::init();
        let display =
            NonNull::new(rs_lvgl_sys::lv_sdl_window_create(WIDTH, HEIGHT)).expect("SDL display");
        rs_lvgl_sys::lv_sdl_window_set_title(
            display.as_ptr(),
            c"rs_lvgl / Component Studio".as_ptr().cast(),
        );
        (
            NonNull::new(rs_lvgl_sys::lv_sdl_mouse_create()).unwrap(),
            NonNull::new(rs_lvgl_sys::lv_sdl_keyboard_create()).unwrap(),
        )
    };
    let display = rs_lvgl::Display::default();
    let mut app = Studio::default();
    let mut ui = Ui::new(&display, palette(false)).unwrap();
    apply_theme(&mut ui, &app);
    ui.mount(&app).unwrap();
    unsafe {
        ui.bind_input(keyboard).unwrap();
    }
    let mut last = Instant::now();
    let mut frame = 0;
    loop {
        let now = Instant::now();
        let elapsed = now.duration_since(last);
        rs_lvgl::tick_inc(elapsed);
        last = now;
        let dark = app.dark;
        if runtime::frame(&mut ui, &mut app, 128).unwrap().is_none() {
            // Native SDL close already destroyed the display; don't drop stale roots.
            core::mem::forget(ui);
            unsafe {
                rs_lvgl_sys::lv_indev_delete(mouse.as_ptr());
                rs_lvgl_sys::lv_indev_delete(keyboard.as_ptr());
            }
            return;
        }
        if app.dark != dark {
            apply_theme(&mut ui, &app);
            ui.update(&app).unwrap();
        }
        let millis = u32::try_from(elapsed.as_millis()).unwrap_or(u32::MAX);
        ui.tick(millis).unwrap();
        if app.toasts.advance(millis) {
            ui.update(&app).unwrap();
        }
        if autorun && frame % 50 == 0 && frame / 50 < PAGES.len() as u64 {
            app.update(StudioAction::Page((frame / 50) as usize));
            ui.update(&app).unwrap();
        }
        if frame % 50 == 35 {
            if let Some(dir) = &capture_dir {
                capture(&mut ui, dir, &app.page.to_string());
            }
        }
        frame += 1;
        if app.quit || limit.is_some_and(|limit| frame >= limit) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    ui.unmount();
    drop(ui);
    unsafe {
        rs_lvgl_sys::lv_indev_delete(mouse.as_ptr());
        rs_lvgl_sys::lv_indev_delete(keyboard.as_ptr());
        display.delete();
    }
    println!("studio clean teardown after {frame} frames");
}

#[cfg(test)]
#[path = "studio/audit.rs"]
mod audit;
