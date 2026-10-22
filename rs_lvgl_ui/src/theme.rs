use crate::node::{Align, Color, FlexAlign, Font, Size, TextAlign};

/// Background color.
pub(crate) const STYLE_BG: u32 = 1 << 0;
pub(crate) const STYLE_FG: u32 = 1 << 1;
pub(crate) const STYLE_BORDER: u32 = 1 << 2;
pub(crate) const STYLE_RADIUS: u32 = 1 << 3;
pub(crate) const STYLE_PAD_LEFT: u32 = 1 << 4;
pub(crate) const STYLE_PAD_RIGHT: u32 = 1 << 5;
pub(crate) const STYLE_PAD_TOP: u32 = 1 << 6;
pub(crate) const STYLE_PAD_BOTTOM: u32 = 1 << 7;
pub(crate) const STYLE_GAP: u32 = 1 << 8;
pub(crate) const STYLE_WIDTH: u32 = 1 << 9;
pub(crate) const STYLE_HEIGHT: u32 = 1 << 10;
pub(crate) const STYLE_MAX_WIDTH: u32 = 1 << 11;
pub(crate) const STYLE_MAX_HEIGHT: u32 = 1 << 12;
pub(crate) const STYLE_MIN_WIDTH: u32 = 1 << 13;
pub(crate) const STYLE_MIN_HEIGHT: u32 = 1 << 14;
pub(crate) const STYLE_GROW: u32 = 1 << 15;
pub(crate) const STYLE_ALIGN: u32 = 1 << 16;
pub(crate) const STYLE_OPACITY: u32 = 1 << 17;
pub(crate) const STYLE_FONT: u32 = 1 << 18;
pub(crate) const STYLE_TEXT_ALIGN: u32 = 1 << 19;
pub(crate) const STYLE_HIDDEN: u32 = 1 << 20;
pub(crate) const STYLE_DISABLED: u32 = 1 << 21;
pub(crate) const STYLE_ARC_COLOR: u32 = 1 << 22;
pub(crate) const STYLE_ARC_WIDTH: u32 = 1 << 23;
pub(crate) const STYLE_ANIM_DURATION: u32 = 1 << 24;
/// Background opacity, tracked separately from the color so that
/// `surface.merged(bg_opacity(..))` keeps the surface color.
pub(crate) const STYLE_BG_OPA: u32 = 1 << 25;
/// Position offset from the aligned position.
pub(crate) const STYLE_OFFSET: u32 = 1 << 26;
pub(crate) const STYLE_FLOATING: u32 = 1 << 27;
/// Flex main and cross axis placement.
pub(crate) const STYLE_FLEX_ALIGN: u32 = 1 << 28;
const STYLE_ALL: u32 = (1 << 29) - 1;
/// Fields fixed by [`Style::neutral`]. Inheritable text properties (color,
/// font, alignment) are left unset so that children inherit them from the
/// nearest ancestor that sets them, as in LVGL itself.
const STYLE_ALL_NEUTRAL: u32 = STYLE_ALL
    & !(STYLE_ARC_COLOR
        | STYLE_ARC_WIDTH
        | STYLE_FG
        | STYLE_FONT
        | STYLE_TEXT_ALIGN
        | STYLE_OFFSET
        | STYLE_FLOATING
        | STYLE_FLEX_ALIGN);

/// Resolved visual properties for one mounted node.
///
/// A style records which fields the application set. Unset fields inherit the
/// LVGL widget class default, so a slider keeps its track and a button keeps
/// its surface unless the view overrides them. [`Style::neutral`] explicitly
/// clears every non-inheritable layout/appearance field and is the base for
/// plain containers and labels; text color, font and text alignment stay unset
/// so they inherit from the parent.
///
/// Styles are immutable values: builder methods return a new value. Assigning
/// a style to a node merges the fields it sets over the node's kind base, so
/// `Style::title(theme)` changes only the text color and font.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Style {
    pub(crate) bg: Color,
    pub(crate) bg_opa: u8,
    pub(crate) fg: Color,
    pub(crate) border: Color,
    pub(crate) border_width: i16,
    pub(crate) radius: i16,
    pub(crate) pad_left: i16,
    pub(crate) pad_right: i16,
    pub(crate) pad_top: i16,
    pub(crate) pad_bottom: i16,
    pub(crate) gap: i16,
    pub(crate) width: Size,
    pub(crate) height: Size,
    pub(crate) max_width: Size,
    pub(crate) max_height: Size,
    pub(crate) min_width: Size,
    pub(crate) min_height: Size,
    pub(crate) flex_grow: u8,
    pub(crate) align: Align,
    pub(crate) opacity: u8,
    pub(crate) font: Font,
    pub(crate) text_align: TextAlign,
    pub(crate) hidden: bool,
    pub(crate) disabled: bool,
    pub(crate) arc_color: Color,
    pub(crate) arc_width: i16,
    pub(crate) anim_duration: u32,
    pub(crate) offset_x: i16,
    pub(crate) offset_y: i16,
    pub(crate) floating: bool,
    pub(crate) flex_main: FlexAlign,
    pub(crate) flex_cross: FlexAlign,
    set: u32,
    pub(crate) transition: Option<crate::StyleTransition>,
    pub(crate) extended: crate::style_properties::ExtendedStyle,
}

impl Default for Style {
    /// A style that sets nothing and inherits every LVGL class default.
    fn default() -> Self {
        Self {
            bg: Color::rgb(0, 0, 0),
            bg_opa: 0,
            fg: Color::rgb(0, 0, 0),
            border: Color::rgb(0, 0, 0),
            border_width: 0,
            radius: 0,
            pad_left: 0,
            pad_right: 0,
            pad_top: 0,
            pad_bottom: 0,
            gap: 0,
            width: Size::Content,
            height: Size::Content,
            max_width: Size::Content,
            max_height: Size::Content,
            min_width: Size::Content,
            min_height: Size::Content,
            flex_grow: 0,
            align: Align::TopLeft,
            opacity: 255,
            font: Font::montserrat_14(),
            text_align: TextAlign::Auto,
            hidden: false,
            disabled: false,
            arc_color: Color::rgb(0, 0, 0),
            arc_width: 0,
            anim_duration: 0,
            offset_x: 0,
            offset_y: 0,
            floating: false,
            flex_main: FlexAlign::Start,
            flex_cross: FlexAlign::Start,
            set: 0,
            transition: None,
            extended: Default::default(),
        }
    }
}

impl Style {
    /// A neutral style: transparent, unpadded, square, content-sized and
    /// top-left aligned.
    ///
    /// Text color, font and text alignment are not set: LVGL inherits them
    /// from the parent, so `Node::column().style(Style::body(theme))` styles
    /// every label inside the column.
    pub fn neutral() -> Self {
        Self {
            set: STYLE_ALL_NEUTRAL,
            ..Self::default()
        }
    }

    pub(crate) fn is_set(&self, flag: u32) -> bool {
        self.set & flag != 0
    }

    /// Merge the fields set by `other` over this style.
    pub(crate) fn merged(mut self, other: Self) -> Self {
        if other.is_set(STYLE_BG) {
            self.bg = other.bg;
        }
        if other.is_set(STYLE_BG_OPA) {
            self.bg_opa = other.bg_opa;
        }
        if other.is_set(STYLE_FG) {
            self.fg = other.fg;
        }
        if other.is_set(STYLE_BORDER) {
            self.border = other.border;
            self.border_width = other.border_width;
        }
        if other.is_set(STYLE_RADIUS) {
            self.radius = other.radius;
        }
        if other.is_set(STYLE_PAD_LEFT) {
            self.pad_left = other.pad_left;
        }
        if other.is_set(STYLE_PAD_RIGHT) {
            self.pad_right = other.pad_right;
        }
        if other.is_set(STYLE_PAD_TOP) {
            self.pad_top = other.pad_top;
        }
        if other.is_set(STYLE_PAD_BOTTOM) {
            self.pad_bottom = other.pad_bottom;
        }
        if other.is_set(STYLE_GAP) {
            self.gap = other.gap;
        }
        if other.is_set(STYLE_WIDTH) {
            self.width = other.width;
        }
        if other.is_set(STYLE_HEIGHT) {
            self.height = other.height;
        }
        if other.is_set(STYLE_MAX_WIDTH) {
            self.max_width = other.max_width;
        }
        if other.is_set(STYLE_MAX_HEIGHT) {
            self.max_height = other.max_height;
        }
        if other.is_set(STYLE_MIN_WIDTH) {
            self.min_width = other.min_width;
        }
        if other.is_set(STYLE_MIN_HEIGHT) {
            self.min_height = other.min_height;
        }
        if other.is_set(STYLE_GROW) {
            self.flex_grow = other.flex_grow;
        }
        if other.is_set(STYLE_ALIGN) {
            self.align = other.align;
        }
        if other.is_set(STYLE_OPACITY) {
            self.opacity = other.opacity;
        }
        if other.is_set(STYLE_FONT) {
            self.font = other.font;
        }
        if other.is_set(STYLE_TEXT_ALIGN) {
            self.text_align = other.text_align;
        }
        if other.is_set(STYLE_HIDDEN) {
            self.hidden = other.hidden;
        }
        if other.is_set(STYLE_DISABLED) {
            self.disabled = other.disabled;
        }
        if other.is_set(STYLE_ARC_COLOR) {
            self.arc_color = other.arc_color;
        }
        if other.is_set(STYLE_ARC_WIDTH) {
            self.arc_width = other.arc_width;
        }
        if other.is_set(STYLE_ANIM_DURATION) {
            self.anim_duration = other.anim_duration;
        }
        if other.is_set(STYLE_OFFSET) {
            self.offset_x = other.offset_x;
            self.offset_y = other.offset_y;
        }
        if other.is_set(STYLE_FLOATING) {
            self.floating = other.floating;
        }
        if other.is_set(STYLE_FLEX_ALIGN) {
            self.flex_main = other.flex_main;
            self.flex_cross = other.flex_cross;
        }
        if other.transition.is_some() {
            self.transition = other.transition;
        }
        self.extended.merge(other.extended);
        self.set |= other.set;
        self
    }

    /// Set an opaque background color.
    ///
    /// This also sets the background opacity to 255. Call
    /// [`Style::bg_opacity`] after this method for a translucent fill.
    pub fn bg(mut self, color: Color) -> Self {
        self.bg = color;
        self.bg_opa = 255;
        self.set |= STYLE_BG | STYLE_BG_OPA;
        self
    }

    /// Set the background opacity independently of the color.
    ///
    /// Merging `Style::default().bg_opacity(128)` over [`Style::surface`]
    /// keeps the surface color and makes it translucent.
    pub fn bg_opacity(mut self, opacity: u8) -> Self {
        self.bg_opa = opacity;
        self.set |= STYLE_BG_OPA;
        self
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
        self.set |= STYLE_FG;
        self
    }

    pub fn border(mut self, color: Color, width: i16) -> Self {
        self.border = color;
        self.border_width = width;
        self.set |= STYLE_BORDER;
        self
    }

    pub fn radius(mut self, radius: i16) -> Self {
        self.radius = radius;
        self.set |= STYLE_RADIUS;
        self
    }

    /// Set all four paddings.
    pub fn pad(mut self, pad: i16) -> Self {
        self.pad_left = pad;
        self.pad_right = pad;
        self.pad_top = pad;
        self.pad_bottom = pad;
        self.set |= STYLE_PAD_LEFT | STYLE_PAD_RIGHT | STYLE_PAD_TOP | STYLE_PAD_BOTTOM;
        self
    }

    pub fn pad_left(mut self, pad: i16) -> Self {
        self.pad_left = pad;
        self.set |= STYLE_PAD_LEFT;
        self
    }

    pub fn pad_right(mut self, pad: i16) -> Self {
        self.pad_right = pad;
        self.set |= STYLE_PAD_RIGHT;
        self
    }

    pub fn pad_top(mut self, pad: i16) -> Self {
        self.pad_top = pad;
        self.set |= STYLE_PAD_TOP;
        self
    }

    pub fn pad_bottom(mut self, pad: i16) -> Self {
        self.pad_bottom = pad;
        self.set |= STYLE_PAD_BOTTOM;
        self
    }

    pub fn gap(mut self, gap: i16) -> Self {
        self.gap = gap;
        self.set |= STYLE_GAP;
        self
    }

    pub fn width(mut self, width: Size) -> Self {
        self.width = width;
        self.set |= STYLE_WIDTH;
        self
    }

    pub fn height(mut self, height: Size) -> Self {
        self.height = height;
        self.set |= STYLE_HEIGHT;
        self
    }

    pub fn max_width(mut self, width: Size) -> Self {
        self.max_width = width;
        self.set |= STYLE_MAX_WIDTH;
        self
    }

    pub fn max_height(mut self, height: Size) -> Self {
        self.max_height = height;
        self.set |= STYLE_MAX_HEIGHT;
        self
    }

    pub fn min_width(mut self, width: Size) -> Self {
        self.min_width = width;
        self.set |= STYLE_MIN_WIDTH;
        self
    }

    pub fn min_height(mut self, height: Size) -> Self {
        self.min_height = height;
        self.set |= STYLE_MIN_HEIGHT;
        self
    }

    pub fn flex_grow(mut self, grow: u8) -> Self {
        self.flex_grow = grow;
        self.set |= STYLE_GROW;
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self.set |= STYLE_ALIGN;
        self
    }

    pub fn opacity(mut self, opacity: u8) -> Self {
        self.opacity = opacity;
        self.set |= STYLE_OPACITY;
        self
    }

    pub fn font(mut self, font: Font) -> Self {
        self.font = font;
        self.set |= STYLE_FONT;
        self
    }

    pub fn text_align(mut self, align: TextAlign) -> Self {
        self.text_align = align;
        self.set |= STYLE_TEXT_ALIGN;
        self
    }

    /// Offset the widget from its aligned position, in pixels.
    ///
    /// Layout containers (`row`, `column`, `grid`) position their children
    /// themselves; the offset applies to [`Style::floating`] children and to
    /// children of plain objects.
    pub fn offset(mut self, x: i16, y: i16) -> Self {
        self.offset_x = x;
        self.offset_y = y;
        self.set |= STYLE_OFFSET;
        self
    }

    /// Take the widget out of its parent's layout and scrolling
    /// (`LV_OBJ_FLAG_FLOATING`).
    ///
    /// A floating child is placed by [`Style::align`] and [`Style::offset`]
    /// relative to its parent and stays in place when the parent scrolls,
    /// which suits badges, floating buttons and backdrops. Main-part only.
    pub fn floating(mut self, floating: bool) -> Self {
        self.floating = floating;
        self.set |= STYLE_FLOATING;
        self
    }

    /// Place the children of a `row` or `column`: `main` along the flow
    /// direction, `cross` perpendicular to it, like CSS `justify-content` and
    /// `align-items`.
    ///
    /// `cross` sets both LVGL's cross place (items inside their line) and
    /// track place (lines inside the container), so it also moves the items
    /// of a single-line container. Unset, both are [`FlexAlign::Start`].
    /// Other kinds ignore it. Main-part only.
    pub fn flex_align(mut self, main: FlexAlign, cross: FlexAlign) -> Self {
        self.flex_main = main;
        self.flex_cross = cross;
        self.set |= STYLE_FLEX_ALIGN;
        self
    }

    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self.set |= STYLE_HIDDEN;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self.set |= STYLE_DISABLED;
        self
    }

    /// Indicator color for a typed `Arc`.
    pub fn arc_color(mut self, color: Color) -> Self {
        self.arc_color = color;
        self.set |= STYLE_ARC_COLOR;
        self
    }

    /// Indicator width for a typed `Arc`, in pixels.
    pub fn arc_width(mut self, width: i16) -> Self {
        self.arc_width = width;
        self.set |= STYLE_ARC_WIDTH;
        self
    }

    /// Style transition duration in milliseconds.
    pub fn anim_duration(mut self, duration: u32) -> Self {
        self.anim_duration = duration;
        self.set |= STYLE_ANIM_DURATION;
        self
    }

    pub fn background(&self) -> Color {
        self.bg
    }

    pub fn background_opacity(&self) -> u8 {
        self.bg_opa
    }

    pub fn foreground(&self) -> Color {
        self.fg
    }

    pub fn border_color(&self) -> Color {
        self.border
    }

    pub fn border_thickness(&self) -> i16 {
        self.border_width
    }

    pub fn corner_radius(&self) -> i16 {
        self.radius
    }

    pub fn padding_left(&self) -> i16 {
        self.pad_left
    }

    pub fn padding_right(&self) -> i16 {
        self.pad_right
    }

    pub fn padding_top(&self) -> i16 {
        self.pad_top
    }

    pub fn padding_bottom(&self) -> i16 {
        self.pad_bottom
    }

    pub fn spacing(&self) -> i16 {
        self.gap
    }

    pub fn width_value(&self) -> Size {
        self.width
    }

    pub fn height_value(&self) -> Size {
        self.height
    }

    pub fn max_width_value(&self) -> Size {
        self.max_width
    }

    pub fn max_height_value(&self) -> Size {
        self.max_height
    }

    pub fn min_width_value(&self) -> Size {
        self.min_width
    }

    pub fn min_height_value(&self) -> Size {
        self.min_height
    }

    pub fn grow(&self) -> u8 {
        self.flex_grow
    }

    pub fn alignment(&self) -> Align {
        self.align
    }

    pub fn opacity_value(&self) -> u8 {
        self.opacity
    }

    pub fn text_font(&self) -> Font {
        self.font
    }

    pub fn text_alignment(&self) -> TextAlign {
        self.text_align
    }

    pub fn is_hidden(&self) -> bool {
        self.hidden
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn offset_value(&self) -> (i16, i16) {
        (self.offset_x, self.offset_y)
    }

    pub fn is_floating(&self) -> bool {
        self.floating
    }

    pub fn flex_align_value(&self) -> (FlexAlign, FlexAlign) {
        (self.flex_main, self.flex_cross)
    }

    pub fn arc_color_value(&self) -> Color {
        self.arc_color
    }

    pub fn arc_width_value(&self) -> i16 {
        self.arc_width
    }

    pub fn anim_duration_value(&self) -> u32 {
        self.anim_duration
    }

    /// Title text using the theme's title font and text color.
    pub fn title(theme: &Theme) -> Self {
        Self::default().fg(theme.text).font(theme.title_font)
    }

    /// Body text using the theme's body font and text color.
    pub fn body(theme: &Theme) -> Self {
        Self::default().fg(theme.text).font(theme.body_font)
    }

    /// Muted body text.
    pub fn muted(theme: &Theme) -> Self {
        Self::default().fg(theme.muted).font(theme.body_font)
    }

    /// A raised surface with padding and rounded corners.
    pub fn surface(theme: &Theme) -> Self {
        Self::default()
            .bg(theme.surface)
            .fg(theme.text)
            .pad(12)
            .radius(8)
    }

    /// A flat accent-filled surface with no inherited native-theme shadow.
    /// Set `shadow_width` afterwards to opt into an explicit shadow.
    pub fn accent(theme: &Theme) -> Self {
        Self::default()
            .bg(theme.accent)
            .fg(theme.background)
            .pad(8)
            .radius(8)
            .shadow_width(0)
    }

    /// Full-screen background.
    pub fn screen(theme: &Theme) -> Self {
        Self::default()
            .bg(theme.background)
            .width(Size::Percent(100))
            .height(Size::Percent(100))
            .pad(0)
    }
}

/// Montserrat 28 when `font-montserrat-28` is enabled, otherwise the default
/// Montserrat 14.
const fn title_font() -> Font {
    #[cfg(feature = "font-montserrat-28")]
    {
        Font::montserrat_28()
    }
    #[cfg(not(feature = "font-montserrat-28"))]
    {
        Font::montserrat_14()
    }
}

/// Token values used by application views.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Theme {
    pub background: Color,
    pub surface: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub danger: Color,
    pub body_font: Font,
    /// Montserrat 28 in the built-in themes, or Montserrat 14 when the
    /// `font-montserrat-28` feature is disabled.
    pub title_font: Font,
}

impl Theme {
    pub const fn dark() -> Self {
        Self {
            background: Color::rgb(0x10, 0x14, 0x18),
            surface: Color::rgb(0x1b, 0x20, 0x27),
            text: Color::rgb(0xf2, 0xf5, 0xf7),
            muted: Color::rgb(0x9a, 0xa5, 0xb1),
            accent: Color::rgb(0x33, 0xc2, 0xa0),
            danger: Color::rgb(0xe5, 0x53, 0x4b),
            body_font: Font::montserrat_14(),
            title_font: title_font(),
        }
    }

    pub const fn light() -> Self {
        Self {
            background: Color::rgb(0xf7, 0xf8, 0xfa),
            surface: Color::rgb(0xff, 0xff, 0xff),
            text: Color::rgb(0x16, 0x1b, 0x21),
            muted: Color::rgb(0x5f, 0x6b, 0x78),
            accent: Color::rgb(0x0f, 0x8f, 0x74),
            danger: Color::rgb(0xc4, 0x2b, 0x24),
            body_font: Font::montserrat_14(),
            title_font: title_font(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
