//! Theme tokens and resolved style data.

mod builders;
mod merge;
mod presets;

use crate::node::{Align, Color, FlexAlign, Font, ItemAlign, Size, TextAlign};

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
/// `surface.merged(style().background_opacity(..))` keeps the surface color.
pub(crate) const STYLE_BG_OPA: u32 = 1 << 25;
/// Position offset from the aligned position.
pub(crate) const STYLE_OFFSET: u32 = 1 << 26;
pub(crate) const STYLE_FLOATING: u32 = 1 << 27;
/// Independent flex main, item and line placement.
pub(crate) const STYLE_JUSTIFY_CONTENT: u32 = 1 << 28;
pub(crate) const STYLE_ALIGN_ITEMS: u32 = 1 << 29;
pub(crate) const STYLE_ALIGN_CONTENT: u32 = 1 << 30;
const STYLE_ALL: u32 = (1 << 31) - 1;
/// Fields fixed by [`Style::plain`]. Inheritable text properties (color,
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
        | STYLE_JUSTIFY_CONTENT
        | STYLE_ALIGN_ITEMS
        | STYLE_ALIGN_CONTENT);

/// Resolved visual properties for one mounted node.
///
/// A style records which fields the application set. Unset fields inherit the
/// LVGL widget class default, so a slider keeps its track and a button keeps
/// its surface unless the view overrides them. [`Style::plain`] explicitly
/// writes base appearance/layout values and is the base for plain containers
/// and labels. Extended properties and flex placement remain unset; text color,
/// font and text alignment stay unset
/// so they inherit from the parent.
///
/// Styles are immutable values: builder methods return a new value. Assigning
/// a style to a node merges the fields it sets over the node's kind base, so
/// `Style::title(theme)` changes only the text color and font.
/// Ordinary views initialize their embedded style automatically; prefer flat
/// modifiers such as `label("Title").font(theme.title_font).text_color(theme.text)`.
/// Use [`crate::style()`] for reusable patches and part/state overrides.
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
    pub(crate) animation_duration: u32,
    pub(crate) offset_x: i16,
    pub(crate) offset_y: i16,
    pub(crate) floating: bool,
    pub(crate) flex_main: FlexAlign,
    pub(crate) flex_cross: ItemAlign,
    pub(crate) flex_track: FlexAlign,
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
            animation_duration: 0,
            offset_x: 0,
            offset_y: 0,
            floating: false,
            flex_main: FlexAlign::Start,
            flex_cross: ItemAlign::Start,
            flex_track: FlexAlign::Start,
            set: 0,
            transition: None,
            extended: Default::default(),
        }
    }
}

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
