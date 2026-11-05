//! Widget-specific values for controls, data and rich text.

use super::{Color, Font};
use alloc::{string::String, vec::Vec};

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

/// Decode the value a table passes to [`crate::Node::on_value_changed`].
///
/// Tables report the selected cell packed as `(row << 16) | column`, or `-1`
/// when no cell is selected. Returns `(row, column)`.
pub fn table_cell(value: i32) -> Option<(u32, u32)> {
    let value = value as u32;
    let (row, column) = (value >> 16, value & 0xFFFF);
    (row != 0xFFFF && column != 0xFFFF).then_some((row, column))
}

/// A selected table cell. Native indices are copied without signed packing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TableCell {
    pub row: u32,
    pub column: u32,
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

    /// Decode the value a calendar passes to [`crate::Node::on_value_changed`].
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
