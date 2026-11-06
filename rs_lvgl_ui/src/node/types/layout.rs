//! Dimensions and placement within flex and grid layouts.

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
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl Align {
    pub(crate) fn to_raw(self) -> rs_lvgl_sys::lv_align_t {
        match self {
            Align::TopLeft => rs_lvgl_sys::lv_align_t_LV_ALIGN_TOP_LEFT,
            Align::TopCenter => rs_lvgl_sys::lv_align_t_LV_ALIGN_TOP_MID,
            Align::TopRight => rs_lvgl_sys::lv_align_t_LV_ALIGN_TOP_RIGHT,
            Align::CenterLeft => rs_lvgl_sys::lv_align_t_LV_ALIGN_LEFT_MID,
            Align::Center => rs_lvgl_sys::lv_align_t_LV_ALIGN_CENTER,
            Align::CenterRight => rs_lvgl_sys::lv_align_t_LV_ALIGN_RIGHT_MID,
            Align::BottomLeft => rs_lvgl_sys::lv_align_t_LV_ALIGN_BOTTOM_LEFT,
            Align::BottomCenter => rs_lvgl_sys::lv_align_t_LV_ALIGN_BOTTOM_MID,
            Align::BottomRight => rs_lvgl_sys::lv_align_t_LV_ALIGN_BOTTOM_RIGHT,
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

/// Placement along the main axis or placement of flex lines in their container.
/// Use [`ItemAlign`] for children inside a line.
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

/// Placement of a child within its flex line, perpendicular to the flow.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ItemAlign {
    #[default]
    Start,
    Center,
    End,
}

impl ItemAlign {
    pub(crate) fn to_raw(self) -> rs_lvgl_sys::lv_flex_align_t {
        match self {
            Self::Start => rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_START,
            Self::Center => rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_CENTER,
            Self::End => rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_END,
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

/// Placement of one child inside a [`crate::WidgetKind::Grid`] parent.
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
