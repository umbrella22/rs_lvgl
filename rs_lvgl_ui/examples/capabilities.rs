//! Native capability gallery. See README's "Capability gallery" for features.
//! UI_FRAMES bounds the run; UI_AUTORUN=1 walks the pages and dialog automatically.
use core::ptr::NonNull;
use rs_lvgl_ui::components::{
    dialog::Dialog,
    file_explorer::{self, FileEntry},
};
use rs_lvgl_ui::prelude::*;
use std::{
    rc::Rc,
    time::{Duration, Instant},
};
const ACCENT: Color = Color::rgb(90, 220, 185);
const PANEL: Color = Color::rgb(25, 37, 54);
const FOCUS_WIDTH: i16 = 2;
const FOCUS_PAD: i16 = 3;
// Outlines draw outside the button; leave one extra pixel for antialiasing.
const FOCUS_SPACE: i16 = FOCUS_WIDTH + FOCUS_PAD + 1;
const TABS: [&str; 4] = ["Styles & motion", "Canvas", "Media", "Text & files"];
#[derive(Clone, Debug)]
enum CapabilityGalleryAction {
    Page(usize),
    Dialog(bool),
    Snapshot,
    Text(String),
    Select(u64),
    Quit,
}
struct Gallery {
    page: usize,
    next_page: Option<usize>,
    dialog: bool,
    snapshot: bool,
    text: String,
    status: String,
    quit: bool,
}
impl App for Gallery {
    type Action = CapabilityGalleryAction;
    fn update(&mut self, action: CapabilityGalleryAction) {
        println!("gallery event: {action:?}");
        match action {
            CapabilityGalleryAction::Page(page) => self.next_page = Some(page),
            CapabilityGalleryAction::Dialog(open) => self.dialog = open,
            CapabilityGalleryAction::Snapshot => self.snapshot = true,
            CapabilityGalleryAction::Text(text) => self.text = text,
            CapabilityGalleryAction::Select(id) => self.status = format!("Selected entry {id}"),
            CapabilityGalleryAction::Quit => self.quit = true,
        }
    }
    fn view(&self, theme: &Theme) -> View<CapabilityGalleryAction> {
        let tabs = row(TABS.iter().enumerate().map(|(i, title)| {
            action_button(title, CapabilityGalleryAction::Page(i))
                .key(Key::Named(TABS[i]))
                .background(if i == self.page { ACCENT } else { PANEL })
                .text_color(if i == self.page {
                    Color::rgb(10, 30, 30)
                } else {
                    theme.text
                })
        }))
        .key(Key::Named("tabs"))
        .padding(FOCUS_SPACE)
        .gap(12)
        .height(px(52))
        .grid_navigation(GridNavigation {
            axis: NavigationAxis::Horizontal,
            rollover: true,
            ..Default::default()
        });
        let content = match self.page {
            0 => styles(theme),
            1 => canvas_page(theme),
            2 => media(theme),
            _ => self.text_and_files(theme),
        };
        column([
            label("LVGL / CAPABILITY LAB")
                .font(theme.body_font)
                .text_color(theme.text)
                .text_color(ACCENT)
                .letter_spacing(3),
            label(TABS[self.page])
                .font(theme.title_font)
                .text_color(theme.text),
            tabs,
            content
                .key(Key::Named("content"))
                .width(percent(100))
                .grow(1),
            row([
                action_button("Open dialog", CapabilityGalleryAction::Dialog(true))
                    .key(Key::Named("dialog-trigger")),
                action_button("Snapshot", CapabilityGalleryAction::Snapshot),
                label(&self.status)
                    .font(theme.body_font)
                    .text_color(theme.text)
                    .grow(1),
                action_button("Quit", CapabilityGalleryAction::Quit),
            ])
            .width(percent(100))
            .padding(FOCUS_SPACE)
            .gap(12),
            label("Tab to move between groups / Arrow keys to select a tab / Enter to activate")
                .font(theme.body_font)
                .text_color(theme.text)
                .text_color(Color::rgb(155, 171, 190)),
        ])
        .style(Style::screen(theme))
        .background(Color::rgb(12, 20, 32))
        .padding(24)
        .gap(14)
    }
    fn overlay(&self, theme: &Theme) -> Option<View<CapabilityGalleryAction>> {
        self.dialog.then(|| {
            Dialog::new("Modal focus")
                .text("Tab stays inside this dialog. Closing it restores the previous focus.")
                .button("Cancel", CapabilityGalleryAction::Dialog(false))
                .primary("Continue", CapabilityGalleryAction::Dialog(false))
                .on_dismiss(CapabilityGalleryAction::Dialog(false))
                .view(theme)
        })
    }
}
fn action_button(text: &str, action: CapabilityGalleryAction) -> View<CapabilityGalleryAction> {
    let focus = style()
        .outline_color(ACCENT)
        .outline_width(FOCUS_WIDTH)
        .outline_opacity(255)
        .outline_offset(FOCUS_PAD);
    rs_lvgl_ui::button(text)
        .background(PANEL)
        .text_color(Color::rgb(231, 238, 246))
        .radius(9)
        .padding(12)
        .shadow_width(0)
        .outline_color(ACCENT)
        .outline_width(0)
        .outline_offset(FOCUS_PAD)
        .transition(StyleTransition::new(
            &[
                TransitionProperty::OutlineWidth,
                TransitionProperty::Opacity,
            ],
            140,
        ))
        .state_style(State::FOCUSED, focus)
        .state_style(State::FOCUS_KEY, focus)
        .state_style(State::PRESSED, style().opacity(150))
        .on_click(action)
}
fn card(theme: &Theme, title: &str) -> View<CapabilityGalleryAction> {
    column([label(title)
        .font(theme.body_font)
        .text_color(theme.text)
        .text_color(ACCENT)
        .letter_spacing(1)])
    .style(Style::surface(theme))
    .background(PANEL)
    .padding(20)
    .gap(16)
    .radius(14)
}
fn styles(theme: &Theme) -> View<CapabilityGalleryAction> {
    column([
        row([])
            // Preserve the cards' outer outline and the blurred shadow,
            // including its downward offset, inside this clipping row.
            .width(percent(100))
            .padding(20)
            .gap(24)
            .child(
                card(theme, "GRADIENT / SHADOW")
                    .width(percent(48))
                    .height(px(164))
                    .background(Color::rgb(25, 75, 105))
                    .background_gradient_color(Color::rgb(31, 45, 72))
                    .background_gradient_direction(GradientDirection::Horizontal)
                    .shadow_width(20)
                    .shadow_color(Color::rgb(0, 0, 0))
                    .shadow_opacity(130)
                    .shadow_offset_y(8)
                    .child(
                        label("Native styles, retained per state")
                            .font(theme.body_font)
                            .text_color(theme.text),
                    )
                    .child(action_button(
                        "Press and focus",
                        CapabilityGalleryAction::Select(1),
                    )),
            )
            .child(
                card(theme, "OUTLINE / TEXT")
                    .width(percent(48))
                    .height(px(164))
                    .outline_width(1)
                    .outline_color(ACCENT)
                    .child(
                        label("Spacing + underline")
                            .font(theme.body_font)
                            .text_color(theme.text)
                            .letter_spacing(2)
                            .text_decoration(TextDecoration::UNDERLINE),
                    )
                    .child(
                        label("Soft state transitions")
                            .font(theme.body_font)
                            .text_color(theme.text)
                            .line_spacing(5),
                    ),
            ),
        card(theme, "EXPLICIT ANIMATION")
            .width(percent(100))
            .height(px(120))
            .child(
                container().width(percent(100)).height(px(28)).child(
                    container()
                        .width(px(52))
                        .height(px(18))
                        .background(ACCENT)
                        .radius(9)
                        .animate(
                            Animation::new(AnimationProperty::X, 0, 600, 1500)
                                .easing(Easing::EaseInOut)
                                .reverse(1500)
                                .repeat(u32::MAX),
                        ),
                ),
            ),
    ])
    .gap(24)
}
fn canvas_page(theme: &Theme) -> View<CapabilityGalleryAction> {
    let mut frame = Canvas::new(820, 260).background(Color::rgb(18, 30, 45), 255);
    for x in (20..820).step_by(40) {
        frame = frame.draw(DrawCommand::Line {
            from: (x, 0),
            to: (x, 259),
            width: 1,
            color: Color::rgb(32, 48, 64),
            opacity: 255,
        });
    }
    for x in 1..800_i16 {
        let y = |v: i16| (135.0 + (f32::from(v) / 43.0).sin() * 68.0) as i16;
        frame = frame.draw(DrawCommand::Line {
            from: (x + 9, y(x - 1)),
            to: (x + 10, y(x)),
            width: 3,
            color: ACCENT,
            opacity: 255,
        });
    }
    frame = frame.draw(DrawCommand::Text {
        x: 20,
        y: 15,
        width: 600,
        height: 30,
        text: "Owned drawing buffer / lines, text and pixels".into(),
        font: theme.body_font,
        color: theme.text,
        opacity: 255,
    });
    card(theme, "DRAW COMMANDS")
        .width(percent(100))
        .child(rs_lvgl_ui::canvas(frame))
        .child(
            label("Snapshot copies the mounted view into an independent RGBA image.")
                .font(theme.body_font)
                .text_color(theme.text),
        )
}
fn media(theme: &Theme) -> View<CapabilityGalleryAction> {
    row([
        card(theme, "QR / CODE 128")
            .width(px(340))
            .height(px(340))
            .child(qrcode(QrCode::new(b"LVGL Rust UI".to_vec(), 144)))
            .child(barcode(Barcode::new("LVGL 2026", 45)))
            .child(
                label("Binary QR payload / preserved spaces")
                    .font(theme.body_font)
                    .text_color(theme.text),
            ),
        card(theme, "LOTTIE / GIF")
            .grow(1)
            .height(px(340))
            .child(
                row([
                    lottie(Lottie::new(
                        include_str!("../tests/assets/moving-square.json"),
                        160,
                        160,
                    )),
                    container().width(px(70)).height(px(70)).child(gif(Gif::new(
                        Rc::<[u8]>::from(include_bytes!("assets/status.gif").as_slice()),
                    ))),
                ])
                .gap(40)
                .height(px(170)),
            )
            .child(
                label("Vector motion + animated red/blue GIF")
                    .font(theme.body_font)
                    .text_color(theme.text),
            ),
    ])
    .width(percent(100))
    .gap(20)
}
impl Gallery {
    fn text_and_files(&self, theme: &Theme) -> View<CapabilityGalleryAction> {
        row([
            card(theme, "CJK / PINYIN")
                .width(px(520))
                .height(px(380))
                .child(
                    textarea(&self.text)
                        .key(Key::Named("text"))
                        .keyboard_id(1)
                        .font(Font::cjk_16())
                        .width(percent(100))
                        .height(px(50))
                        .on_text_changed(|text| CapabilityGalleryAction::Text(text.into())),
                )
                .child(pinyin_keyboard(1, Font::cjk_16()).key(Key::Named("ime")))
                .child(
                    label("Type ni, then select a Chinese candidate.")
                        .font(theme.body_font)
                        .text_color(theme.text),
                ),
            card(theme, "FILE EXPLORER").grow(1).height(px(380)).child(
                file_explorer::view(
                    theme,
                    "/samples",
                    None,
                    [
                        FileEntry::directory(1, "Animations", CapabilityGalleryAction::Select(1)),
                        FileEntry::file(
                            2,
                            "moving-square.json",
                            CapabilityGalleryAction::Select(2),
                        ),
                        FileEntry::file(3, "status.gif", CapabilityGalleryAction::Select(3)),
                    ],
                )
                .grow(1),
            ),
        ])
        .width(percent(100))
        .gap(20)
    }
}
fn main() {
    let limit = std::env::var("UI_FRAMES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok());
    let autorun = std::env::var_os("UI_AUTORUN").is_some();
    // SAFETY: all native/UI work remains on this thread; normal exit releases
    // screens and inputs before deleting the display.
    let (mouse, keyboard) = unsafe {
        rs_lvgl::init();
        let display = NonNull::new(rs_lvgl_sys::lv_sdl_window_create(960, 760)).unwrap();
        rs_lvgl_sys::lv_sdl_window_set_title(display.as_ptr(), c"LVGL capability lab".as_ptr());
        (
            NonNull::new(rs_lvgl_sys::lv_sdl_mouse_create()).unwrap(),
            NonNull::new(rs_lvgl_sys::lv_sdl_keyboard_create()).unwrap(),
        )
    };
    let display = rs_lvgl::Display::default();
    let mut screens = Screens::new(&display, Theme::dark()).unwrap();
    let mut app = Gallery {
        page: 0,
        next_page: None,
        dialog: false,
        snapshot: false,
        text: "你好".into(),
        status: "Ready".into(),
        quit: false,
    };
    screens.show(&app, ScreenTransition::immediate()).unwrap();
    unsafe {
        screens.current_mut().unwrap().bind_input(keyboard).unwrap();
    }
    let mut last = Instant::now();
    let mut frame = 0;
    loop {
        let now = Instant::now();
        rs_lvgl::tick_inc(now.duration_since(last));
        last = now;
        screens.timer_handler();
        // SDL's title-bar close deletes the native display inside its handler.
        // Native deletion has already released objects; Rust tree handles must
        // not call into that deleted display. Prefer Quit or a bounded run.
        if unsafe { rs_lvgl_sys::lv_display_get_default() }.is_null() {
            core::mem::forget(screens);
            unsafe {
                rs_lvgl_sys::lv_indev_delete(mouse.as_ptr());
                rs_lvgl_sys::lv_indev_delete(keyboard.as_ptr());
            }
            return;
        }
        if autorun {
            match frame {
                80 => screens
                    .current_mut()
                    .unwrap()
                    .post(CapabilityGalleryAction::Dialog(true))
                    .unwrap(),
                150 => screens
                    .current_mut()
                    .unwrap()
                    .post(CapabilityGalleryAction::Dialog(false))
                    .unwrap(),
                220 => screens
                    .current_mut()
                    .unwrap()
                    .post(CapabilityGalleryAction::Page(1))
                    .unwrap(),
                320 => screens
                    .current_mut()
                    .unwrap()
                    .post(CapabilityGalleryAction::Snapshot)
                    .unwrap(),
                400 => screens
                    .current_mut()
                    .unwrap()
                    .post(CapabilityGalleryAction::Page(2))
                    .unwrap(),
                650 => screens
                    .current_mut()
                    .unwrap()
                    .post(CapabilityGalleryAction::Page(3))
                    .unwrap(),
                _ => {}
            }
        }
        screens.current_mut().unwrap().drain(&mut app, 64).unwrap();
        if let Some(page) = app.next_page.take().filter(|page| *page != app.page) {
            app.page = page;
            screens
                .show(&app, ScreenTransition::new(ScreenEffect::MoveLeft, 220))
                .unwrap();
            unsafe {
                screens.current_mut().unwrap().bind_input(keyboard).unwrap();
            }
            screens
                .current_mut()
                .unwrap()
                .focus(Key::Named(TABS[app.page]))
                .unwrap();
        }
        if app.snapshot {
            let image = screens.current_mut().unwrap().snapshot().unwrap();
            app.status = format!("Captured {} x {}", image.width, image.height);
            app.snapshot = false;
            println!("{}: {} RGBA bytes", app.status, image.pixels.len());
            screens.current_mut().unwrap().update(&app).unwrap();
        }
        frame += 1;
        if app.quit || limit.is_some_and(|limit| frame >= limit) {
            break;
        }
        std::thread::sleep(Duration::from_millis(8));
    }
    screens.unmount();
    unsafe {
        rs_lvgl_sys::lv_indev_delete(mouse.as_ptr());
        rs_lvgl_sys::lv_indev_delete(keyboard.as_ptr());
        display.delete();
    }
    println!("gallery clean teardown after {frame} frames");
}

#[cfg(test)]
#[path = "support/capabilities_audit.rs"]
mod audit;
