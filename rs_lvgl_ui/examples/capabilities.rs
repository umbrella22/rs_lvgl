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
enum Msg {
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
    type Msg = Msg;
    fn update(&mut self, msg: Msg) {
        println!("gallery event: {msg:?}");
        match msg {
            Msg::Page(page) => self.next_page = Some(page),
            Msg::Dialog(open) => self.dialog = open,
            Msg::Snapshot => self.snapshot = true,
            Msg::Text(text) => self.text = text,
            Msg::Select(id) => self.status = format!("Selected entry {id}"),
            Msg::Quit => self.quit = true,
        }
    }
    fn view(&self, theme: &Theme) -> View<Msg> {
        let tabs = row(TABS.iter().enumerate().map(|(i, title)| {
            action_button(title, Msg::Page(i))
                .key(Key::Named(TABS[i]))
                .style(
                    Style::default()
                        .bg(if i == self.page { ACCENT } else { PANEL })
                        .fg(if i == self.page {
                            Color::rgb(10, 30, 30)
                        } else {
                            theme.text
                        }),
                )
        }))
        .key(Key::Named("tabs"))
        .style(
            Style::neutral()
                .pad(FOCUS_SPACE)
                .gap(12)
                .height(Size::Px(52)),
        )
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
                .style(Style::body(theme).fg(ACCENT).text_letter_space(3)),
            label(TABS[self.page]).style(Style::title(theme)),
            tabs,
            content
                .key(Key::Named("content"))
                .style(Style::default().width(Size::Percent(100)).flex_grow(1)),
            row([
                action_button("Open dialog", Msg::Dialog(true)).key(Key::Named("dialog-trigger")),
                action_button("Snapshot", Msg::Snapshot),
                label(&self.status).style(Style::body(theme).flex_grow(1)),
                action_button("Quit", Msg::Quit),
            ])
            .style(
                Style::neutral()
                    .width(Size::Percent(100))
                    .pad(FOCUS_SPACE)
                    .gap(12),
            ),
            label("Tab to move between groups / Arrow keys to select a tab / Enter to activate")
                .style(Style::body(theme).fg(Color::rgb(155, 171, 190))),
        ])
        .style(
            Style::screen(theme)
                .bg(Color::rgb(12, 20, 32))
                .pad(24)
                .gap(14),
        )
    }
    fn overlay(&self, theme: &Theme) -> Option<View<Msg>> {
        self.dialog.then(|| {
            Dialog::new("Modal focus")
                .text("Tab stays inside this dialog. Closing it restores the previous focus.")
                .button("Cancel", Msg::Dialog(false))
                .primary("Continue", Msg::Dialog(false))
                .on_dismiss(Msg::Dialog(false))
                .view(theme)
        })
    }
}
fn action_button(text: &str, msg: Msg) -> View<Msg> {
    let focus = Style::default()
        .outline_color(ACCENT)
        .outline_width(FOCUS_WIDTH)
        .outline_opacity(255)
        .outline_pad(FOCUS_PAD);
    rs_lvgl_ui::button(text)
        .style(
            Style::default()
                .bg(PANEL)
                .fg(Color::rgb(231, 238, 246))
                .radius(9)
                .pad(12)
                .shadow_width(0)
                .outline_color(ACCENT)
                .outline_width(0)
                .outline_pad(FOCUS_PAD)
                .transition(StyleTransition::new(
                    &[
                        TransitionProperty::OutlineWidth,
                        TransitionProperty::Opacity,
                    ],
                    140,
                )),
        )
        .state_style(State::FOCUSED, focus)
        .state_style(State::FOCUS_KEY, focus)
        .state_style(State::PRESSED, Style::default().opacity(150))
        .on_click(msg)
}
fn card(theme: &Theme, title: &str) -> View<Msg> {
    column([label(title).style(Style::body(theme).fg(ACCENT).text_letter_space(1))])
        .style(Style::surface(theme).bg(PANEL).pad(20).gap(16).radius(14))
}
fn styles(theme: &Theme) -> View<Msg> {
    column([
        row([])
            // Preserve the cards' outer outline and the blurred shadow,
            // including its downward offset, inside this clipping row.
            .style(Style::neutral().width(Size::Percent(100)).pad(20).gap(24))
            .child(
                card(theme, "GRADIENT / SHADOW")
                    .style(
                        Style::default()
                            .width(Size::Percent(48))
                            .height(Size::Px(164))
                            .bg(Color::rgb(25, 75, 105))
                            .bg_grad_color(Color::rgb(31, 45, 72))
                            .bg_grad_dir(GradientDirection::Horizontal)
                            .shadow_width(20)
                            .shadow_color(Color::rgb(0, 0, 0))
                            .shadow_opacity(130)
                            .shadow_offset_y(8),
                    )
                    .child(label("Native styles, retained per state").style(Style::body(theme)))
                    .child(action_button("Press and focus", Msg::Select(1))),
            )
            .child(
                card(theme, "OUTLINE / TEXT")
                    .style(
                        Style::default()
                            .width(Size::Percent(48))
                            .height(Size::Px(164))
                            .outline_width(1)
                            .outline_color(ACCENT),
                    )
                    .child(
                        label("Spacing + underline").style(
                            Style::body(theme)
                                .text_letter_space(2)
                                .text_decoration(TextDecoration::UNDERLINE),
                        ),
                    )
                    .child(
                        label("Soft state transitions")
                            .style(Style::body(theme).text_line_space(5)),
                    ),
            ),
        card(theme, "EXPLICIT ANIMATION")
            .style(
                Style::default()
                    .width(Size::Percent(100))
                    .height(Size::Px(120)),
            )
            .child(
                obj()
                    .style(
                        Style::neutral()
                            .width(Size::Percent(100))
                            .height(Size::Px(28)),
                    )
                    .child(
                        obj()
                            .style(
                                Style::neutral()
                                    .width(Size::Px(52))
                                    .height(Size::Px(18))
                                    .bg(ACCENT)
                                    .radius(9),
                            )
                            .animate(
                                Animation::new(AnimationProperty::X, 0, 600, 1500)
                                    .easing(Easing::EaseInOut)
                                    .reverse(1500)
                                    .repeat(u32::MAX),
                            ),
                    ),
            ),
    ])
    .style(Style::neutral().gap(24))
}
fn canvas_page(theme: &Theme) -> View<Msg> {
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
        .style(Style::default().width(Size::Percent(100)))
        .child(rs_lvgl_ui::canvas(frame))
        .child(
            label("Snapshot copies the mounted view into an independent RGBA image.")
                .style(Style::body(theme)),
        )
}
fn media(theme: &Theme) -> View<Msg> {
    row([
        card(theme, "QR / CODE 128")
            .style(Style::default().width(Size::Px(340)).height(Size::Px(340)))
            .child(qrcode(QrCode::new(b"LVGL Rust UI".to_vec(), 144)))
            .child(barcode(Barcode::new("LVGL 2026", 45)))
            .child(label("Binary QR payload / preserved spaces").style(Style::body(theme))),
        card(theme, "LOTTIE / GIF")
            .style(Style::default().flex_grow(1).height(Size::Px(340)))
            .child(
                row([
                    lottie(Lottie::new(
                        include_str!("../tests/assets/moving-square.json"),
                        160,
                        160,
                    )),
                    obj()
                        .style(Style::neutral().width(Size::Px(70)).height(Size::Px(70)))
                        .child(gif(Gif::new(Rc::<[u8]>::from(
                            include_bytes!("assets/status.gif").as_slice(),
                        )))),
                ])
                .style(Style::neutral().gap(40).height(Size::Px(170))),
            )
            .child(label("Vector motion + animated red/blue GIF").style(Style::body(theme))),
    ])
    .style(Style::neutral().width(Size::Percent(100)).gap(20))
}
impl Gallery {
    fn text_and_files(&self, theme: &Theme) -> View<Msg> {
        row([
            card(theme, "CJK / PINYIN")
                .style(Style::default().width(Size::Px(520)).height(Size::Px(380)))
                .child(
                    textarea(&self.text)
                        .key(Key::Named("text"))
                        .keyboard_id(1)
                        .style(
                            Style::default()
                                .font(Font::cjk_16())
                                .width(Size::Percent(100))
                                .height(Size::Px(50)),
                        )
                        .on_text_changed(|text| Msg::Text(text.into())),
                )
                .child(pinyin_keyboard(1, Font::cjk_16()).key(Key::Named("ime")))
                .child(
                    label("Type ni, then select a Chinese candidate.").style(Style::body(theme)),
                ),
            card(theme, "FILE EXPLORER")
                .style(Style::default().flex_grow(1).height(Size::Px(380)))
                .child(
                    file_explorer::view(
                        theme,
                        "/samples",
                        None,
                        [
                            FileEntry::directory(1, "Animations", Msg::Select(1)),
                            FileEntry::file(2, "moving-square.json", Msg::Select(2)),
                            FileEntry::file(3, "status.gif", Msg::Select(3)),
                        ],
                    )
                    .style(Style::default().flex_grow(1)),
                ),
        ])
        .style(Style::neutral().width(Size::Percent(100)).gap(20))
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
                    .post(Msg::Dialog(true))
                    .unwrap(),
                150 => screens
                    .current_mut()
                    .unwrap()
                    .post(Msg::Dialog(false))
                    .unwrap(),
                220 => screens.current_mut().unwrap().post(Msg::Page(1)).unwrap(),
                320 => screens.current_mut().unwrap().post(Msg::Snapshot).unwrap(),
                400 => screens.current_mut().unwrap().post(Msg::Page(2)).unwrap(),
                650 => screens.current_mut().unwrap().post(Msg::Page(3)).unwrap(),
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
