//! Colors, text alignment and native style selectors.

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

/// Widget part targeted by [`crate::Node::part_style`].
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

/// Widget states targeted by [`crate::Node::state_style`]. Combine with `|`; a
/// combined state matches only while every listed state is active.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct State(u16);

impl State {
    pub const PRESSED: Self = Self(rs_lvgl_sys::lv_state_t_LV_STATE_PRESSED as u16);
    /// Set by [`crate::Node::checked`] or by checkable LVGL widgets.
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
