//! Style merge operations.

use super::{
    Style, STYLE_ALIGN, STYLE_ALL_NEUTRAL, STYLE_ANIM_DURATION, STYLE_ARC_COLOR, STYLE_ARC_WIDTH,
    STYLE_BG, STYLE_BG_OPA, STYLE_BORDER, STYLE_DISABLED, STYLE_FG, STYLE_FLEX_ALIGN,
    STYLE_FLOATING, STYLE_FONT, STYLE_GAP, STYLE_GROW, STYLE_HEIGHT, STYLE_HIDDEN,
    STYLE_MAX_HEIGHT, STYLE_MAX_WIDTH, STYLE_MIN_HEIGHT, STYLE_MIN_WIDTH, STYLE_OFFSET,
    STYLE_OPACITY, STYLE_PAD_BOTTOM, STYLE_PAD_LEFT, STYLE_PAD_RIGHT, STYLE_PAD_TOP, STYLE_RADIUS,
    STYLE_TEXT_ALIGN, STYLE_WIDTH,
};

impl Style {
    /// A neutral style: transparent, unpadded, square, content-sized and
    /// top-left aligned.
    ///
    /// Text color, font and text alignment are not set: LVGL inherits them
    /// from the parent, so `crate::column([]).style(Style::body(theme))` styles
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
}
