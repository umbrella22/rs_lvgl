//! Style builders operations.

use super::{
    Style, STYLE_ALIGN, STYLE_ANIM_DURATION, STYLE_ARC_COLOR, STYLE_ARC_WIDTH, STYLE_BG,
    STYLE_BG_OPA, STYLE_BORDER, STYLE_DISABLED, STYLE_FG, STYLE_FLEX_ALIGN, STYLE_FLOATING,
    STYLE_FONT, STYLE_GAP, STYLE_GROW, STYLE_HEIGHT, STYLE_HIDDEN, STYLE_MAX_HEIGHT,
    STYLE_MAX_WIDTH, STYLE_MIN_HEIGHT, STYLE_MIN_WIDTH, STYLE_OFFSET, STYLE_OPACITY,
    STYLE_PAD_BOTTOM, STYLE_PAD_LEFT, STYLE_PAD_RIGHT, STYLE_PAD_TOP, STYLE_RADIUS,
    STYLE_TEXT_ALIGN, STYLE_WIDTH,
};
use crate::node::{Align, Color, FlexAlign, Font, Size, TextAlign};

impl Style {
    /// Set an opaque background color.
    ///
    /// This also sets the background opacity to 255. Call
    /// [`Style::background_opacity`] after this method for a translucent fill.
    pub fn background(mut self, color: Color) -> Self {
        self.bg = color;
        self.bg_opa = 255;
        self.set |= STYLE_BG | STYLE_BG_OPA;
        self
    }

    /// Set the background opacity independently of the color.
    ///
    /// Merging `style().background_opacity(128)` over [`Style::surface`]
    /// keeps the surface color and makes it translucent.
    pub fn background_opacity(mut self, opacity: u8) -> Self {
        self.bg_opa = opacity;
        self.set |= STYLE_BG_OPA;
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
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
    pub fn padding(mut self, pad: i16) -> Self {
        self.pad_left = pad;
        self.pad_right = pad;
        self.pad_top = pad;
        self.pad_bottom = pad;
        self.set |= STYLE_PAD_LEFT | STYLE_PAD_RIGHT | STYLE_PAD_TOP | STYLE_PAD_BOTTOM;
        self
    }

    pub fn padding_left(mut self, pad: i16) -> Self {
        self.pad_left = pad;
        self.set |= STYLE_PAD_LEFT;
        self
    }

    pub fn padding_right(mut self, pad: i16) -> Self {
        self.pad_right = pad;
        self.set |= STYLE_PAD_RIGHT;
        self
    }

    pub fn padding_top(mut self, pad: i16) -> Self {
        self.pad_top = pad;
        self.set |= STYLE_PAD_TOP;
        self
    }

    pub fn padding_bottom(mut self, pad: i16) -> Self {
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

    pub fn grow(mut self, grow: u8) -> Self {
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

    pub fn background_value(&self) -> Color {
        self.bg
    }

    pub fn background_opacity_value(&self) -> u8 {
        self.bg_opa
    }

    pub fn text_color_value(&self) -> Color {
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

    pub fn padding_left_value(&self) -> i16 {
        self.pad_left
    }

    pub fn padding_right_value(&self) -> i16 {
        self.pad_right
    }

    pub fn padding_top_value(&self) -> i16 {
        self.pad_top
    }

    pub fn padding_bottom_value(&self) -> i16 {
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

    pub fn grow_value(&self) -> u8 {
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
}
