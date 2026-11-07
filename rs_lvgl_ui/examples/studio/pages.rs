use super::*;
use components::{
    file_explorer::{self, FileEntry},
    list::{self, ListItem},
    nav,
    panel::Panel,
};
use std::rc::Rc;

pub(super) fn view(app: &Studio, theme: &Theme) -> View<StudioAction> {
    match app.page {
        0 => overview(app, theme),
        1 => controls(app, theme),
        2 => input(app, theme),
        3 => data(theme),
        4 => layout(app, theme),
        5 => media(app, theme),
        6 => canvas_page(app, theme),
        7 => composition(app, theme),
        9 => super::depth::view(app, theme),
        10 => super::charts::view(app, theme),
        _ => super::native::view(app, theme),
    }
}
fn half(node: View<StudioAction>, height: i16) -> View<StudioAction> {
    node.width(percent(49)).height(px(height))
}
fn third(node: View<StudioAction>, height: i16) -> View<StudioAction> {
    node.width(percent(32)).height(px(height))
}
fn full(height: i16) -> Style {
    style().width(percent(100)).height(px(height))
}
fn metric(value: &str, title: &str, color: Color, theme: &Theme) -> View<StudioAction> {
    studio_column()
        .width(content())
        .gap(6)
        .child(styled_label(value, color, Font::montserrat_32()))
        .child(text(title, theme))
}
fn signal_chart(height: i16) -> View<StudioAction> {
    rs_lvgl_ui::chart(ChartSpec::new(
        ChartType::Line,
        ChartAxes::new(ChartRange::new(0, 100), ChartRange::new(0, 100)),
        vec![
            ChartSeries::new(
                2,
                ORANGE,
                ChartData::values(vec![20, 33, 27, 56, 43, 70, 62, 79, 68, 84, 74, 92]),
            ),
            ChartSeries::new(
                1,
                TEAL,
                ChartData::values(vec![12, 18, 16, 26, 21, 40, 34, 48, 39, 55, 48, 64]),
            ),
        ],
    ))
    .style(
        full(height)
            .background_opacity(0)
            .border(Color::rgb(0, 0, 0), 0),
    )
}

fn overview(app: &Studio, theme: &Theme) -> View<StudioAction> {
    let plot = card(
        "A live workspace",
        "Synthetic signal / last 12 samples",
        theme,
    )
    .width(percent(64))
    .height(px(326))
    .child(studio_row().children([
        metric("84.2", "Signal strength", ORANGE, theme),
        metric("12", "Sample points", TEAL, theme),
    ]))
    .child(signal_chart(134));
    let control = card("Try the controls", "Changes carry between pages.", theme)
        .width(percent(34))
        .height(px(326))
        .child(
            studio_row()
                .justify_content(FlexAlign::SpaceBetween)
                .align_items(ItemAlign::Center)
                .align_content(FlexAlign::Center)
                .child(metric(
                    &format!("{}%", app.value),
                    "Output level",
                    theme.text,
                    theme,
                ))
                .child(
                    switch(app.enabled)
                        .focusable(true)
                        .on_value_changed(|v| StudioAction::Enabled(v != 0)),
                ),
        )
        .child(
            slider(0, 100, app.value)
                .key(Key::Named("overview-slider"))
                .style(full(12))
                .focusable(true)
                .on_value_changed(StudioAction::Value),
        )
        .child(primary("Explore components", StudioAction::Page(1), theme).style(full(44)))
        .child(text("Touch, click or use Tab + Enter.", theme));
    studio_column()
        .child(studio_row().children([plot, control]))
        .child(
            studio_row().children([
                third(
                    card("Compose", "Small nodes. Complete interfaces.", theme),
                    242,
                )
                .child(
                    spans([
                        Span::new("App").color(ORANGE),
                        Span::new("  /  View  /  "),
                        Span::new("Ui").color(TEAL),
                    ])
                    .style(full(40).font(Font::montserrat_20())),
                )
                .child(action_button(
                    "Open composition",
                    StudioAction::Page(7),
                    theme,
                )),
                third(
                    card("Create", "Pixels, motion and vector media.", theme),
                    242,
                )
                .child(studio_row().children([
                    rs_lvgl_ui::image(sample_image(false)),
                    text("Made for your device.", theme),
                ]))
                .child(action_button("Open media", StudioAction::Page(5), theme)),
                third(
                    card("Connect", "A native engine, a Rust workflow.", theme),
                    242,
                )
                .child(
                    studio_row().height(px(38)).children([
                        led(TEAL)
                            .width(px(18))
                            .height(px(18))
                            .radius(9)
                            .shadow_width(0),
                        text("LVGL 9.6 / native rendering", theme),
                    ]),
                )
                .child(action_button(
                    "Open a dialog",
                    StudioAction::Dialog(true),
                    theme,
                )),
            ]),
        )
}

fn controls(app: &Studio, t: &Theme) -> View<StudioAction> {
    studio_column()
        .child(
            studio_row().children([
                third(card("Output", "Slider / bar / shared value", t), 280)
                    .child(metric(
                        &format!("{}%", app.value),
                        "Controlled by application state",
                        ORANGE,
                        t,
                    ))
                    .child(
                        slider(0, 100, app.value)
                            .key(Key::Named("controls-slider"))
                            .style(full(14))
                            .focusable(true)
                            .on_value_changed(StudioAction::Value),
                    )
                    .child(progress_bar(0, 100, app.value).style(full(8))),
                third(
                    card("Switch it up", "Switch / checkbox / status LED", t),
                    280,
                )
                .child(
                    studio_row()
                        .justify_content(FlexAlign::SpaceBetween)
                        .align_items(ItemAlign::Center)
                        .align_content(FlexAlign::Center)
                        .children([
                            styled_label("Device enabled", t.text, t.body_font),
                            switch(app.enabled)
                                .key(Key::Named("enabled"))
                                .focusable(true)
                                .on_value_changed(|v| StudioAction::Enabled(v != 0)),
                        ]),
                )
                .child(
                    checkbox("Send notifications", app.checked)
                        .focusable(true)
                        .on_value_changed(|v| StudioAction::Checked(v != 0)),
                )
                .child(
                    studio_row().children([
                        led(if app.enabled { TEAL } else { t.muted })
                            .brightness(if app.enabled { 255 } else { 50 })
                            .width(px(20))
                            .height(px(20))
                            .radius(10)
                            .shadow_width(0),
                        text(
                            if app.enabled {
                                "Device is online"
                            } else {
                                "Device is paused"
                            },
                            t,
                        ),
                    ]),
                ),
                third(card("Read the room", "Arc / animated spinner", t), 280).child(
                    studio_row()
                        .justify_content(FlexAlign::SpaceAround)
                        .align_items(ItemAlign::Center)
                        .align_content(FlexAlign::Center)
                        .children([
                            arc(0, 100, app.value)
                                .width(px(136))
                                .height(px(136))
                                .arc_color(ORANGE)
                                .arc_width(12),
                            spinner()
                                .spinner_animation(1300, 70)
                                .width(px(46))
                                .height(px(46)),
                        ]),
                ),
            ]),
        )
        .child(
            studio_row().children([
                third(card("Choose a mode", "Dropdown / roller", t), 282)
                    .child(
                        dropdown(["Quiet", "Balanced", "Performance"], app.selected)
                            .style(full(42))
                            .focusable(true)
                            .on_value_changed(StudioAction::Select),
                    )
                    .child(
                        roller(["Quiet", "Balanced", "Performance"], app.selected)
                            .visible_rows(2)
                            .style(full(90))
                            .focusable(true)
                            .on_value_changed(StudioAction::Select),
                    ),
                third(card("Fine adjustments", "Spinbox / action buttons", t), 282)
                    .child(
                        spinbox(0, 100, app.value)
                            .key(Key::Named("spinbox"))
                            .digits(3, 0)
                            .style(full(48))
                            .focusable(true)
                            .on_value_changed(StudioAction::Value),
                    )
                    .child(studio_row().children([
                        action_button("-", StudioAction::Step(-1), t),
                        primary("+", StudioAction::Step(1), t),
                    ]))
                    .child(
                        action_button("Unavailable", StudioAction::Click, t)
                            .disabled(true)
                            .opacity(110),
                    ),
                third(
                    card("Quick actions", "Button matrix / event feedback", t),
                    282,
                )
                .child(
                    button_matrix([["1", "2", "3"], ["4", "5", "6"]])
                        .style(full(112))
                        .focusable(true)
                        .on_value_changed(|_| StudioAction::Click),
                )
                .child(text(&format!("{} actions received", app.clicks), t)),
            ]),
        )
}

fn input(app: &Studio, t: &Theme) -> View<StudioAction> {
    let editor = half(card("Say something", "Text area / native keyboard", t), 320)
        .child(
            textarea(&app.text)
                .placeholder("Your message")
                .keyboard_id(101)
                .key(Key::Named("text-input"))
                .style(full(44))
                .focusable(true)
                .on_text_changed(|s| StudioAction::Text(s.into())),
        )
        .child(keyboard(101).style(full(150)));
    let ime = half(card("Hello, world", "CJK type / Pinyin input", t), 320)
        .child(
            textarea(&app.chinese)
                .keyboard_id(102)
                .key(Key::Named("chinese-input"))
                .style(full(44).font(Font::cjk_16()))
                .focusable(true)
                .on_text_changed(|s| StudioAction::Chinese(s.into())),
        )
        .child(pinyin_keyboard(102, Font::cjk_16()).style(full(150)));
    studio_column().child(studio_row().children([editor, ime])).child(studio_row().children([
        half(card("Pick a date", &app.date, t), 264)
            .child(calendar(2026, 10).today(CalendarDate::new(2026, 10, 22))
                .highlighted([CalendarDate::new(2026, 10, 8), CalendarDate::new(2026, 10, 22)])
                .style(full(154)).on_value_changed(StudioAction::Date)),
        half(card("Type with personality", "Labels / spans / wrapping / fonts", t), 264)
            .child(spans([Span::new("One toolkit. ").color(t.text), Span::new("Many possibilities.").color(ORANGE)])
                .style(full(64).font(Font::montserrat_28())))
            .child(styled_label("你好，世界", t.text, Font::cjk_16()))
            .child(text("Enter text above. Use ni in the Pinyin field, then select a candidate. Every change updates application state.", t).width(percent(100))),
    ]))
}

fn data(t: &Theme) -> View<StudioAction> {
    studio_column()
        .child(
            card(
                "Signals over time",
                "Orange: channel A     Teal: channel B     /     synthetic data",
                t,
            )
            .style(full(280))
            .child(signal_chart(166)),
        )
        .child(
            studio_row().children([
                half(
                    card(
                        "Device inventory",
                        "A native table with selectable cells",
                        t,
                    ),
                    285,
                )
                .child(
                    table([
                        ["Device", "Mode", "Level"],
                        ["Studio A", "Balanced", "64%"],
                        ["Studio B", "Quiet", "28%"],
                        ["Studio C", "Active", "86%"],
                    ])
                    .column_widths([160, 146, 120])
                    .style(full(166))
                    .on_value_changed(|_| StudioAction::Click),
                ),
                half(card("Measure & trace", "Scale / polyline", t), 285)
                    .child(scale(0, 100, 21, 5).style(full(70)))
                    .child(
                        line([
                            (4, 55),
                            (58, 34),
                            (108, 43),
                            (165, 8),
                            (222, 27),
                            (284, 3),
                            (338, 18),
                            (410, 5),
                        ])
                        .style(full(70)),
                    )
                    .child(text("Geometry and measurement use native LVGL drawing.", t)),
            ]),
        )
}

fn layout(app: &Studio, t: &Theme) -> View<StudioAction> {
    let tabs = tabview(["Design", "Build", "Ship"], app.tab)
        .style(full(180))
        .children(
            [
                "Shape the interface.",
                "Connect your application.",
                "Make it your own.",
            ]
            .iter()
            .map(|s| {
                studio_column()
                    .padding(20)
                    .child(styled_label(s, t.text, Font::montserrat_20()))
            }),
        )
        .on_value_changed(StudioAction::Tab);
    let tiles = tileview(
        [
            Tile::new(0, 0, ScrollDirection::RIGHT),
            Tile::new(1, 0, ScrollDirection::LEFT),
        ],
        app.tile,
    )
    .style(full(138))
    .children([
        studio_column()
            .background(TEAL)
            .padding(20)
            .child(styled_label("01 / Explore", WHITE, Font::montserrat_20())),
        studio_column()
            .background(BLUE)
            .padding(20)
            .child(styled_label("02 / Create", WHITE, Font::montserrat_20())),
    ])
    .on_value_changed(StudioAction::Tile);
    let grid = grid(
        [GridTrack::Fr(1), GridTrack::Fr(1)],
        [GridTrack::Fr(1), GridTrack::Fr(1)],
        (0..4).map(|i| {
            action_button(&format!("Cell {}", i + 1), StudioAction::Click, t)
                .key(Key::Id(400 + i))
                .grid_cell((i % 2) as u16, (i / 2) as u16, 1, 1)
                .grid_align(GridAlign::Stretch, GridAlign::Stretch)
                .background(if i == 0 { ORANGE } else { t.background })
        }),
    )
    .style(full(142).gap(12))
    .grid_navigation(GridNavigation::default());
    studio_column()
        .child(
            studio_row().children([
                half(
                    card(
                        "One space, three tabs",
                        "Native tabview / controlled selection",
                        t,
                    ),
                    308,
                )
                .child(tabs),
                half(
                    card(
                        "Swipe to the next idea",
                        "Native tileview / touch or button",
                        t,
                    ),
                    308,
                )
                .child(tiles)
                .child(action_button(
                    "Next tile",
                    StudioAction::Tile((1 - app.tile) as i32),
                    t,
                )),
            ]),
        )
        .child(
            studio_row().children([
                half(
                    card("A flexible grid", "Arrow-key navigation / equal tracks", t),
                    254,
                )
                .child(grid),
                half(
                    card(
                        "Flex, with intention",
                        "Rows / columns / nested surfaces",
                        t,
                    ),
                    254,
                )
                .child(
                    studio_row().children([
                        studio_column()
                            .width(percent(30))
                            .height(px(140))
                            .background(TEAL)
                            .radius(10)
                            .padding(16)
                            .child(styled_label("Side", WHITE, t.body_font)),
                        studio_column().grow(1).gap(12).children(
                            ["Header", "Content", "Footer"].iter().map(|name| {
                                container()
                                    .style(full(38).background(t.background).radius(8))
                                    .child(text(name, t).align(Align::Center))
                            }),
                        ),
                    ]),
                ),
            ]),
        )
}

const fn tile_pixels(alternate: bool) -> [u8; 72 * 72 * 2] {
    let mut pixels = [0u8; 72 * 72 * 2];
    let mut y = 0;
    while y < 72 {
        let mut x = 0;
        while x < 72 {
            let dx = x as i32 - 36;
            let dy = y as i32 - 36;
            let (r, g, b) = if dx * dx + dy * dy < 25 * 25 {
                if alternate {
                    (91u16, 117u16, 218u16)
                } else {
                    (225, 88, 47)
                }
            } else if x % 12 < 1 || y % 12 < 1 {
                (208, 222, 211)
            } else {
                (234, 241, 231)
            };
            let value = ((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3);
            let index = (y * 72 + x) * 2;
            pixels[index] = value as u8;
            pixels[index + 1] = (value >> 8) as u8;
            x += 1;
        }
        y += 1;
    }
    pixels
}
static TILE_A: [u8; 72 * 72 * 2] = tile_pixels(false);
static TILE_B: [u8; 72 * 72 * 2] = tile_pixels(true);
fn sample_image(alternate: bool) -> ImageSource {
    ImageSource::rgb565(72, 72, if alternate { &TILE_B } else { &TILE_A })
}

fn media(app: &Studio, t: &Theme) -> View<StudioAction> {
    studio_column()
        .child(
            studio_row().children([
                third(card("Still & interactive", "Image / image button", t), 264)
                    .child(
                        studio_row()
                            .justify_content(FlexAlign::SpaceAround)
                            .align_items(ItemAlign::Center)
                            .align_content(FlexAlign::Center)
                            .children([
                                rs_lvgl_ui::image(sample_image(false)),
                                image_button(sample_image(false))
                                    .width(px(72))
                                    .height(px(72))
                                    .pressed_image(sample_image(true))
                                    .on_click(StudioAction::Click),
                            ]),
                    )
                    .child(text(&format!("{} image actions", app.clicks), t)),
                third(card("Frame by frame", "Animated image / GIF", t), 264)
                    .child(
                        studio_row()
                            .justify_content(FlexAlign::SpaceAround)
                            .align_items(ItemAlign::Center)
                            .align_content(FlexAlign::Center)
                            .children([
                                animated_image([sample_image(false), sample_image(true)], 1400),
                                gif(Gif::new(Rc::<[u8]>::from(
                                    &include_bytes!("../assets/status.gif")[..],
                                ))),
                            ]),
                    )
                    .child(text("Both animations run on native timers.", t).width(percent(100))),
                third(card("Vector in motion", "Lottie / ThorVG", t), 264).child(lottie(
                    Lottie::new(
                        Rc::<str>::from(include_str!("../assets/studio-motion.json")),
                        180,
                        130,
                    ),
                )),
            ]),
        )
        .child(
            studio_row().children([
                half(
                    card("Scan to explore", "QR code / repository address", t),
                    294,
                )
                .child(
                    studio_row().children([
                        qrcode(QrCode::new(
                            b"https://github.com/umbrella22/rs_lvgl".to_vec(),
                            166,
                        )),
                        studio_column()
                            .grow(1)
                            .child(styled_label(
                                "Open source.\nMade to build on.",
                                t.text,
                                Font::montserrat_20(),
                            ))
                            .child(text("github.com/umbrella22/rs_lvgl", t).width(percent(100))),
                    ]),
                ),
                half(
                    card("A readable identity", "Code 128 / encoded text", t),
                    294,
                )
                .child(barcode(Barcode::new("RS-LVGL-2026", 100)))
                .child(styled_label("RS-LVGL-2026", t.text, Font::montserrat_20()))
                .child(text("Generated by the native barcode extension.", t)),
            ]),
        )
}

fn canvas_page(app: &Studio, t: &Theme) -> View<StudioAction> {
    let mut frame = Canvas::new(940, 205).background(INK, 255);
    for x in (20..940).step_by(40) {
        frame = frame.draw(DrawCommand::Line {
            from: (x, 20),
            to: (x, 185),
            width: 1,
            color: Color::rgb(43, 53, 67),
            opacity: 255,
        });
    }
    for y in (25..205).step_by(40) {
        frame = frame.draw(DrawCommand::Line {
            from: (20, y),
            to: (920, y),
            width: 1,
            color: Color::rgb(43, 53, 67),
            opacity: 255,
        });
    }
    for x in 1..890_i16 {
        let y = |v: i16| (110.0 + (f32::from(v) / 68.0).sin() * 43.0) as i16;
        frame = frame.draw(DrawCommand::Line {
            from: (x + 20, y(x - 1)),
            to: (x + 21, y(x)),
            width: 3,
            color: ORANGE,
            opacity: 255,
        });
    }
    frame = frame
        .draw(DrawCommand::Rect {
            x: 38,
            y: 28,
            width: 138,
            height: 34,
            radius: 8,
            color: TEAL,
            opacity: 255,
        })
        .draw(DrawCommand::Text {
            x: 50,
            y: 36,
            width: 120,
            height: 24,
            text: "CANVAS / LIVE".into(),
            font: t.body_font,
            color: WHITE,
            opacity: 255,
        })
        .draw(DrawCommand::Pixel {
            x: 20,
            y: 20,
            color: WHITE,
            opacity: 255,
        });
    studio_column()
        .child(
            card(
                "Your pixels, your rules",
                "Lines / rounded rectangles / text / pixels",
                t,
            )
            .style(full(315))
            .child(rs_lvgl_ui::canvas(frame)),
        )
        .child(
            studio_row().children([
                third(card("Curved type", "Arc label", t), 248).child(
                    arc_label("RUST / LVGL", 48, 200, 160)
                        .width(px(200))
                        .height(px(130))
                        .text_color(TEAL)
                        .font(Font::montserrat_16()),
                ),
                third(
                    card("Native motion", "Scoped animation / reversible", t),
                    248,
                )
                .child(
                    container().style(full(110)).scrollable(false).child(
                        container()
                            .width(px(42))
                            .height(px(42))
                            .background(ORANGE)
                            .radius(12)
                            .align(Align::CenterLeft)
                            .animate(
                                Animation::new(AnimationProperty::X, 12, 200, 1400)
                                    .reverse(1400)
                                    .repeat(u32::MAX)
                                    .easing(Easing::EaseInOut),
                            ),
                    ),
                ),
                third(
                    card("Custom properties", "Props / tick / cleanup", t),
                    248,
                )
                .gap(8)
                .child(custom(super::native::Gauge::default(), super::native::GaugeProps {
                    value: app.value, color: TEAL, track: t.background,
                }).key(Key::Named("custom-gauge")))
                .child(row([
                    action_button("-5", StudioAction::Step(-5), t),
                    text(&format!("{}%", app.value), t),
                    action_button("+5", StudioAction::Step(5), t),
                ]).gap(10).align_items(ItemAlign::Center)),
            ]),
        )
}

fn composition(app: &Studio, t: &Theme) -> View<StudioAction> {
    let list = list::list(
        t,
        [
            ListItem::new("Studio workspace")
                .detail("Your components in one place")
                .on_click(StudioAction::Click)
                .view(t),
            ListItem::new("Device settings")
                .detail("Open the next page")
                .chevron(true)
                .on_click(StudioAction::Push)
                .view(t),
        ],
    )
    .style(full(136));
    let files = file_explorer::view(
        t,
        "/studio/assets",
        Some(StudioAction::File(0)),
        [
            FileEntry::directory(1, "Animations", StudioAction::File(1)),
            FileEntry::file(2, "studio-motion.json", StudioAction::File(2)),
            FileEntry::file(3, "status.gif", StudioAction::File(3)),
        ],
    )
    .style(full(160));
    studio_column()
        .child(
            studio_row().children([
                half(
                    Panel::new("Your workspace")
                        .action(action_button("Details", StudioAction::Push, t))
                        .child(list)
                        .view(t),
                    268,
                )
                .radius(16),
                half(
                    card(
                        "Files, without surprises",
                        "Application-provided sample entries",
                        t,
                    ),
                    278,
                )
                .child(files),
            ]),
        )
        .child(
            studio_row().children([
                half(
                    card("Go a little deeper", "Stack navigation / back behavior", t),
                    306,
                )
                .child(
                    nav::view(
                        &app.nav,
                        t,
                        *app.nav.current(),
                        studio_column()
                            .child(
                                text(
                                    if app.nav.can_go_back() {
                                        "A nested page. Back returns to the workspace."
                                    } else {
                                        "Start with a page, then push another."
                                    },
                                    t,
                                )
                                .width(percent(100)),
                            )
                            .child(action_button(
                                if app.nav.can_go_back() {
                                    "Back to workspace"
                                } else {
                                    "Open device"
                                },
                                if app.nav.can_go_back() {
                                    StudioAction::Back
                                } else {
                                    StudioAction::Push
                                },
                                t,
                            )),
                        StudioAction::Back,
                    )
                    .style(full(164)),
                ),
                half(
                    card("A moment of attention", "Message box / modal / toast", t).gap(10),
                    306,
                )
                .child(
                    message_box("Everything is ready.")
                        .message_box_title("Workspace prepared")
                        .message_box_buttons(["Got it"])
                        .style(full(140))
                        .on_value_changed(|_| StudioAction::Toast),
                )
                .child(studio_row().padding(6).children([
                    primary("Open dialog", StudioAction::Dialog(true), t),
                    action_button("Show toast", StudioAction::Toast, t),
                ])),
            ]),
        )
}
