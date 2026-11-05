//! Flat modifiers update the style already embedded in a view description.

use crate::node::{Align, Color, FlexAlign, Font, Node, Size, TextAlign};
use crate::StyleTransition;

macro_rules! modifiers {
    ($($name:ident($($argument:ident: $ty:ty),+) => $documentation:literal;)*) => {
        impl<M, K> Node<M, K> {
            $(
                #[doc = $documentation]
                #[inline]
                pub fn $name(mut self, $($argument: $ty),+) -> Self {
                    self.style = self.style.$name($($argument),+);
                    self
                }
            )*
        }
    };
}

modifiers! {
    background(color: Color) => "Set the background color and an opaque fill.";
    background_opacity(opacity: u8) => "Set background opacity, preserving its color. Apply after background when combining them.";
    text_color(color: Color) => "Set the text color inherited by descendant labels.";
    border(color: Color, width: i16) => "Set border color and width in pixels. Negative widths fail validation.";
    radius(radius: i16) => "Set the corner radius in pixels. Negative radii fail validation.";
    padding(padding: i16) => "Set all four inner paddings in pixels.";
    padding_left(padding: i16) => "Set left inner padding in pixels.";
    padding_right(padding: i16) => "Set right inner padding in pixels.";
    padding_top(padding: i16) => "Set top inner padding in pixels.";
    padding_bottom(padding: i16) => "Set bottom inner padding in pixels.";
    gap(gap: i16) => "Set main-axis spacing in rows/columns and both grid gaps in pixels. Plain objects do not use it.";
    width(width: Size) => "Set width with an explicit pixel, percentage or content unit.";
    height(height: Size) => "Set height with an explicit pixel, percentage or content unit.";
    min_width(width: Size) => "Set minimum width with an explicit size unit.";
    min_height(height: Size) => "Set minimum height with an explicit size unit.";
    max_width(width: Size) => "Set maximum width with an explicit size unit.";
    max_height(height: Size) => "Set maximum height with an explicit size unit.";
    grow(grow: u8) => "Set proportional flex growth inside a row or column.";
    align(align: Align) => "Align this object when its parent layout does not place it.";
    opacity(opacity: u8) => "Set object opacity from transparent (0) to opaque (255).";
    font(font: Font) => "Set the static font inherited by descendant labels. A retained font_resource takes priority.";
    text_align(align: TextAlign) => "Set the alignment of text inside the widget.";
    offset(x: i16, y: i16) => "Offset from the aligned position in pixels; layout-managed children need floating placement.";
    floating(floating: bool) => "Exclude this object from parent layout and scrolling.";
    flex_align(main: FlexAlign, cross: FlexAlign) => "Place children along the main and cross axes of a row or column.";
    hidden(hidden: bool) => "Set whether the object is hidden.";
    disabled(disabled: bool) => "Set the object's disabled state.";
    arc_color(color: Color) => "Set the indicator color of an arc.";
    arc_width(width: i16) => "Set the indicator width of an arc in pixels.";
    anim_duration(duration: u32) => "Set native style animation duration in milliseconds.";
    transition(transition: StyleTransition) => "Set a native transition for the specified style properties.";
}
