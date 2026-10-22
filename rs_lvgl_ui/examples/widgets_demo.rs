//! A replica of LVGL's official `widgets` demo built with `rs_lvgl_ui`.
//!
//! Run with:
//!
//! ```text
//! cargo run -p rs_lvgl_ui --example widgets_demo --features simulator
//! ```
//!
//! The three tabs (Profile, Analytics, Shop) mirror the official demo's
//! content. Components the typed layer covers (layout, labels, buttons,
//! sliders, switches) are built as `Node`s; the remaining official widgets are
//! created through the documented `Node::raw` escape. See
//! `docs/component-studio.md` for the current component coverage.
//!
//! `DEMO_FRAMES=<n>` bounds a non-interactive smoke run. `DEMO_SCREENSHOT=<path>`
//! writes a PPM snapshot of the active screen, which is useful for visual
//! comparison with the C demo.

use core::ptr::NonNull;
use std::thread::sleep;
use std::time::{Duration, Instant};

use rs_lvgl::Display;
use rs_lvgl_sys::lv_obj_t;
use rs_lvgl_ui::{
    Align, App, ChartSeries, ChartType, Color, Event, GridTrack, ImageSource, Key, Node, Part,
    Size, Style, Theme, Ui,
};

#[path = "support/runtime.rs"]
mod runtime;

const WIDTH: i32 = 480;
const HEIGHT: i32 = 800;

const PALETTE: [Color; 4] = [
    Color::rgb(0x33, 0xc2, 0xa0),
    Color::rgb(0x2f, 0x80, 0xed),
    Color::rgb(0xf2, 0x9e, 0x2e),
    Color::rgb(0xe5, 0x53, 0x4b),
];

const fn product_image_data() -> [u8; 32 * 32 * 2] {
    let mut data = [0u8; 32 * 32 * 2];
    let mut y = 0;
    while y < 32 {
        let mut x = 0;
        while x < 32 {
            let red = ((x * 8) & 0x1f) as u16;
            let green = ((y * 8) & 0x3f) as u16;
            let blue = 0x0a_u16;
            let rgb = (red << 11) | (green << 5) | blue;
            let index = (y * 32 + x) * 2;
            data[index] = (rgb & 0xff) as u8;
            data[index + 1] = (rgb >> 8) as u8;
            x += 1;
        }
        y += 1;
    }
    data
}

static PRODUCT_IMAGE: [u8; 32 * 32 * 2] = product_image_data();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Tab {
    Profile,
    Analytics,
    Shop,
}

#[derive(Clone, Debug)]
enum Msg {
    Tab(Tab),
    ToggleTheme,
    Accent(usize),
    Speed(i32),
    TeamPlayer(bool),
    HardWorking(bool),
    Buy(usize),
    LogOut,
    Quit,
    Username(String),
    Password(String),
    Birthday(String),
    Gender(u32),
    Notification(usize, bool),
    Focus(Option<u64>),
}

struct Demo {
    tab: Tab,
    dark: bool,
    accent: usize,
    speed: i32,
    team_player: bool,
    hard_working: bool,
    purchases: u32,
    logouts: u32,
    last_buy: Option<usize>,
    quit: bool,
    username: String,
    password: String,
    birthday: String,
    gender: u32,
    notifications: [bool; 6],
    keyboard_for: Option<u64>,
}

impl Demo {
    fn new() -> Self {
        Self {
            tab: Tab::Profile,
            dark: true,
            accent: 0,
            speed: 65,
            team_player: true,
            hard_working: false,
            purchases: 0,
            logouts: 0,
            last_buy: None,
            quit: false,
            username: String::new(),
            password: String::new(),
            birthday: String::new(),
            gender: 0,
            notifications: [false, false, true, false, true, false],
            keyboard_for: None,
        }
    }

    fn tab_index(&self) -> u32 {
        match self.tab {
            Tab::Profile => 0,
            Tab::Analytics => 1,
            Tab::Shop => 2,
        }
    }

    fn effective_theme(&self, theme: &Theme) -> Theme {
        let mut theme = *theme;
        theme.accent = PALETTE[self.accent];
        theme
    }

    fn profile_page(&self, theme: &Theme) -> Vec<Node<Msg>> {
        let mut page = Vec::new();

        page.push(
            Node::row()
                .key(Key::Named("profile-header"))
                .style(Style::surface(theme).width(Size::Percent(100)).gap(12))
                .child(
                    Node::obj()
                        .key(Key::Named("avatar"))
                        .style(
                            Style::default()
                                .bg(theme.accent)
                                .width(Size::Px(72))
                                .height(Size::Px(72))
                                .radius(36),
                        )
                        .child(
                            Node::label("ES").style(
                                Style::body(theme)
                                    .fg(theme.background)
                                    .font(theme.title_font)
                                    .align(Align::Center),
                            ),
                        ),
                )
                .child(
                    Node::column()
                        .style(Style::default().flex_grow(1).gap(4))
                        .child(Node::label("Elena Smith").style(Style::title(theme)))
                        .child(
                            Node::label(
                                "This is a short description of me. Take a look at my profile!",
                            )
                            .style(Style::muted(theme).width(Size::Percent(100))),
                        )
                        .child(Node::label("Email: elena@smith.com").style(Style::body(theme)))
                        .child(Node::label("Phone: +79 246 123 4567").style(Style::body(theme))),
                ),
        );

        page.push(
            Node::row()
                .key(Key::Named("profile-actions"))
                .style(Style::default().width(Size::Percent(100)).gap(8))
                .child(
                    Node::button("Log out")
                        .key(Key::Named("logout"))
                        .style(Style::accent(theme).flex_grow(1))
                        .on_click(Msg::LogOut),
                )
                .child(
                    Node::button("Invite").key(Key::Named("invite")).style(
                        Style::accent(theme)
                            .flex_grow(1)
                            .disabled(true)
                            .opacity(140),
                    ),
                ),
        );

        let mut form = card(theme, "profile-form", "Your profile");
        form = form
            .child(Node::label("User name").style(Style::muted(theme)))
            .child(
                Node::textarea(self.username.as_str())
                    .placeholder("Your name")
                    .keyboard_id(1)
                    .key(Key::Named("username"))
                    .style(input_style(theme))
                    .on_text_changed(|text| Msg::Username(text.into()))
                    .on_event(|event| match event {
                        Event::Focused => Some(Msg::Focus(Some(1))),
                        Event::Defocused | Event::Ready | Event::Cancel => Some(Msg::Focus(None)),
                        _ => None,
                    }),
            )
            .child(Node::label("Password").style(Style::muted(theme)))
            .child(
                Node::textarea(self.password.as_str())
                    .placeholder("Min. 8 chars.")
                    .password(true)
                    .keyboard_id(2)
                    .key(Key::Named("password"))
                    .style(input_style(theme))
                    .on_text_changed(|text| Msg::Password(text.into()))
                    .on_event(|event| match event {
                        Event::Focused => Some(Msg::Focus(Some(2))),
                        Event::Defocused | Event::Ready | Event::Cancel => Some(Msg::Focus(None)),
                        _ => None,
                    }),
            )
            .child(
                Node::grid([GridTrack::Fr(1), GridTrack::Fr(1)], [GridTrack::Content])
                    .key(Key::Named("profile-grid"))
                    .style(Style::default().width(Size::Percent(100)).gap(12))
                    .child(
                        Node::column()
                            .grid_cell(0, 0, 1, 1)
                            .style(Style::default().gap(4))
                            .child(Node::label("Gender").style(Style::muted(theme)))
                            .child(
                                Node::dropdown(["Male", "Female", "Other"], self.gender)
                                    .key(Key::Named("gender"))
                                    .style(input_style(theme))
                                    .on_value_changed(|value| Msg::Gender(value.max(0) as u32)),
                            ),
                    )
                    .child(
                        Node::column()
                            .grid_cell(1, 0, 1, 1)
                            .style(Style::default().gap(4))
                            .child(Node::label("Birthday").style(Style::muted(theme)))
                            .child(
                                Node::textarea(self.birthday.as_str())
                                    .placeholder("01.01.1990")
                                    .key(Key::Named("birthday"))
                                    .style(input_style(theme))
                                    .on_text_changed(|text| Msg::Birthday(text.into())),
                            ),
                    ),
            )
            .child(Node::label("Calendar").style(Style::muted(theme)))
            .child(raw_calendar(Key::Named("calendar")));
        page.push(form);

        page.push(
            card(theme, "profile-skills", "Your skills")
                .child(Node::label("Experience").style(Style::muted(theme)))
                .child(
                    Node::slider(0, 100, self.speed)
                        .key(Key::Named("experience"))
                        .style(
                            Style::default()
                                .width(Size::Percent(100))
                                .height(Size::Px(16))
                                .anim_duration(120),
                        )
                        .part_style(Part::Indicator, Style::default().bg(theme.accent))
                        .on_value_changed(Msg::Speed),
                )
                .child(
                    Node::label(format!("Experience: {}%", self.speed))
                        .key(Key::Named("experience-label"))
                        .style(Style::body(theme)),
                )
                .child(
                    Node::row()
                        .style(Style::default().width(Size::Percent(100)))
                        .child(Node::label("Team player").style(Style::body(theme).flex_grow(1)))
                        .child(
                            Node::switch(self.team_player)
                                .key(Key::Named("team-player"))
                                .on_value_changed(|value| Msg::TeamPlayer(value != 0)),
                        ),
                )
                .child(
                    Node::row()
                        .style(Style::default().width(Size::Percent(100)))
                        .child(Node::label("Hard-working").style(Style::body(theme).flex_grow(1)))
                        .child(
                            Node::switch(self.hard_working)
                                .key(Key::Named("hard-working"))
                                .on_value_changed(|value| Msg::HardWorking(value != 0)),
                        ),
                ),
        );

        page
    }

    fn analytics_page(&self, theme: &Theme) -> Vec<Node<Msg>> {
        let mut page = Vec::new();

        page.push(
            Node::row()
                .key(Key::Named("analytics-charts"))
                .style(Style::default().width(Size::Percent(100)).gap(12))
                .child(
                    card_flex(theme, "chart-visitors", "Unique visitors").child(
                        Node::chart(
                            ChartType::Line,
                            0,
                            100,
                            vec![ChartSeries::new(
                                theme.accent,
                                vec![18, 32, 28, 45, 38, 52, 61, 49, 66, 58, 72, 81],
                            )],
                        )
                        .key(Key::Named("chart-visitors-plot"))
                        .style(
                            Style::default()
                                .width(Size::Percent(100))
                                .height(Size::Px(150)),
                        ),
                    ),
                )
                .child(
                    card_flex(theme, "chart-revenue", "Monthly revenue").child(
                        Node::chart(
                            ChartType::Bar,
                            0,
                            100,
                            vec![
                                ChartSeries::new(
                                    theme.muted,
                                    vec![22, 30, 26, 38, 34, 42, 45, 40, 48, 52, 47, 55],
                                ),
                                ChartSeries::new(
                                    theme.accent,
                                    vec![30, 38, 35, 48, 44, 55, 60, 52, 64, 68, 62, 74],
                                ),
                            ],
                        )
                        .key(Key::Named("chart-revenue-plot"))
                        .style(
                            Style::default()
                                .width(Size::Percent(100))
                                .height(Size::Px(150)),
                        ),
                    ),
                ),
        );

        page.push(
            Node::row()
                .key(Key::Named("analytics-gauges"))
                .style(Style::default().width(Size::Percent(100)).gap(12))
                .child(gauge_card(
                    theme,
                    "gauge-target",
                    "Monthly target",
                    68,
                    theme.accent,
                ))
                .child(gauge_card(
                    theme,
                    "gauge-sessions",
                    "Sessions",
                    54,
                    theme.danger,
                ))
                .child(gauge_card(
                    theme,
                    "gauge-speed",
                    "Network speed",
                    82,
                    theme.muted,
                )),
        );

        page.push(
            card(theme, "analytics-scale", "Linear scale").child(
                Node::scale(0, 100, 11, 5).key(Key::Named("scale")).style(
                    Style::default()
                        .width(Size::Percent(100))
                        .height(Size::Px(60))
                        .fg(theme.text),
                ),
            ),
        );

        let mut notifications = card(theme, "analytics-notifications", "Notifications");
        for (index, (text, disabled)) in [
            ("Item purchased", false),
            ("New connection", false),
            ("New subscriber", false),
            ("New message", true),
            ("Milestone reached", true),
            ("Out of stock", false),
        ]
        .iter()
        .enumerate()
        {
            notifications = notifications.child(
                Node::checkbox(*text, self.notifications[index])
                    .key(Key::Id(index as u64))
                    .style(Style::default().fg(theme.text).disabled(*disabled))
                    .on_value_changed(move |value| Msg::Notification(index, value != 0)),
            );
        }
        page.push(notifications);

        page
    }

    fn shop_page(&self, theme: &Theme) -> Vec<Node<Msg>> {
        let mut page = Vec::new();

        page.push(
            card(theme, "monthly-summary", "Monthly Summary")
                .child(Node::label("8-15 July, 2021").style(Style::muted(theme)))
                .child(Node::label("$27,123.25").style(Style::title(theme)))
                .child(
                    Node::label("17% growth this week")
                        .style(Style::body(theme).fg(Color::rgb(0x2e, 0xa0, 0x6a))),
                )
                .child(
                    Node::chart(
                        ChartType::Stacked,
                        0,
                        100,
                        vec![
                            ChartSeries::new(
                                Color::rgb(0x2e, 0xa0, 0x6a),
                                vec![20, 32, 28, 40, 35, 44, 38, 48],
                            ),
                            ChartSeries::new(
                                Color::rgb(0x2f, 0x80, 0xed),
                                vec![18, 24, 22, 30, 28, 34, 31, 38],
                            ),
                            ChartSeries::new(theme.danger, vec![14, 20, 18, 24, 22, 28, 25, 30]),
                        ],
                    )
                    .key(Key::Named("summary-chart"))
                    .style(
                        Style::default()
                            .width(Size::Percent(100))
                            .height(Size::Px(150)),
                    ),
                ),
        );

        let mut products = card(theme, "top-products", "Top products");
        for (index, (name, category, price)) in [
            ("Blue T-shirt", "Clothes", "$722"),
            ("Blue T-shirt", "Clothes", "$411"),
            ("Blue T-shirt", "Clothes", "$917"),
            ("Blue T-shirt", "Clothes", "$64"),
            ("Blue T-shirt", "Clothes", "$805"),
        ]
        .iter()
        .enumerate()
        {
            products = products.child(
                Node::row()
                    .key(Key::Id(index as u64))
                    .style(Style::default().width(Size::Percent(100)).gap(8))
                    .child(
                        Node::image(ImageSource::rgb565(32, 32, &PRODUCT_IMAGE))
                            .key(Key::Named("thumb"))
                            .style(
                                Style::default()
                                    .width(Size::Px(32))
                                    .height(Size::Px(32))
                                    .radius(6),
                            ),
                    )
                    .child(
                        Node::column()
                            .style(Style::default().flex_grow(1))
                            .child(Node::label(*name).style(Style::body(theme)))
                            .child(Node::label(*category).style(Style::muted(theme))),
                    )
                    .child(Node::label(*price).style(Style::body(theme)))
                    .child(
                        Node::button("Buy")
                            .key(Key::Named("buy"))
                            .style(Style::accent(theme).pad(6))
                            .on_click(Msg::Buy(index)),
                    ),
            );
        }

        let mut shop_notifications = card(theme, "shop-notifications", "Notification");
        for (index, (text, disabled)) in [
            ("Item purchased", false),
            ("New connection", false),
            ("New subscriber", false),
            ("New message", true),
            ("Milestone reached", true),
            ("Out of stock", false),
        ]
        .iter()
        .enumerate()
        {
            shop_notifications = shop_notifications.child(
                Node::checkbox(*text, self.notifications[index])
                    .key(Key::Id(index as u64))
                    .style(Style::default().fg(theme.text).disabled(*disabled))
                    .on_value_changed(move |value| Msg::Notification(index, value != 0)),
            );
        }

        page.push(
            Node::column()
                .key(Key::Named("shop-lists"))
                .style(Style::default().width(Size::Percent(100)).gap(12))
                .child(products)
                .child(shop_notifications),
        );

        page
    }
}

impl App for Demo {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Tab(tab) => self.tab = tab,
            Msg::ToggleTheme => self.dark = !self.dark,
            Msg::Accent(index) => self.accent = index,
            Msg::Speed(value) => self.speed = value,
            Msg::TeamPlayer(value) => self.team_player = value,
            Msg::HardWorking(value) => self.hard_working = value,
            Msg::Buy(index) => {
                self.purchases += 1;
                self.last_buy = Some(index);
            }
            Msg::LogOut => self.logouts += 1,
            Msg::Quit => self.quit = true,
            Msg::Username(value) => self.username = value,
            Msg::Password(value) => self.password = value,
            Msg::Birthday(value) => self.birthday = value,
            Msg::Gender(value) => self.gender = value,
            Msg::Notification(index, checked) => {
                if let Some(slot) = self.notifications.get_mut(index) {
                    *slot = checked;
                }
            }
            Msg::Focus(target) => self.keyboard_for = target,
        }
    }

    fn view(&self, theme: &Theme) -> Node<Msg> {
        let theme = self.effective_theme(theme);

        Node::column()
            .style(Style::screen(&theme).gap(10).pad(12))
            .child(
                Node::row()
                    .key(Key::Named("header"))
                    .style(Style::default().width(Size::Percent(100)).gap(8))
                    .child(
                        Node::column()
                            .style(Style::default().flex_grow(1))
                            .child(Node::label("LVGL widgets demo").style(Style::title(&theme)))
                            .child(
                                Node::label("Recreated with rs_lvgl_ui")
                                    .style(Style::muted(&theme)),
                            ),
                    )
                    .child(
                        Node::button(if self.dark { "Light" } else { "Dark" })
                            .key(Key::Named("theme"))
                            .style(Style::accent(&theme))
                            .on_click(Msg::ToggleTheme),
                    )
                    .child(
                        Node::button("Quit")
                            .key(Key::Named("quit"))
                            .style(Style::accent(&theme))
                            .on_click(Msg::Quit),
                    ),
            )
            .child(
                Node::row()
                    .key(Key::Named("palette"))
                    .style(Style::default().width(Size::Percent(100)).gap(6))
                    .children((0..PALETTE.len()).map(|index| {
                        Node::button(" ")
                            .key(Key::Id(index as u64))
                            .style(
                                Style::default()
                                    .bg(PALETTE[index])
                                    .width(Size::Px(28))
                                    .height(Size::Px(28))
                                    .shadow_width(0)
                                    .radius(14),
                            )
                            .on_click(Msg::Accent(index))
                    })),
            )
            .child(
                Node::tabview(["Profile", "Analytics", "Shop"], self.tab_index())
                    .key(Key::Named("tabs"))
                    .style(
                        Style::default()
                            .width(Size::Percent(100))
                            .flex_grow(1),
                    )
                    .on_value_changed(|index| {
                        Msg::Tab(match index {
                            0 => Tab::Profile,
                            1 => Tab::Analytics,
                            _ => Tab::Shop,
                        })
                    })
                    .child(
                        Node::column()
                            .key(Key::Named("profile-page"))
                            .style(Style::default().width(Size::Percent(100)).gap(12))
                            .children(self.profile_page(&theme)),
                    )
                    .child(
                        Node::column()
                            .key(Key::Named("analytics-page"))
                            .style(Style::default().width(Size::Percent(100)).gap(12))
                            .children(self.analytics_page(&theme)),
                    )
                    .child(
                        Node::column()
                            .key(Key::Named("shop-page"))
                            .style(Style::default().width(Size::Percent(100)).gap(12))
                            .children(self.shop_page(&theme)),
                    ),
            )
            .children(self.keyboard_for.map(|target| {
                Node::keyboard(target)
                    .key(Key::Named("keyboard"))
                    .style(
                        Style::default()
                            .width(Size::Percent(100))
                            .height(Size::Px(230)),
                    )
                    .on_event(|event| match event {
                        Event::Ready | Event::Cancel => Some(Msg::Focus(None)),
                        _ => None,
                    })
            }))
            .child(
                Node::column()
                    .key(Key::Named("footer"))
                    .style(
                        Style::default()
                            .width(Size::Percent(100))
                            .gap(2)
                            .bg(theme.background)
                            .pad(4),
                    )
                    .child(
                        Node::label(format!(
                            "Purchases: {}   Logouts: {}   Last: {}",
                            self.purchases,
                            self.logouts,
                            self.last_buy
                                .map(|index| format!("#{}", index + 1))
                                .unwrap_or_else(|| "-".to_string())
                        ))
                        .key(Key::Named("status"))
                        .style(Style::muted(&theme)),
                    )
                    .child(
                        Node::label("Typed: grid, tabs, text, inputs, keyboard, charts, gauges, images, part styles.  Raw: calendar.")
                            .key(Key::Named("legend"))
                            .style(Style::muted(&theme).width(Size::Percent(100))),
                    ),
            )
    }
}

fn card(theme: &Theme, key: &'static str, title: &'static str) -> Node<Msg> {
    Node::column()
        .key(Key::Named(key))
        .style(Style::surface(theme).width(Size::Percent(100)).gap(8))
        .child(Node::label(title).style(Style::title(theme)))
}

fn card_flex(theme: &Theme, key: &'static str, title: &'static str) -> Node<Msg> {
    Node::column()
        .key(Key::Named(key))
        .style(Style::surface(theme).flex_grow(1).gap(8))
        .child(Node::label(title).style(Style::body(theme)))
}

fn input_style(theme: &Theme) -> Style {
    Style::default()
        .bg(theme.surface)
        .fg(theme.text)
        .border(theme.muted, 1)
        .radius(6)
        .pad(8)
        .width(Size::Percent(100))
}

fn gauge_card(
    theme: &Theme,
    key: &'static str,
    title: &'static str,
    value: i32,
    color: Color,
) -> Node<Msg> {
    card_flex(theme, key, title).child(
        Node::column()
            .style(Style::default().width(Size::Percent(100)).gap(2))
            .child(
                Node::arc(0, 100, value).key(Key::Named("gauge")).style(
                    Style::default()
                        .width(Size::Px(104))
                        .height(Size::Px(104))
                        .arc_color(color)
                        .arc_width(10),
                ),
            )
            .child(Node::label(format!("{value}%")).style(Style::body(theme))),
    )
}

fn raw_color(color: Color) -> rs_lvgl_sys::lv_color_t {
    rs_lvgl_sys::lv_color_t {
        blue: color.b,
        green: color.g,
        red: color.r,
    }
}

fn apply_theme(ui: &mut Ui<Msg>, app: &Demo) {
    // Application tokens and LVGL's native widget styles are separate. Keep
    // tabs, calendar children, checkboxes and keyboard keys in the same mode.
    // SAFETY: the example owns the sole live display, all updates run on its
    // UI thread, and the vendored font has static lifetime.
    unsafe {
        rs_lvgl_sys::lv_theme_default_init(
            rs_lvgl_sys::lv_display_get_default(),
            raw_color(PALETTE[app.accent]),
            raw_color(PALETTE[1]),
            app.dark,
            &raw const rs_lvgl_sys::lv_font_montserrat_14,
        );
    }
    ui.set_theme(if app.dark {
        Theme::dark()
    } else {
        Theme::light()
    });
}

/// Wrap a raw factory with the safety contract every factory below satisfies:
/// it creates exactly one direct child of `parent` (internal labels are children
/// of that object), returns it, and never deletes or retains `parent`.
fn raw(
    key: Key,
    create: impl Fn(NonNull<lv_obj_t>) -> Option<NonNull<lv_obj_t>> + 'static,
) -> Node<Msg> {
    // SAFETY: see the function contract above.
    unsafe { Node::raw(create) }.key(key)
}

fn raw_calendar(key: Key) -> Node<Msg> {
    raw(key, move |parent| unsafe {
        let calendar = rs_lvgl_sys::lv_calendar_create(parent.as_ptr());
        let calendar = NonNull::new(calendar)?;
        // Raw nodes own their layout; declarative styles are intentionally
        // skipped for them. Native theme colors follow apply_theme updates.
        rs_lvgl_sys::lv_obj_set_size(calendar.as_ptr(), rs_lvgl_sys::lv_pct(100), 260);
        rs_lvgl_sys::lv_calendar_set_month_shown(calendar.as_ptr(), 1990, 1);
        rs_lvgl_sys::lv_calendar_add_header_dropdown(calendar.as_ptr());
        Some(calendar)
    })
}

fn save_screenshot(path: &str) {
    // SAFETY: LVGL runs on this thread and the screen is alive for the loop.
    unsafe {
        let screen = rs_lvgl_sys::lv_screen_active();
        if screen.is_null() {
            return;
        }
        let width = rs_lvgl_sys::lv_obj_get_width(screen);
        let height = rs_lvgl_sys::lv_obj_get_height(screen);
        let buffer = rs_lvgl_sys::lv_snapshot_take(
            screen,
            rs_lvgl_sys::lv_color_format_t_LV_COLOR_FORMAT_ARGB8888,
        );
        if buffer.is_null() {
            eprintln!("snapshot failed");
            return;
        }
        let size = (*buffer).data_size as usize;
        let data = core::slice::from_raw_parts((*buffer).data, size);
        let pixels = width.max(0) as usize * height.max(0) as usize;
        let mut ppm = Vec::with_capacity(15 + pixels * 3);
        ppm.extend_from_slice(format!("P6\n{width} {height}\n255\n").as_bytes());
        for index in 0..pixels.min(data.len() / 4) {
            let offset = index * 4;
            // ARGB8888 little-endian: B, G, R, A.
            ppm.push(data[offset + 2]);
            ppm.push(data[offset + 1]);
            ppm.push(data[offset]);
        }
        if let Err(error) = std::fs::write(path, ppm) {
            eprintln!("screenshot write failed: {error}");
        }
        rs_lvgl_sys::lv_draw_buf_destroy(buffer);
    }
}

fn main() {
    let frame_limit: Option<u32> = std::env::var("DEMO_FRAMES")
        .ok()
        .and_then(|value| value.parse().ok());
    let screenshot = std::env::var("DEMO_SCREENSHOT").ok();

    // SAFETY: LVGL is initialized before the display is created, and all access
    // stays on this thread.
    unsafe {
        rs_lvgl::init();
        if rs_lvgl_sys::lv_sdl_window_create(WIDTH, HEIGHT).is_null() {
            panic!("lv_sdl_window_create failed");
        }
        if rs_lvgl_sys::lv_sdl_mouse_create().is_null() {
            panic!("lv_sdl_mouse_create failed");
        }
    }
    println!("SDL window created: {WIDTH}x{HEIGHT} (LVGL native mouse input)");

    let display = Display::default();
    let mut ui = Ui::new(&display, Theme::dark()).expect("ui creation");
    let mut app = Demo::new();
    app.tab = match std::env::var("DEMO_TAB").as_deref() {
        Ok("analytics") => Tab::Analytics,
        Ok("shop") => Tab::Shop,
        _ => Tab::Profile,
    };
    app.keyboard_for = std::env::var("DEMO_KEYBOARD")
        .ok()
        .and_then(|value| value.parse().ok());
    apply_theme(&mut ui, &app);
    ui.mount(&app).expect("mount");
    let scroll_y: Option<i32> = std::env::var("DEMO_SCROLL_Y")
        .ok()
        .and_then(|value| value.parse().ok());

    let mut last = Instant::now();
    let mut active_dark = true;
    let mut active_accent = app.accent;
    let mut frames = 0u32;
    let mut screenshot_taken = false;
    'running: loop {
        let now = Instant::now();
        rs_lvgl::tick_inc(now.duration_since(last));
        last = now;

        match runtime::frame(&mut ui, &mut app, 64) {
            Ok(Some(_)) => {}
            Ok(None) => {
                println!("SDL window closed");
                core::mem::forget(ui);
                return;
            }
            Err(error) => {
                eprintln!("ui error: {error}");
                break;
            }
        }
        if app.quit {
            break 'running;
        }
        if app.dark != active_dark || app.accent != active_accent {
            active_dark = app.dark;
            active_accent = app.accent;
            apply_theme(&mut ui, &app);
            if let Err(error) = ui.update(&app) {
                eprintln!("ui error: {error}");
                break;
            }
        }

        frames += 1;
        if frames == 10 {
            if let Some(y) = scroll_y {
                unsafe {
                    let ui_root = rs_lvgl_sys::lv_obj_get_child(rs_lvgl_sys::lv_screen_active(), 0);
                    let app_tree = rs_lvgl_sys::lv_obj_get_child(ui_root, 0);
                    let tabview = rs_lvgl_sys::lv_obj_get_child(app_tree, 2);
                    if !tabview.is_null() {
                        let content = rs_lvgl_sys::lv_tabview_get_content(tabview);
                        let active = rs_lvgl_sys::lv_tabview_get_tab_active(tabview);
                        let page = rs_lvgl_sys::lv_obj_get_child(content, active as i32);
                        if !page.is_null() {
                            rs_lvgl_sys::lv_obj_update_layout(page);
                            rs_lvgl_sys::lv_obj_scroll_to_y(page, y, false);
                        }
                    }
                }
            }
        }
        if !screenshot_taken && frames == 20 {
            if let Some(path) = &screenshot {
                save_screenshot(path);
                screenshot_taken = true;
                if frame_limit.is_none() {
                    break 'running;
                }
            }
        }
        if frame_limit.is_some_and(|limit| frames >= limit) {
            break 'running;
        }

        sleep(Duration::from_millis(5));
    }

    let stats = ui.stats();
    println!(
        "widgets demo stopped: {} nodes, {} messages, created {}, deleted {}, moved {}",
        stats.mounted_nodes,
        stats.messages_processed,
        stats.created_nodes,
        stats.deleted_nodes,
        stats.moved_nodes
    );

    ui.unmount();
    drop(ui);
    // SAFETY: the loop stopped, LVGL handled all events, and the UI is
    // unmounted. The SDL backend's display is deleted by `deinit`.
    unsafe { rs_lvgl::deinit() };
}

#[cfg(test)]
#[path = "support/widgets_audit.rs"]
mod audit;
