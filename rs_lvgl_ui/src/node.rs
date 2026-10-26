use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use cstr_core::CStr;

use crate::custom::CustomWidgetKind;
use crate::{Style, UiError};

/// A resolved RGB color.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn from_rgb((r, g, b): (u8, u8, u8)) -> Self {
        Self { r, g, b }
    }

    pub(crate) fn to_raw(self) -> rs_lvgl_sys::lv_color_t {
        rs_lvgl_sys::lv_color_t {
            blue: self.b,
            green: self.g,
            red: self.r,
        }
    }
}

/// A build-time LVGL font.
///
/// Fonts are static LVGL objects. `Font::montserrat_14` is always available;
/// `Font::montserrat_<size>` exists for every size enabled with a
/// `font-montserrat-<size>` feature. Custom fonts enter through
/// [`Font::from_raw`].
#[derive(Clone, Copy)]
pub struct Font(pub(crate) *const rs_lvgl_sys::lv_font_t);

impl Font {
    /// Wrap an LVGL font descriptor that remains valid for the program's
    /// lifetime.
    ///
    /// # Safety
    ///
    /// `raw` must point to a fully initialized, immutable `lv_font_t` that
    /// outlives every object using it. LVGL retains the pointer in its styles.
    pub const unsafe fn from_raw(raw: *const rs_lvgl_sys::lv_font_t) -> Self {
        Self(raw)
    }

    /// Vendored Source Han Sans SC CJK subset, 14 pixels.
    #[cfg(feature = "font-cjk-14")]
    pub const fn cjk_14() -> Self {
        Self(core::ptr::addr_of!(
            rs_lvgl_sys::lv_font_source_han_sans_sc_14_cjk
        ))
    }
    /// Vendored Source Han Sans SC CJK subset, 16 pixels.
    #[cfg(feature = "font-cjk-16")]
    pub const fn cjk_16() -> Self {
        Self(core::ptr::addr_of!(
            rs_lvgl_sys::lv_font_source_han_sans_sc_16_cjk
        ))
    }

    /// Montserrat 14, LVGL's default font. Always available.
    pub const fn montserrat_14() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_14))
    }

    #[cfg(feature = "font-montserrat-8")]
    pub const fn montserrat_8() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_8))
    }

    #[cfg(feature = "font-montserrat-10")]
    pub const fn montserrat_10() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_10))
    }

    #[cfg(feature = "font-montserrat-12")]
    pub const fn montserrat_12() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_12))
    }

    #[cfg(feature = "font-montserrat-16")]
    pub const fn montserrat_16() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_16))
    }

    #[cfg(feature = "font-montserrat-18")]
    pub const fn montserrat_18() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_18))
    }

    #[cfg(feature = "font-montserrat-20")]
    pub const fn montserrat_20() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_20))
    }

    #[cfg(feature = "font-montserrat-22")]
    pub const fn montserrat_22() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_22))
    }

    #[cfg(feature = "font-montserrat-24")]
    pub const fn montserrat_24() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_24))
    }

    #[cfg(feature = "font-montserrat-26")]
    pub const fn montserrat_26() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_26))
    }

    #[cfg(feature = "font-montserrat-28")]
    pub const fn montserrat_28() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_28))
    }

    #[cfg(feature = "font-montserrat-30")]
    pub const fn montserrat_30() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_30))
    }

    #[cfg(feature = "font-montserrat-32")]
    pub const fn montserrat_32() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_32))
    }

    #[cfg(feature = "font-montserrat-34")]
    pub const fn montserrat_34() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_34))
    }

    #[cfg(feature = "font-montserrat-36")]
    pub const fn montserrat_36() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_36))
    }

    #[cfg(feature = "font-montserrat-38")]
    pub const fn montserrat_38() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_38))
    }

    #[cfg(feature = "font-montserrat-40")]
    pub const fn montserrat_40() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_40))
    }

    #[cfg(feature = "font-montserrat-42")]
    pub const fn montserrat_42() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_42))
    }

    #[cfg(feature = "font-montserrat-44")]
    pub const fn montserrat_44() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_44))
    }

    #[cfg(feature = "font-montserrat-46")]
    pub const fn montserrat_46() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_46))
    }

    #[cfg(feature = "font-montserrat-48")]
    pub const fn montserrat_48() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_48))
    }
}

impl Default for Font {
    fn default() -> Self {
        Self::montserrat_14()
    }
}

impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Font").field(&self.0).finish()
    }
}

impl PartialEq for Font {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Font {}

/// Widget part targeted by [`Node::part_style`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Part {
    Main,
    Scrollbar,
    Indicator,
    Knob,
    Selected,
    Items,
    Cursor,
}

impl Part {
    pub(crate) fn to_raw(self) -> rs_lvgl_sys::lv_part_t {
        match self {
            Part::Main => rs_lvgl_sys::lv_part_t_LV_PART_MAIN,
            Part::Scrollbar => rs_lvgl_sys::lv_part_t_LV_PART_SCROLLBAR,
            Part::Indicator => rs_lvgl_sys::lv_part_t_LV_PART_INDICATOR,
            Part::Knob => rs_lvgl_sys::lv_part_t_LV_PART_KNOB,
            Part::Selected => rs_lvgl_sys::lv_part_t_LV_PART_SELECTED,
            Part::Items => rs_lvgl_sys::lv_part_t_LV_PART_ITEMS,
            Part::Cursor => rs_lvgl_sys::lv_part_t_LV_PART_CURSOR,
        }
    }
}

/// Widget states targeted by [`Node::state_style`]. Combine with `|`; a
/// combined state matches only while every listed state is active.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct State(u16);

impl State {
    pub const PRESSED: Self = Self(rs_lvgl_sys::lv_state_t_LV_STATE_PRESSED as u16);
    /// Set by [`Node::checked`] or by checkable LVGL widgets.
    pub const CHECKED: Self = Self(rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED as u16);
    pub const FOCUSED: Self = Self(rs_lvgl_sys::lv_state_t_LV_STATE_FOCUSED as u16);
    /// Focused by a keypad or encoder rather than a pointer.
    pub const FOCUS_KEY: Self = Self(rs_lvgl_sys::lv_state_t_LV_STATE_FOCUS_KEY as u16);
    pub const EDITED: Self = Self(rs_lvgl_sys::lv_state_t_LV_STATE_EDITED as u16);
    pub const HOVERED: Self = Self(rs_lvgl_sys::lv_state_t_LV_STATE_HOVERED as u16);
    pub const DISABLED: Self = Self(rs_lvgl_sys::lv_state_t_LV_STATE_DISABLED as u16);

    /// No state bits: the default style of a part.
    pub(crate) const DEFAULT: Self = Self(0);

    pub(crate) const fn to_raw(self) -> rs_lvgl_sys::lv_style_selector_t {
        self.0 as rs_lvgl_sys::lv_style_selector_t
    }
}

impl core::ops::BitOr for State {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// A part and state pair targeted by a node's extra styles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Selector {
    pub(crate) part: Part,
    pub(crate) state: State,
}

impl Selector {
    pub(crate) const fn new(part: Part, state: State) -> Self {
        Self { part, state }
    }

    pub(crate) fn to_raw(self) -> rs_lvgl_sys::lv_style_selector_t {
        self.part.to_raw() | self.state.to_raw()
    }
}

/// Text alignment inside a label or value widget.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextAlign {
    #[default]
    Auto,
    Left,
    Center,
    Right,
}

impl TextAlign {
    pub(crate) fn to_raw(self) -> rs_lvgl_sys::lv_text_align_t {
        match self {
            TextAlign::Auto => rs_lvgl_sys::lv_text_align_t_LV_TEXT_ALIGN_AUTO,
            TextAlign::Left => rs_lvgl_sys::lv_text_align_t_LV_TEXT_ALIGN_LEFT,
            TextAlign::Center => rs_lvgl_sys::lv_text_align_t_LV_TEXT_ALIGN_CENTER,
            TextAlign::Right => rs_lvgl_sys::lv_text_align_t_LV_TEXT_ALIGN_RIGHT,
        }
    }
}

/// A resolved size value.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Size {
    Px(i16),
    Percent(u8),
    /// Let LVGL size the object from its content.
    #[default]
    Content,
}

/// Alignment of an object inside its parent when no layout positions it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Align {
    #[default]
    TopLeft,
    TopMid,
    TopRight,
    LeftMid,
    Center,
    RightMid,
    BottomLeft,
    BottomMid,
    BottomRight,
}

impl Align {
    pub(crate) fn to_raw(self) -> rs_lvgl_sys::lv_align_t {
        match self {
            Align::TopLeft => rs_lvgl_sys::lv_align_t_LV_ALIGN_TOP_LEFT,
            Align::TopMid => rs_lvgl_sys::lv_align_t_LV_ALIGN_TOP_MID,
            Align::TopRight => rs_lvgl_sys::lv_align_t_LV_ALIGN_TOP_RIGHT,
            Align::LeftMid => rs_lvgl_sys::lv_align_t_LV_ALIGN_LEFT_MID,
            Align::Center => rs_lvgl_sys::lv_align_t_LV_ALIGN_CENTER,
            Align::RightMid => rs_lvgl_sys::lv_align_t_LV_ALIGN_RIGHT_MID,
            Align::BottomLeft => rs_lvgl_sys::lv_align_t_LV_ALIGN_BOTTOM_LEFT,
            Align::BottomMid => rs_lvgl_sys::lv_align_t_LV_ALIGN_BOTTOM_MID,
            Align::BottomRight => rs_lvgl_sys::lv_align_t_LV_ALIGN_BOTTOM_RIGHT,
        }
    }
}

/// Label overflow behavior.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LabelLongMode {
    #[default]
    Wrap,
    Dots,
    Scroll,
    ScrollCircular,
    Clip,
}

impl LabelLongMode {
    pub(crate) fn to_raw(self) -> rs_lvgl_sys::lv_label_long_mode_t {
        match self {
            LabelLongMode::Wrap => rs_lvgl_sys::lv_label_long_mode_t_LV_LABEL_LONG_MODE_WRAP,
            LabelLongMode::Dots => rs_lvgl_sys::lv_label_long_mode_t_LV_LABEL_LONG_MODE_DOTS,
            LabelLongMode::Scroll => rs_lvgl_sys::lv_label_long_mode_t_LV_LABEL_LONG_MODE_SCROLL,
            LabelLongMode::ScrollCircular => {
                rs_lvgl_sys::lv_label_long_mode_t_LV_LABEL_LONG_MODE_SCROLL_CIRCULAR
            }
            LabelLongMode::Clip => rs_lvgl_sys::lv_label_long_mode_t_LV_LABEL_LONG_MODE_CLIP,
        }
    }
}

/// Roller scrolling behavior.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RollerMode {
    /// Scrolling stops at the first and last option.
    #[default]
    Normal,
    /// Options repeat endlessly in both directions.
    Infinite,
}

#[cfg(feature = "widget-roller")]
impl RollerMode {
    pub(crate) fn to_raw(self) -> rs_lvgl_sys::lv_roller_mode_t {
        match self {
            RollerMode::Normal => rs_lvgl_sys::lv_roller_mode_t_LV_ROLLER_MODE_NORMAL,
            RollerMode::Infinite => rs_lvgl_sys::lv_roller_mode_t_LV_ROLLER_MODE_INFINITE,
        }
    }
}

/// Largest number of digits a spinbox can display.
pub const SPINBOX_MAX_DIGITS: u32 = 10;

/// Largest row or column count accepted for a table.
///
/// LVGL reports the selected cell with 16-bit indices and reserves
/// `0xFFFF` for "no cell".
pub const TABLE_MAX_CELLS_PER_AXIS: usize = 0xFFFF;

/// Decode the value a table passes to [`Node::on_value_changed`].
///
/// Tables report the selected cell packed as `(row << 16) | column`, or `-1`
/// when no cell is selected. Returns `(row, column)`.
pub fn table_cell(value: i32) -> Option<(u32, u32)> {
    if value < 0 {
        return None;
    }
    let value = value as u32;
    Some((value >> 16, value & 0xFFFF))
}

/// A set of scroll directions, combined with `|`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ScrollDir(u8);

impl ScrollDir {
    pub const NONE: Self = Self(rs_lvgl_sys::lv_dir_t_LV_DIR_NONE as u8);
    pub const LEFT: Self = Self(rs_lvgl_sys::lv_dir_t_LV_DIR_LEFT as u8);
    pub const RIGHT: Self = Self(rs_lvgl_sys::lv_dir_t_LV_DIR_RIGHT as u8);
    pub const TOP: Self = Self(rs_lvgl_sys::lv_dir_t_LV_DIR_TOP as u8);
    pub const BOTTOM: Self = Self(rs_lvgl_sys::lv_dir_t_LV_DIR_BOTTOM as u8);
    pub const HOR: Self = Self(rs_lvgl_sys::lv_dir_t_LV_DIR_HOR as u8);
    pub const VER: Self = Self(rs_lvgl_sys::lv_dir_t_LV_DIR_VER as u8);
    pub const ALL: Self = Self(rs_lvgl_sys::lv_dir_t_LV_DIR_ALL as u8);

    pub(crate) fn to_raw(self) -> rs_lvgl_sys::lv_dir_t {
        self.0 as rs_lvgl_sys::lv_dir_t
    }
}

impl core::ops::BitOr for ScrollDir {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// One tile of a [`crate::tileview`]: its grid position and the directions
/// the user may swipe away from it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tile {
    pub column: u8,
    pub row: u8,
    pub dir: ScrollDir,
}

impl Tile {
    pub const fn new(column: u8, row: u8, dir: ScrollDir) -> Self {
        Self { column, row, dir }
    }
}

/// A calendar date. Months and days start at 1.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CalendarDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl CalendarDate {
    pub const fn new(year: u16, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }

    /// Decode the value a calendar passes to [`Node::on_value_changed`].
    ///
    /// Calendars report the pressed date packed as
    /// `year * 10000 + month * 100 + day`, or `-1` when no date is pressed.
    pub fn from_value(value: i32) -> Option<Self> {
        if value <= 0 {
            return None;
        }
        let year = value / 10_000;
        let month = (value / 100) % 100;
        let day = value % 100;
        if year > i32::from(u16::MAX) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return None;
        }
        Some(Self::new(year as u16, month as u8, day as u8))
    }

    /// The packed form used by [`CalendarDate::from_value`].
    pub fn to_value(self) -> i32 {
        i32::from(self.year) * 10_000 + i32::from(self.month) * 100 + i32::from(self.day)
    }

    pub(crate) fn is_valid(self) -> bool {
        (1..=12).contains(&self.month) && (1..=31).contains(&self.day)
    }

    #[cfg(feature = "widget-calendar")]
    pub(crate) fn to_raw(self) -> rs_lvgl_sys::lv_calendar_date_t {
        rs_lvgl_sys::lv_calendar_date_t {
            year: self.year,
            month: self.month,
            day: self.day,
        }
    }
}

/// One styled run of text in a [`crate::spans`] group.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Span {
    pub text: String,
    /// Text color; `None` inherits the span group's color.
    pub color: Option<Color>,
    /// Text font; `None` inherits the span group's font.
    pub font: Option<Font>,
}

impl Span {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: None,
            font: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }
}

/// Repeat count that makes an animated image loop forever.
pub const ANIM_REPEAT_INFINITE: u32 = rs_lvgl_sys::LV_ANIM_REPEAT_INFINITE;

/// Largest number of frames an animated image accepts (LVGL stores the
/// frame count in an `int8_t`).
pub const ANIM_IMAGE_MAX_FRAMES: usize = 127;

/// A child identity used for sibling reconciliation.
///
/// `Auto` children match by position among `Auto` siblings. `Named` and `Id`
/// children match by value, which preserves native object identity across
/// inserts, removals and reorders.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Key {
    #[default]
    Auto,
    Named(&'static str),
    Id(u64),
}

/// Where a static image comes from.
#[derive(Clone, Copy)]
pub enum ImageSource {
    /// A null-terminated LVGL path or symbol string that remains valid for the
    /// program's lifetime.
    Text(&'static CStr),
    /// A static RGB565 image. The descriptor is built when the node is
    /// mounted and kept alive by the mounted tree.
    Rgb565 {
        width: u16,
        height: u16,
        data: &'static [u8],
    },
    /// A raw LVGL image descriptor, only constructible through the unsafe
    /// [`ImageSource::from_raw`].
    Raw(RawImage),
}

/// An LVGL image descriptor pointer whose validity was asserted by
/// [`ImageSource::from_raw`].
///
/// The pointer field is private so a safe caller cannot construct a raw
/// image source with an arbitrary pointer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RawImage(*const rs_lvgl_sys::lv_image_dsc_t);

impl RawImage {
    /// The wrapped descriptor pointer.
    pub const fn as_ptr(self) -> *const rs_lvgl_sys::lv_image_dsc_t {
        self.0
    }
}

impl ImageSource {
    /// Wrap an LVGL image descriptor.
    ///
    /// # Safety
    ///
    /// The descriptor and its pixel data must stay valid and immutable for as
    /// long as an image object references them, including after this `Node` has
    /// been reconciled away.
    pub const unsafe fn from_raw(raw: *const rs_lvgl_sys::lv_image_dsc_t) -> Self {
        Self::Raw(RawImage(raw))
    }

    /// A static RGB565 image. `data` must contain `width * height * 2` bytes.
    pub const fn rgb565(width: u16, height: u16, data: &'static [u8]) -> Self {
        Self::Rgb565 {
            width,
            height,
            data,
        }
    }

    /// Build the LVGL descriptor for a static RGB565 source.
    pub(crate) fn descriptor(self) -> Option<Box<rs_lvgl_sys::lv_image_dsc_t>> {
        let ImageSource::Rgb565 {
            width,
            height,
            data,
        } = self
        else {
            return None;
        };
        let expected = width as usize * height as usize * 2;
        if width == 0 || height == 0 || data.len() != expected {
            return None;
        }
        let mut descriptor = rs_lvgl_sys::lv_image_dsc_t::default();
        descriptor
            .header
            .set_magic(rs_lvgl_sys::LV_IMAGE_HEADER_MAGIC);
        descriptor
            .header
            .set_cf(rs_lvgl_sys::lv_color_format_t_LV_COLOR_FORMAT_RGB565);
        descriptor.header.set_w(width as u32);
        descriptor.header.set_h(height as u32);
        descriptor.header.set_stride(width as u32 * 2);
        descriptor.data_size = expected as u32;
        descriptor.data = data.as_ptr();
        Some(Box::new(descriptor))
    }

    pub(crate) fn ptr(
        self,
        descriptor: Option<&rs_lvgl_sys::lv_image_dsc_t>,
    ) -> *const cty::c_void {
        match self {
            ImageSource::Text(text) => text.as_ptr().cast(),
            ImageSource::Rgb565 { .. } => descriptor
                .map(|descriptor| descriptor as *const _ as *const cty::c_void)
                .unwrap_or(core::ptr::null()),
            ImageSource::Raw(raw) => raw.as_ptr().cast(),
        }
    }
}

impl PartialEq for ImageSource {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ImageSource::Text(left), ImageSource::Text(right)) => left.as_ptr() == right.as_ptr(),
            (
                ImageSource::Rgb565 {
                    width: left_width,
                    height: left_height,
                    data: left_data,
                },
                ImageSource::Rgb565 {
                    width: right_width,
                    height: right_height,
                    data: right_data,
                },
            ) => {
                left_width == right_width
                    && left_height == right_height
                    && left_data.as_ptr() == right_data.as_ptr()
                    && left_data.len() == right_data.len()
            }
            (ImageSource::Raw(left), ImageSource::Raw(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for ImageSource {}

impl fmt::Debug for ImageSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageSource::Text(text) => f.debug_tuple("Text").field(text).finish(),
            ImageSource::Rgb565 {
                width,
                height,
                data,
            } => f
                .debug_struct("Rgb565")
                .field("width", width)
                .field("height", height)
                .field("bytes", &data.len())
                .finish(),
            ImageSource::Raw(raw) => f.debug_tuple("Raw").field(&raw.as_ptr()).finish(),
        }
    }
}

/// One grid track size.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GridTrack {
    Px(i16),
    /// Proportional share of the free space; must be at least 1.
    Fr(u8),
    /// Size to the largest child on the track.
    Content,
}

impl GridTrack {
    pub(crate) fn to_raw(self) -> i32 {
        // LV_COORD_MAX is (1 << 29) - 1; LVGL encodes FR/Content relative to it.
        const COORD_MAX: i32 = 0x1fff_ffff;
        match self {
            GridTrack::Px(value) => value as i32,
            GridTrack::Fr(value) => COORD_MAX - 100 + value as i32,
            GridTrack::Content => COORD_MAX - 101,
        }
    }
}

/// Placement of the children of a `row` or `column` along one axis.
///
/// The space-distributing variants only make sense on the main axis; on the
/// cross axis LVGL treats them as [`FlexAlign::Start`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FlexAlign {
    #[default]
    Start,
    Center,
    End,
    SpaceEvenly,
    SpaceAround,
    SpaceBetween,
}

impl FlexAlign {
    pub(crate) fn to_raw(self) -> rs_lvgl_sys::lv_flex_align_t {
        match self {
            FlexAlign::Start => rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_START,
            FlexAlign::Center => rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_CENTER,
            FlexAlign::End => rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_END,
            FlexAlign::SpaceEvenly => rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_SPACE_EVENLY,
            FlexAlign::SpaceAround => rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_SPACE_AROUND,
            FlexAlign::SpaceBetween => rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_SPACE_BETWEEN,
        }
    }
}

/// Alignment of a child inside its grid cell.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GridAlign {
    Start,
    Center,
    End,
    #[default]
    Stretch,
}

impl GridAlign {
    pub(crate) fn to_raw(self) -> rs_lvgl_sys::lv_grid_align_t {
        match self {
            GridAlign::Start => rs_lvgl_sys::lv_grid_align_t_LV_GRID_ALIGN_START,
            GridAlign::Center => rs_lvgl_sys::lv_grid_align_t_LV_GRID_ALIGN_CENTER,
            GridAlign::End => rs_lvgl_sys::lv_grid_align_t_LV_GRID_ALIGN_END,
            GridAlign::Stretch => rs_lvgl_sys::lv_grid_align_t_LV_GRID_ALIGN_STRETCH,
        }
    }
}

/// Placement of one child inside a [`WidgetKind::Grid`] parent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GridCell {
    pub column: u16,
    pub row: u16,
    pub column_span: u16,
    pub row_span: u16,
    pub column_align: GridAlign,
    pub row_align: GridAlign,
}

impl GridCell {
    pub fn new(column: u16, row: u16, column_span: u16, row_span: u16) -> Self {
        Self {
            column,
            row,
            column_span,
            row_span,
            column_align: GridAlign::Stretch,
            row_align: GridAlign::Start,
        }
    }
}

/// Chart presentation type.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ChartType {
    #[default]
    Line,
    Bar,
    Stacked,
}

#[cfg(feature = "widget-chart")]
impl ChartType {
    pub(crate) fn to_raw(self) -> rs_lvgl_sys::lv_chart_type_t {
        match self {
            ChartType::Line => rs_lvgl_sys::lv_chart_type_t_LV_CHART_TYPE_LINE,
            ChartType::Bar => rs_lvgl_sys::lv_chart_type_t_LV_CHART_TYPE_BAR,
            ChartType::Stacked => rs_lvgl_sys::lv_chart_type_t_LV_CHART_TYPE_STACKED,
        }
    }
}

/// One chart series: a color and one value per point.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChartSeries {
    pub color: Color,
    pub values: Vec<i32>,
}

impl ChartSeries {
    pub fn new(color: Color, values: impl Into<Vec<i32>>) -> Self {
        Self {
            color,
            values: values.into(),
        }
    }
}

/// The native widget described by a [`Node`].
///
/// The enum is non-exhaustive: new LVGL widgets are added over time.
#[non_exhaustive]
pub enum WidgetKind {
    #[cfg(feature = "widget-ime-pinyin")]
    PinyinKeyboard {
        target: u64,
        font: Font,
    },
    #[cfg(feature = "widget-qrcode")]
    QrCode {
        code: crate::QrCode,
    },
    #[cfg(feature = "widget-barcode")]
    Barcode {
        code: crate::Barcode,
    },
    #[cfg(feature = "widget-gif")]
    Gif {
        animation: crate::Gif,
    },
    #[cfg(feature = "widget-lottie")]
    Lottie {
        animation: crate::Lottie,
    },

    #[cfg(feature = "widget-canvas")]
    Canvas {
        frame: crate::Canvas,
    },
    Obj,
    Row,
    Column,
    Grid {
        columns: Vec<GridTrack>,
        rows: Vec<GridTrack>,
    },
    /// A native LVGL tabview. Its children are the tab page roots, one per
    /// title in `tabs`.
    #[cfg(feature = "widget-tabview")]
    Tabview {
        tabs: Vec<String>,
        active: u32,
    },
    Label {
        text: String,
        long_mode: LabelLongMode,
    },
    Button {
        text: String,
    },
    #[cfg(feature = "widget-slider")]
    Slider {
        min: i32,
        max: i32,
        value: i32,
    },
    #[cfg(feature = "widget-bar")]
    Bar {
        min: i32,
        max: i32,
        value: i32,
    },
    #[cfg(feature = "widget-switch")]
    Switch {
        checked: bool,
    },
    #[cfg(feature = "widget-image")]
    Image {
        source: ImageSource,
        /// LVGL zoom factor: 256 is 100%.
        scale: u16,
    },
    #[cfg(feature = "widget-checkbox")]
    Checkbox {
        text: String,
        checked: bool,
    },
    #[cfg(feature = "widget-textarea")]
    Textarea {
        text: String,
        placeholder: String,
        one_line: bool,
        password: bool,
    },
    #[cfg(feature = "widget-dropdown")]
    Dropdown {
        options: Vec<String>,
        selected: u32,
    },
    /// A gauge-style arc: the background arc and knob are hidden, and the
    /// indicator uses the style's `arc_color`/`arc_width` when set.
    #[cfg(feature = "widget-arc")]
    Arc {
        min: i32,
        max: i32,
        value: i32,
    },
    #[cfg(feature = "widget-scale")]
    Scale {
        min: i32,
        max: i32,
        total_ticks: u32,
        major_tick_every: u32,
    },
    #[cfg(feature = "widget-chart")]
    Chart {
        chart_type: ChartType,
        /// Primary Y-axis minimum.
        min: i32,
        /// Primary Y-axis maximum.
        max: i32,
        series: Vec<ChartSeries>,
    },
    /// A keyboard bound to the textarea registered with `keyboard_id`.
    #[cfg(feature = "widget-keyboard")]
    Keyboard {
        target: u64,
    },
    /// A scrollable option list; the selected option is controlled.
    #[cfg(feature = "widget-roller")]
    Roller {
        options: Vec<String>,
        selected: u32,
        visible_rows: u32,
        mode: RollerMode,
    },
    /// A numeric field. `value` is an integer; `decimal_pos` only places the
    /// decimal separator when displaying it.
    #[cfg(feature = "widget-spinbox")]
    Spinbox {
        min: i32,
        max: i32,
        value: i32,
        digit_count: u32,
        decimal_pos: u32,
        step: u32,
    },
    /// An animated busy indicator.
    #[cfg(feature = "widget-spinner")]
    Spinner {
        duration_ms: u32,
        arc_angle: u32,
    },
    /// A light-emitting diode indicator.
    #[cfg(feature = "widget-led")]
    Led {
        color: Color,
        brightness: u8,
    },
    /// A polyline through `points`, in pixels relative to the widget.
    #[cfg(feature = "widget-line")]
    Line {
        points: Vec<(i32, i32)>,
    },
    /// A grid of text buttons, one inner vector per row.
    #[cfg(feature = "widget-buttonmatrix")]
    ButtonMatrix {
        rows: Vec<Vec<String>>,
    },
    /// A text table, one inner vector per row. Every row has the same number
    /// of cells.
    #[cfg(feature = "widget-table")]
    Table {
        cells: Vec<Vec<String>>,
        column_widths: Vec<i32>,
    },
    /// A swipeable grid of full-size tiles. Its children are the tile roots,
    /// one per entry in `tiles`.
    #[cfg(feature = "widget-tileview")]
    Tileview {
        tiles: Vec<Tile>,
        active: u32,
    },
    /// An inline message box: an optional title, a text, optional footer
    /// buttons, and children placed below the text in its content area.
    #[cfg(feature = "widget-msgbox")]
    Msgbox {
        title: Option<String>,
        text: String,
        buttons: Vec<String>,
    },
    /// A month calendar. The shown month is controlled by the application.
    #[cfg(feature = "widget-calendar")]
    Calendar {
        year: u16,
        month: u8,
        today: Option<CalendarDate>,
        highlighted: Vec<CalendarDate>,
    },
    /// Rich text made of individually styled spans.
    #[cfg(feature = "widget-span")]
    Spans {
        spans: Vec<Span>,
    },
    /// A button drawn from images.
    #[cfg(feature = "widget-imagebutton")]
    ImageButton {
        released: ImageSource,
        pressed: Option<ImageSource>,
    },
    /// An image that cycles through `frames`.
    #[cfg(feature = "widget-animimage")]
    AnimImage {
        frames: Vec<ImageSource>,
        duration_ms: u32,
        repeat: u32,
    },
    /// Text laid out along an arc.
    #[cfg(feature = "widget-arclabel")]
    ArcLabel {
        text: String,
        radius: u32,
        start_angle: i32,
        angle_size: i32,
    },
    /// An opaque native subtree: either a static `crate::raw` factory or a
    /// lifecycle widget supplied to `crate::custom`.
    Custom(CustomWidgetKind),
}

/// Boxed value-changed mapping registered with [`Node::on_value_changed`].
pub type ValueChangedHandler<M> = Box<dyn FnMut(i32) -> M>;

/// Boxed event mapping registered with [`Node::on_event`].
pub type EventHandler<M> = Box<dyn FnMut(&crate::Event) -> Option<M>>;

/// Boxed text-changed mapping registered with [`Node::on_text_changed`].
pub type TextChangedHandler<M> = Box<dyn FnMut(&str) -> M>;

pub(crate) type BubbledHandler<M> = Box<dyn FnMut(&crate::InputEvent) -> Option<M>>;

/// Legacy single-handler representation.
///
/// Nodes no longer store this enum: each `Node::on_*` builder now sets an
/// independent handler, so `on_click` and `on_event` can be combined. The type
/// stays exported for source compatibility and is not read by the runtime.
#[derive(Default)]
pub enum Handlers<M> {
    #[default]
    None,
    Click(M),
    ValueChanged(ValueChangedHandler<M>),
    TextChanged(TextChangedHandler<M>),
    Event(EventHandler<M>),
}

/// Handlers retained by one mounted node. Every handler is independent.
pub(crate) struct HandlerSet<M> {
    pub(crate) click: Option<M>,
    pub(crate) value_changed: Option<ValueChangedHandler<M>>,
    pub(crate) text_changed: Option<TextChangedHandler<M>>,
    pub(crate) event: Option<EventHandler<M>>,
    pub(crate) key: Option<Box<dyn FnMut(u32) -> M>>,
    pub(crate) rotary: Option<Box<dyn FnMut(i32) -> M>>,
    pub(crate) gesture: Option<Box<dyn FnMut(crate::GestureDirection) -> M>>,
    pub(crate) scroll: Option<Box<dyn FnMut(crate::ScrollOffset) -> M>>,
    pub(crate) pointer: Option<Box<dyn FnMut(crate::PointerEvent) -> M>>,
    pub(crate) long_press: Option<M>,
    pub(crate) bubbled: Option<BubbledHandler<M>>,
}

impl<M> Default for HandlerSet<M> {
    fn default() -> Self {
        Self {
            click: None,
            value_changed: None,
            text_changed: None,
            event: None,
            key: None,
            rotary: None,
            gesture: None,
            scroll: None,
            pointer: None,
            long_press: None,
            bubbled: None,
        }
    }
}

impl<M> HandlerSet<M> {
    pub(crate) fn has_input(&self) -> bool {
        self.key.is_some()
            || self.rotary.is_some()
            || self.gesture.is_some()
            || self.scroll.is_some()
            || self.pointer.is_some()
            || self.long_press.is_some()
            || self.bubbled.is_some()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.click.is_none()
            && self.value_changed.is_none()
            && self.text_changed.is_none()
            && self.event.is_none()
            && !self.has_input()
    }
}

/// The backing descriptor type for [`crate::View`].
///
/// Constructing a descriptor never touches LVGL. Application and component
/// functions return [`crate::View`] and use ordinary constructors such as
/// [`crate::column`], [`crate::label`] and [`crate::button`].
pub struct Node<M> {
    pub(crate) key: Key,
    pub(crate) kind: WidgetKind,
    pub(crate) style: Style,
    pub(crate) part_styles: Vec<(Selector, Style)>,
    pub(crate) keyboard_id: Option<u64>,
    pub(crate) grid_cell: Option<GridCell>,
    pub(crate) children: Vec<Node<M>>,
    pub(crate) handlers: HandlerSet<M>,
    pub(crate) clickable: Option<bool>,
    pub(crate) scrollable: Option<bool>,
    pub(crate) checked: Option<bool>,
    pub(crate) interaction: crate::scroll::Interaction,
    pub(crate) animations: Vec<crate::Animation>,
    #[cfg(feature = "input-keypad")]
    pub(crate) focusable: Option<bool>,
    #[cfg(feature = "input-keypad")]
    pub(crate) modal: bool,
    #[cfg(feature = "input-gridnav")]
    pub(crate) grid_navigation: Option<crate::GridNavigation>,
}

impl<M> Node<M> {
    pub(crate) fn new(kind: WidgetKind) -> Self {
        let style = match &kind {
            WidgetKind::Obj
            | WidgetKind::Row
            | WidgetKind::Column
            | WidgetKind::Grid { .. }
            | WidgetKind::Label { .. } => Style::neutral(),
            #[cfg(feature = "widget-span")]
            WidgetKind::Spans { .. } => Style::neutral(),
            #[cfg(feature = "widget-arclabel")]
            WidgetKind::ArcLabel { .. } => Style::neutral(),
            _ => Style::default(),
        };
        Self {
            key: Key::Auto,
            kind,
            style,
            part_styles: Vec::new(),
            keyboard_id: None,
            grid_cell: None,
            children: Vec::new(),
            handlers: HandlerSet::default(),
            clickable: None,
            scrollable: None,
            checked: None,
            interaction: crate::scroll::Interaction::default(),
            animations: Vec::new(),
            #[cfg(feature = "input-keypad")]
            focusable: None,
            #[cfg(feature = "input-keypad")]
            modal: false,
            #[cfg(feature = "input-gridnav")]
            grid_navigation: None,
        }
    }

    #[cfg(feature = "widget-textarea")]
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        if let WidgetKind::Textarea { placeholder, .. } = &mut self.kind {
            *placeholder = text.into();
        }
        self
    }

    #[cfg(feature = "widget-textarea")]
    pub fn password(mut self, enabled: bool) -> Self {
        if let WidgetKind::Textarea { password, .. } = &mut self.kind {
            *password = enabled;
        }
        self
    }

    #[cfg(feature = "widget-textarea")]
    pub fn multiline(mut self) -> Self {
        if let WidgetKind::Textarea { one_line, .. } = &mut self.kind {
            *one_line = false;
        }
        self
    }

    /// Register this textarea as a keyboard target.
    ///
    /// IDs are application-scoped; the reconciler resolves them after each
    /// render. Only textareas may carry a keyboard id.
    pub fn keyboard_id(mut self, id: u64) -> Self {
        self.keyboard_id = Some(id);
        self
    }

    /// Number of rows a roller shows. Ignored for other kinds.
    #[cfg(feature = "widget-roller")]
    pub fn visible_rows(mut self, rows: u32) -> Self {
        if let WidgetKind::Roller { visible_rows, .. } = &mut self.kind {
            *visible_rows = rows;
        }
        self
    }

    /// Scrolling behavior of a roller. Ignored for other kinds.
    #[cfg(feature = "widget-roller")]
    pub fn roller_mode(mut self, roller_mode: RollerMode) -> Self {
        if let WidgetKind::Roller { mode, .. } = &mut self.kind {
            *mode = roller_mode;
        }
        self
    }

    /// Digit count and decimal separator position of a spinbox.
    ///
    /// `decimal_pos` is the number of integer digits before the separator;
    /// `0` hides it. Ignored for other kinds.
    #[cfg(feature = "widget-spinbox")]
    pub fn digits(mut self, count: u32, decimal_position: u32) -> Self {
        if let WidgetKind::Spinbox {
            digit_count,
            decimal_pos,
            ..
        } = &mut self.kind
        {
            *digit_count = count;
            *decimal_pos = decimal_position;
        }
        self
    }

    /// Increment applied by LVGL's spinbox key handling. Ignored for other
    /// kinds.
    #[cfg(feature = "widget-spinbox")]
    pub fn step(mut self, value: u32) -> Self {
        if let WidgetKind::Spinbox { step, .. } = &mut self.kind {
            *step = value;
        }
        self
    }

    /// Spinner animation parameters. Ignored for other kinds.
    #[cfg(feature = "widget-spinner")]
    pub fn spinner_anim(mut self, duration: u32, angle: u32) -> Self {
        if let WidgetKind::Spinner {
            duration_ms,
            arc_angle,
        } = &mut self.kind
        {
            *duration_ms = duration;
            *arc_angle = angle;
        }
        self
    }

    /// LED brightness. LVGL clamps it to `80..=255`; `80` looks off.
    /// Ignored for other kinds.
    #[cfg(feature = "widget-led")]
    pub fn brightness(mut self, value: u8) -> Self {
        if let WidgetKind::Led { brightness, .. } = &mut self.kind {
            *brightness = value;
        }
        self
    }

    /// Table column widths in pixels. Columns without an entry keep their
    /// current width (LVGL's default unless set before). Ignored for other
    /// kinds.
    #[cfg(feature = "widget-table")]
    pub fn column_widths(mut self, widths: impl IntoIterator<Item = i32>) -> Self {
        if let WidgetKind::Table { column_widths, .. } = &mut self.kind {
            *column_widths = widths.into_iter().collect();
        }
        self
    }

    /// Message box title. Ignored for other kinds.
    #[cfg(feature = "widget-msgbox")]
    pub fn msgbox_title(mut self, text: impl Into<String>) -> Self {
        if let WidgetKind::Msgbox { title, .. } = &mut self.kind {
            *title = Some(text.into());
        }
        self
    }

    /// Message box footer buttons. A click reports the button index through
    /// [`Node::on_value_changed`]. Ignored for other kinds.
    #[cfg(feature = "widget-msgbox")]
    pub fn msgbox_buttons(mut self, texts: impl IntoIterator<Item = impl Into<String>>) -> Self {
        if let WidgetKind::Msgbox { buttons, .. } = &mut self.kind {
            *buttons = texts.into_iter().map(Into::into).collect();
        }
        self
    }

    /// The date marked as today. Ignored for other kinds.
    #[cfg(feature = "widget-calendar")]
    pub fn today(mut self, date: CalendarDate) -> Self {
        if let WidgetKind::Calendar { today, .. } = &mut self.kind {
            *today = Some(date);
        }
        self
    }

    /// Dates drawn highlighted. Ignored for other kinds.
    #[cfg(feature = "widget-calendar")]
    pub fn highlighted(mut self, dates: impl IntoIterator<Item = CalendarDate>) -> Self {
        if let WidgetKind::Calendar { highlighted, .. } = &mut self.kind {
            *highlighted = dates.into_iter().collect();
        }
        self
    }

    /// Image button picture while pressed. Ignored for other kinds.
    #[cfg(feature = "widget-imagebutton")]
    pub fn pressed_image(mut self, source: ImageSource) -> Self {
        if let WidgetKind::ImageButton { pressed, .. } = &mut self.kind {
            *pressed = Some(source);
        }
        self
    }

    /// Animated image repeat count. Ignored for other kinds.
    #[cfg(feature = "widget-animimage")]
    pub fn repeat(mut self, count: u32) -> Self {
        if let WidgetKind::AnimImage { repeat, .. } = &mut self.kind {
            *repeat = count;
        }
        self
    }

    pub fn key(mut self, key: Key) -> Self {
        self.key = key;
        self
    }

    /// Merge the fields set by `style` over the node's kind base.
    ///
    /// Unset fields keep the kind base (or the LVGL class default for
    /// interactive widgets).
    pub fn style(mut self, style: Style) -> Self {
        self.style = self.style.merged(style);
        self
    }

    /// Apply a style to one widget part (for example a slider indicator).
    ///
    /// Part styles are applied after the main style and override it on that
    /// part. Replacing or removing a part style restores the class defaults for
    /// that part when the view updates.
    pub fn part_style(mut self, part: Part, style: Style) -> Self {
        self.part_styles
            .push((Selector::new(part, State::DEFAULT), style));
        self
    }

    /// Apply a style to the main part while `state` is active (for example
    /// a pressed or checked list item).
    ///
    /// Object-wide properties (`hidden`, `disabled`, `gap`, `floating`) are
    /// ignored in state styles. Replacing or removing a state style restores
    /// the class defaults for that state when the view updates.
    pub fn state_style(self, state: State, style: Style) -> Self {
        self.part_state_style(Part::Main, state, style)
    }

    /// Apply a style to one widget part while `state` is active.
    pub fn part_state_style(mut self, part: Part, state: State, style: Style) -> Self {
        self.part_styles.push((Selector::new(part, state), style));
        self
    }

    /// Place this node in a grid parent.
    pub fn grid_cell(mut self, column: u16, row: u16, column_span: u16, row_span: u16) -> Self {
        self.grid_cell = Some(GridCell::new(column, row, column_span, row_span));
        self
    }

    /// Override the alignment inside this node's grid cell.
    pub fn grid_align(mut self, column: GridAlign, row: GridAlign) -> Self {
        if let Some(cell) = &mut self.grid_cell {
            cell.column_align = column;
            cell.row_align = row;
        }
        self
    }

    pub fn child(mut self, child: Node<M>) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = Node<M>>) -> Self {
        self.children.extend(children);
        self
    }

    /// Convert every message this subtree produces with `map`.
    ///
    /// This is how a component with its own message type is embedded in an
    /// application: `menu::view(&state).map(Msg::Menu)`. Click messages are
    /// converted once, when `map` is called; value, text and event mappings
    /// are converted when the event fires. `map` must be a pure conversion.
    pub fn map<N: 'static>(self, map: impl Fn(M) -> N + 'static) -> Node<N>
    where
        M: 'static,
    {
        let map: Rc<dyn Fn(M) -> N> = Rc::new(map);
        self.map_with(&map)
    }

    fn map_with<N: 'static>(self, map: &Rc<dyn Fn(M) -> N>) -> Node<N>
    where
        M: 'static,
    {
        let HandlerSet {
            click,
            value_changed,
            text_changed,
            event,
            key,
            rotary,
            gesture,
            scroll,
            pointer,
            long_press,
            bubbled,
        } = self.handlers;
        let handlers = HandlerSet {
            key: key.map(|mut handler| {
                let map = map.clone();
                Box::new(move |value| map(handler(value))) as Box<dyn FnMut(u32) -> N>
            }),
            rotary: rotary.map(|mut handler| {
                let map = map.clone();
                Box::new(move |value| map(handler(value))) as Box<dyn FnMut(i32) -> N>
            }),
            gesture: gesture.map(|mut handler| {
                let map = map.clone();
                Box::new(move |value| map(handler(value)))
                    as Box<dyn FnMut(crate::GestureDirection) -> N>
            }),
            scroll: scroll.map(|mut handler| {
                let map = map.clone();
                Box::new(move |value| map(handler(value)))
                    as Box<dyn FnMut(crate::ScrollOffset) -> N>
            }),
            pointer: pointer.map(|mut handler| {
                let map = map.clone();
                Box::new(move |value| map(handler(value)))
                    as Box<dyn FnMut(crate::PointerEvent) -> N>
            }),
            long_press: long_press.map(|msg| map(msg)),
            bubbled: bubbled.map(|mut handler| {
                let map = map.clone();
                Box::new(move |event: &crate::InputEvent| handler(event).map(|msg| map(msg)))
                    as BubbledHandler<N>
            }),
            click: click.map(|msg| map(msg)),
            value_changed: value_changed.map(|mut handler| {
                let map = map.clone();
                Box::new(move |value| map(handler(value))) as ValueChangedHandler<N>
            }),
            text_changed: text_changed.map(|mut handler| {
                let map = map.clone();
                Box::new(move |text: &str| map(handler(text))) as TextChangedHandler<N>
            }),
            event: event.map(|mut handler| {
                let map = map.clone();
                Box::new(move |event: &crate::Event| handler(event).map(|msg| map(msg)))
                    as EventHandler<N>
            }),
        };
        Node {
            key: self.key,
            kind: self.kind,
            style: self.style,
            part_styles: self.part_styles,
            keyboard_id: self.keyboard_id,
            grid_cell: self.grid_cell,
            children: self
                .children
                .into_iter()
                .map(|child| child.map_with(map))
                .collect(),
            handlers,
            clickable: self.clickable,
            scrollable: self.scrollable,
            checked: self.checked,
            interaction: self.interaction,
            animations: self.animations,
            #[cfg(feature = "input-keypad")]
            focusable: self.focusable,
            #[cfg(feature = "input-keypad")]
            modal: self.modal,
            #[cfg(feature = "input-gridnav")]
            grid_navigation: self.grid_navigation,
        }
    }

    /// Control `LV_STATE_CHECKED`, which selects [`State::CHECKED`] styles.
    ///
    /// The state is controlled: it is compared with the native object on
    /// every render. Left unset, the state is not touched. Switches and
    /// checkboxes ignore this; their checked value is part of the widget.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    /// Set LVGL's `LV_OBJ_FLAG_CLICKABLE` explicitly.
    ///
    /// Left unset, the widget keeps LVGL's default for its class (plain
    /// objects and containers are clickable). Handlers never change the flag.
    pub fn clickable(mut self, clickable: bool) -> Self {
        self.clickable = Some(clickable);
        self
    }

    /// Set LVGL's `LV_OBJ_FLAG_SCROLLABLE` explicitly.
    ///
    /// Left unset, the widget keeps LVGL's default for its class (plain
    /// objects and containers are scrollable when content overflows).
    pub fn scrollable(mut self, scrollable: bool) -> Self {
        self.scrollable = Some(scrollable);
        self
    }

    /// Override the native class's default focus-group membership. Hidden
    /// or disabled nodes (including descendants of such nodes) are excluded.
    /// Unset restores the class default on the next render.
    #[cfg(feature = "input-keypad")]
    pub fn focusable(mut self, enabled: bool) -> Self {
        self.focusable = Some(enabled);
        self
    }

    /// Restrict keyboard/encoder focus to this subtree while it is visible and
    /// enabled. The last modal in view/overlay preorder wins. Closing restores
    /// the previous mounted node when still eligible, including auto-keyed nodes.
    /// Pointer blocking requires a backdrop (provided by components::Dialog).
    /// Has no effect until the UI's focus group is explicitly activated.
    #[cfg(feature = "input-keypad")]
    pub fn modal(mut self, enabled: bool) -> Self {
        self.modal = enabled;
        self
    }

    /// Navigate the direct children of an Obj/Row/Column/Grid with arrow keys.
    /// The container occupies one Tab position after explicit focus activation.
    /// Nested navigation or modal scopes inside a grid are rejected; place
    /// independent grids/modals in sibling containers or in App::overlay.
    #[cfg(feature = "input-gridnav")]
    pub fn grid_navigation(mut self, navigation: crate::GridNavigation) -> Self {
        self.grid_navigation = Some(navigation);
        self
    }

    /// Enable native event propagation to the parent. Use on_bubbled_event on ancestors.
    /// Unset leaves the existing native setting unchanged.
    pub fn event_bubble(mut self, value: bool) -> Self {
        self.interaction.event_bubble = Some(value);
        self
    }

    /// Let LVGL choose a gesture target above this node, following its gesture-bubble flags.
    /// Unset leaves the existing native setting unchanged.
    pub fn gesture_bubble(mut self, value: bool) -> Self {
        self.interaction.gesture_bubble = Some(value);
        self
    }

    /// Override allowed native scroll directions.
    /// Unset leaves the existing native setting unchanged.
    pub fn scroll_dir(mut self, value: ScrollDir) -> Self {
        self.interaction.dir = Some(value);
        self
    }

    /// Set horizontal snapping alignment for snappable children.
    /// Unset leaves the existing native setting unchanged.
    pub fn scroll_snap_x(mut self, value: crate::ScrollSnap) -> Self {
        self.interaction.snap_x = Some(value);
        self
    }

    /// Set vertical snapping alignment for snappable children.
    /// Unset leaves the existing native setting unchanged.
    pub fn scroll_snap_y(mut self, value: crate::ScrollSnap) -> Self {
        self.interaction.snap_y = Some(value);
        self
    }

    /// Override native scrollbar visibility.
    /// Unset leaves the existing native setting unchanged.
    pub fn scrollbar_mode(mut self, value: crate::ScrollbarMode) -> Self {
        self.interaction.scrollbar = Some(value);
        self
    }

    /// Enable or disable native momentum scrolling.
    /// Unset leaves the existing native setting unchanged.
    pub fn scroll_momentum(mut self, value: bool) -> Self {
        self.interaction.momentum = Some(value);
        self
    }

    /// Enable or disable native elastic scrolling.
    /// Unset leaves the existing native setting unchanged.
    pub fn scroll_elastic(mut self, value: bool) -> Self {
        self.interaction.elastic = Some(value);
        self
    }

    /// Control the absolute scroll offset in pixels after layout. LVGL
    /// clips it to content bounds. Unset releases control. Native scroll
    /// events, including programmatic ones, still reach on_scroll.
    /// Values outside LVGL's LV_COORD_MIN..=LV_COORD_MAX fail validation.
    pub fn scroll_to(mut self, x: i32, y: i32) -> Self {
        self.interaction.offset = Some(crate::ScrollOffset { x, y });
        self
    }

    /// Enqueue `msg` when the node is clicked.
    ///
    /// Messages are cloned for every click, so `M` must implement `Clone`.
    /// Handlers are independent: this can be combined with the other `on_*`
    /// builders; calling the same builder twice replaces that handler only.
    pub fn on_click(mut self, msg: M) -> Self
    where
        M: Clone,
    {
        self.handlers.click = Some(msg);
        self
    }

    /// Map a value-changed event to a message.
    ///
    /// The value is read from the native widget after the change: the value
    /// of sliders, bars and arcs; `1`/`0` for checked/unchecked switches and
    /// checkboxes; the selected index of dropdowns and rollers; the value of
    /// spinboxes; the active tab index of tabviews; the active tile index of
    /// tileviews (`-1` before any tile is active); the pressed date of
    /// calendars packed by [`CalendarDate::to_value`] (`-1` when none); the
    /// clicked footer button index of message boxes. Other kinds pass the
    /// value documented on their [`WidgetKind`] variant, or `0`.
    pub fn on_value_changed<F>(mut self, map: F) -> Self
    where
        F: FnMut(i32) -> M + 'static,
    {
        self.handlers.value_changed = Some(Box::new(map));
        self
    }

    /// Map a textarea's current text to a message.
    ///
    /// The string is borrowed from LVGL for the duration of the call; copy it
    /// if the message must retain it. Non-UTF-8 native text is skipped.
    pub fn on_text_changed<F>(mut self, map: F) -> Self
    where
        F: FnMut(&str) -> M + 'static,
    {
        self.handlers.text_changed = Some(Box::new(map));
        self
    }

    /// Map the full 32-bit LVGL key code. NEXT/PREV are consumed by focus navigation.
    pub fn on_key<F>(mut self, map: F) -> Self
    where
        F: FnMut(u32) -> M + 'static,
    {
        self.handlers.key = Some(Box::new(map));
        self
    }

    /// Map the signed native rotary delta without truncation.
    pub fn on_rotary<F>(mut self, map: F) -> Self
    where
        F: FnMut(i32) -> M + 'static,
    {
        self.handlers.rotary = Some(Box::new(map));
        self
    }

    /// Map one directional gesture event (the generic Gesture event is separate).
    pub fn on_gesture<F>(mut self, map: F) -> Self
    where
        F: FnMut(crate::GestureDirection) -> M + 'static,
    {
        self.handlers.gesture = Some(Box::new(map));
        self
    }

    /// Map the actual offset in pixels on continuous LV_EVENT_SCROLL, including programmatic scrolling.
    pub fn on_scroll<F>(mut self, map: F) -> Self
    where
        F: FnMut(crate::ScrollOffset) -> M + 'static,
    {
        self.handlers.scroll = Some(Box::new(map));
        self
    }

    /// Map press/click/release pointer events with their display coordinates.
    pub fn on_pointer<F>(mut self, map: F) -> Self
    where
        F: FnMut(crate::PointerEvent) -> M + 'static,
    {
        self.handlers.pointer = Some(Box::new(map));
        self
    }

    /// Enqueue once on LV_EVENT_LONG_PRESSED (not its repeat event).
    pub fn on_long_press(mut self, msg: M) -> Self {
        self.handlers.long_press = Some(msg);
        self
    }

    /// Observe events that LVGL bubbles from descendants. Direct handlers
    /// remain local. Enable event_bubble on each intervening child for
    /// ordinary events; gesture_bubble follows LVGL gesture target rules.
    pub fn on_bubbled_event<F>(mut self, map: F) -> Self
    where
        F: FnMut(&crate::InputEvent) -> Option<M> + 'static,
    {
        self.handlers.bubbled = Some(Box::new(map));
        self
    }

    /// Map any supported event to an optional message.
    ///
    /// Only events whose target is this node are delivered; events bubbled
    /// from descendants are ignored.
    pub fn on_event<F>(mut self, map: F) -> Self
    where
        F: FnMut(&crate::Event) -> Option<M> + 'static,
    {
        self.handlers.event = Some(Box::new(map));
        self
    }
}

pub(crate) fn validate<M>(node: &Node<M>) -> Result<(), UiError> {
    let mut keyboard_ids = Vec::new();
    #[cfg(feature = "input-gridnav")]
    validate_navigation(node, false)?;
    validate_node(node, &mut keyboard_ids)
}

fn validate_node<M>(node: &Node<M>, keyboard_ids: &mut Vec<u64>) -> Result<(), UiError> {
    if let Some(offset) = node.interaction.offset {
        // Native coordinates reserve high bits for encoded size values.
        // Bounding both operands also keeps native scroll arithmetic in i32.
        let max = rs_lvgl_sys::LV_COORD_MAX as i32;
        if !(-max..=max).contains(&offset.x) || !(-max..=max).contains(&offset.y) {
            return Err(UiError::InvalidProperty);
        }
    }
    crate::animation::validate(&node.animations)?;
    validate_siblings(&node.children)?;
    validate_kind(node)?;
    validate_style(&node.style)?;
    for (_, style) in &node.part_styles {
        validate_style(style)?;
    }
    if let Some(id) = node.keyboard_id {
        // Keyboard targets are resolved tree-wide, so ids must be unique
        // across the whole tree rather than among siblings.
        if keyboard_ids.contains(&id) {
            return Err(UiError::InvalidProperty);
        }
        keyboard_ids.push(id);
    }
    for child in &node.children {
        validate_node(child, keyboard_ids)?;
    }
    Ok(())
}

fn validate_siblings<M>(children: &[Node<M>]) -> Result<(), UiError> {
    let mut named: Vec<&'static str> = Vec::new();
    let mut ids: Vec<u64> = Vec::new();
    for child in children {
        match child.key {
            Key::Auto => {}
            Key::Named(name) => {
                if named.contains(&name) {
                    return Err(UiError::DuplicateKey);
                }
                named.push(name);
            }
            Key::Id(id) => {
                if ids.contains(&id) {
                    return Err(UiError::DuplicateKey);
                }
                ids.push(id);
            }
        }
    }
    Ok(())
}

fn is_textarea(kind: &WidgetKind) -> bool {
    #[cfg(feature = "widget-textarea")]
    {
        matches!(kind, WidgetKind::Textarea { .. })
    }
    #[cfg(not(feature = "widget-textarea"))]
    {
        let _ = kind;
        false
    }
}

fn is_container(kind: &WidgetKind) -> bool {
    match kind {
        WidgetKind::Obj | WidgetKind::Row | WidgetKind::Column | WidgetKind::Grid { .. } => true,
        #[cfg(feature = "widget-tabview")]
        WidgetKind::Tabview { .. } => true,
        #[cfg(feature = "widget-tileview")]
        WidgetKind::Tileview { .. } => true,
        #[cfg(feature = "widget-msgbox")]
        WidgetKind::Msgbox { .. } => true,
        _ => false,
    }
}

#[cfg(any(feature = "widget-slider", feature = "widget-bar"))]
fn validate_range(min: i32, max: i32, value: i32) -> Result<(), UiError> {
    if min > max || value < min || value > max {
        return Err(UiError::InvalidProperty);
    }
    Ok(())
}

fn validate_kind<M>(node: &Node<M>) -> Result<(), UiError> {
    if node.handlers.text_changed.is_some() && !is_textarea(&node.kind) {
        return Err(UiError::InvalidProperty);
    }
    let container = is_container(&node.kind);
    if !container && !node.children.is_empty() {
        return Err(UiError::InvalidProperty);
    }

    if node.keyboard_id.is_some() && !is_textarea(&node.kind) {
        return Err(UiError::InvalidProperty);
    }
    if let Some(cell) = node.grid_cell {
        if cell.column_span == 0 || cell.row_span == 0 {
            return Err(UiError::InvalidProperty);
        }
    }

    match &node.kind {
        #[cfg(feature = "widget-ime-pinyin")]
        WidgetKind::PinyinKeyboard { font, .. } => {
            if font.0.is_null() {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-qrcode")]
        WidgetKind::QrCode { code } => code.validate()?,
        #[cfg(feature = "widget-barcode")]
        WidgetKind::Barcode { code } => code.validate()?,
        #[cfg(feature = "widget-gif")]
        WidgetKind::Gif { animation } => animation.validate()?,
        #[cfg(feature = "widget-lottie")]
        WidgetKind::Lottie { animation } => animation.validate()?,

        #[cfg(feature = "widget-canvas")]
        WidgetKind::Canvas { frame } => frame.validate()?,
        WidgetKind::Grid { columns, rows } => {
            if columns.is_empty() || rows.is_empty() {
                return Err(UiError::InvalidProperty);
            }
            for track in columns.iter().chain(rows) {
                match track {
                    GridTrack::Px(value) if *value < 0 => return Err(UiError::InvalidProperty),
                    GridTrack::Fr(value) if *value == 0 => return Err(UiError::InvalidProperty),
                    _ => {}
                }
            }
        }
        #[cfg(feature = "widget-tabview")]
        WidgetKind::Tabview { tabs, active } => {
            if tabs.is_empty()
                || *active as usize >= tabs.len()
                || node.children.len() != tabs.len()
            {
                return Err(UiError::InvalidProperty);
            }
            if tabs.iter().any(|tab| tab.as_bytes().contains(&0)) {
                return Err(UiError::InvalidProperty);
            }
        }
        WidgetKind::Label { text, .. } | WidgetKind::Button { text } => {
            if text.as_bytes().contains(&0) {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-checkbox")]
        WidgetKind::Checkbox { text, .. } => {
            if text.as_bytes().contains(&0) {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-textarea")]
        WidgetKind::Textarea {
            text, placeholder, ..
        } => {
            if text.as_bytes().contains(&0) || placeholder.as_bytes().contains(&0) {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-dropdown")]
        WidgetKind::Dropdown { options, selected } => {
            if options.is_empty() || *selected as usize >= options.len() {
                return Err(UiError::InvalidProperty);
            }
            if options
                .iter()
                .any(|option| option.as_bytes().contains(&0) || option.contains('\n'))
            {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-arc")]
        WidgetKind::Arc { min, max, value } => {
            if min > max || *value < *min || *value > *max {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-scale")]
        WidgetKind::Scale {
            min,
            max,
            total_ticks,
            major_tick_every,
        } => {
            if min > max
                || *total_ticks < 2
                || *major_tick_every == 0
                || *major_tick_every > *total_ticks
            {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-chart")]
        WidgetKind::Chart {
            chart_type: _,
            min,
            max,
            series,
        } => {
            if min > max || series.is_empty() {
                return Err(UiError::InvalidProperty);
            }
            let points = series[0].values.len();
            if points == 0 || series.iter().any(|series| series.values.len() != points) {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-slider")]
        WidgetKind::Slider { min, max, value } => validate_range(*min, *max, *value)?,
        #[cfg(feature = "widget-bar")]
        WidgetKind::Bar { min, max, value } => validate_range(*min, *max, *value)?,
        #[cfg(feature = "widget-image")]
        WidgetKind::Image { source, scale } => {
            if *scale == 0 {
                return Err(UiError::InvalidProperty);
            }
            validate_image_source(source)?;
        }
        #[cfg(feature = "widget-roller")]
        WidgetKind::Roller {
            options,
            selected,
            visible_rows,
            mode: _,
        } => {
            if options.is_empty() || *selected as usize >= options.len() || *visible_rows == 0 {
                return Err(UiError::InvalidProperty);
            }
            if options
                .iter()
                .any(|option| option.as_bytes().contains(&0) || option.contains('\n'))
            {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-spinbox")]
        WidgetKind::Spinbox {
            min,
            max,
            value,
            digit_count,
            decimal_pos,
            step,
        } => {
            if min > max || *value < *min || *value > *max {
                return Err(UiError::InvalidProperty);
            }
            if *digit_count == 0
                || *digit_count > SPINBOX_MAX_DIGITS
                || *decimal_pos >= *digit_count
            {
                return Err(UiError::InvalidProperty);
            }
            if *step == 0 {
                return Err(UiError::InvalidProperty);
            }
            // LVGL silently clamps the range to the representable digits.
            let limit = 10i64.pow(*digit_count) - 1;
            if i64::from(*max) > limit || i64::from(*min) < -limit {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-spinner")]
        WidgetKind::Spinner {
            duration_ms,
            arc_angle,
        } => {
            if *duration_ms == 0 || *arc_angle == 0 || *arc_angle > 360 {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-line")]
        WidgetKind::Line { points } => {
            if points.len() > u32::MAX as usize
                || points.iter().any(|&(x, y)| {
                    // lv_value_precise_t changes from i32 to f32 when a vector
                    // renderer is selected. Reject integers it cannot represent.
                    [x, y]
                        .into_iter()
                        .any(|v| (v as rs_lvgl_sys::lv_value_precise_t) as f64 != f64::from(v))
                })
            {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-buttonmatrix")]
        WidgetKind::ButtonMatrix { rows } => {
            if rows.is_empty() || rows.iter().any(Vec::is_empty) {
                return Err(UiError::InvalidProperty);
            }
            // An empty string terminates LVGL's map and "\n" separates rows.
            if rows
                .iter()
                .flatten()
                .any(|text| text.is_empty() || text == "\n" || text.as_bytes().contains(&0))
            {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-table")]
        WidgetKind::Table {
            cells,
            column_widths,
        } => {
            let columns = cells.first().map(Vec::len).unwrap_or(0);
            if cells.is_empty()
                || columns == 0
                || cells.len() > TABLE_MAX_CELLS_PER_AXIS
                || columns > TABLE_MAX_CELLS_PER_AXIS
                || cells.iter().any(|row| row.len() != columns)
                || column_widths.len() > columns
                || column_widths.iter().any(|width| *width < 0)
            {
                return Err(UiError::InvalidProperty);
            }
            if cells
                .iter()
                .flatten()
                .any(|text| text.as_bytes().contains(&0))
            {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-tileview")]
        WidgetKind::Tileview { tiles, active } => {
            if tiles.is_empty()
                || *active as usize >= tiles.len()
                || node.children.len() != tiles.len()
            {
                return Err(UiError::InvalidProperty);
            }
            for (index, tile) in tiles.iter().enumerate() {
                if tiles[..index]
                    .iter()
                    .any(|other| (other.column, other.row) == (tile.column, tile.row))
                {
                    return Err(UiError::InvalidProperty);
                }
            }
        }
        #[cfg(feature = "widget-msgbox")]
        WidgetKind::Msgbox {
            title,
            text,
            buttons,
        } => {
            if title
                .iter()
                .chain(core::iter::once(text))
                .chain(buttons)
                .any(|text| text.as_bytes().contains(&0))
            {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-calendar")]
        WidgetKind::Calendar {
            year: _,
            month,
            today,
            highlighted,
        } => {
            if !(1..=12).contains(month)
                || today.map(|date| !date.is_valid()).unwrap_or(false)
                || highlighted.iter().any(|date| !date.is_valid())
            {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-span")]
        WidgetKind::Spans { spans } => {
            if spans.iter().any(|span| span.text.as_bytes().contains(&0)) {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-imagebutton")]
        WidgetKind::ImageButton { released, pressed } => {
            for source in core::iter::once(released).chain(pressed) {
                validate_image_source(source)?;
            }
        }
        #[cfg(feature = "widget-animimage")]
        WidgetKind::AnimImage {
            frames,
            duration_ms,
            repeat: _,
        } => {
            if frames.is_empty() || frames.len() > ANIM_IMAGE_MAX_FRAMES || *duration_ms == 0 {
                return Err(UiError::InvalidProperty);
            }
            for frame in frames {
                validate_image_source(frame)?;
            }
        }
        #[cfg(feature = "widget-arclabel")]
        WidgetKind::ArcLabel {
            text, angle_size, ..
        } => {
            if text.as_bytes().contains(&0) || !(0..=360).contains(angle_size) {
                return Err(UiError::InvalidProperty);
            }
        }
        WidgetKind::Obj | WidgetKind::Row | WidgetKind::Column => {}
        #[cfg(feature = "widget-switch")]
        WidgetKind::Switch { .. } => {}
        #[cfg(feature = "widget-led")]
        WidgetKind::Led { .. } => {}
        #[cfg(feature = "widget-keyboard")]
        WidgetKind::Keyboard { .. } => {}
        WidgetKind::Custom(custom) => {
            // `crate::raw` owns its native callbacks; lifecycle widgets may use
            // the typed handler because the reconciler registers it on the
            // custom root object.
            if !custom.is_lifecycle() && !node.handlers.is_empty() {
                return Err(UiError::InvalidProperty);
            }
        }
    }
    Ok(())
}

fn validate_image_source(source: &ImageSource) -> Result<(), UiError> {
    match source {
        ImageSource::Raw(raw) if raw.as_ptr().is_null() => Err(UiError::InvalidProperty),
        ImageSource::Rgb565 {
            width,
            height,
            data,
        } => {
            let expected = (*width as usize)
                .checked_mul(*height as usize)
                .and_then(|pixels| pixels.checked_mul(2));
            if *width == 0 || *height == 0 || expected != Some(data.len()) {
                return Err(UiError::InvalidProperty);
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_style(style: &Style) -> Result<(), UiError> {
    style.extended.validate()?;
    if let Some(transition) = style.transition {
        transition.validate()?;
    }
    if style.border_width < 0 || style.radius < 0 {
        return Err(UiError::InvalidProperty);
    }
    for size in [
        style.width,
        style.height,
        style.max_width,
        style.max_height,
        style.min_width,
        style.min_height,
    ] {
        if let Size::Px(value) = size {
            if value < 0 {
                return Err(UiError::InvalidProperty);
            }
        }
    }
    Ok(())
}

#[cfg(feature = "input-gridnav")]
fn validate_navigation<M>(node: &Node<M>, inside_grid: bool) -> Result<(), UiError> {
    if inside_grid && (node.grid_navigation.is_some() || node.modal) {
        return Err(UiError::InvalidProperty);
    }
    if node.grid_navigation.is_some()
        && !matches!(
            node.kind,
            WidgetKind::Obj | WidgetKind::Row | WidgetKind::Column | WidgetKind::Grid { .. }
        )
    {
        return Err(UiError::InvalidProperty);
    }
    // The IME's focus target is a private native child, not the direct grid child.
    #[cfg(feature = "widget-ime-pinyin")]
    if inside_grid && matches!(node.kind, WidgetKind::PinyinKeyboard { .. }) {
        return Err(UiError::InvalidProperty);
    }
    for child in &node.children {
        validate_navigation(child, inside_grid || node.grid_navigation.is_some())?;
    }
    Ok(())
}
