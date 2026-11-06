//! Additional native style properties. Options preserve LVGL inheritance and
//! distinguish an omitted property from an explicit zero.

use crate::{Angle, Color, Scale, Style, UiError};
use core::ptr::NonNull;
use rs_lvgl_sys::lv_obj_t;

/// Direction of a two-stop background gradient.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum GradientDirection {
    None = rs_lvgl_sys::lv_grad_dir_t_LV_GRAD_DIR_NONE,
    Horizontal = rs_lvgl_sys::lv_grad_dir_t_LV_GRAD_DIR_HOR,
    Vertical = rs_lvgl_sys::lv_grad_dir_t_LV_GRAD_DIR_VER,
}

/// Native pixel blending operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlendMode {
    Normal,
    Additive,
    Subtractive,
    Multiply,
    Difference,
}

/// Text decoration flags. Underline and strikethrough may be combined.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TextDecoration(u8);
impl TextDecoration {
    pub const NONE: Self = Self(0);
    pub const UNDERLINE: Self = Self(1);
    pub const STRIKETHROUGH: Self = Self(2);
}
impl core::ops::BitOr for TextDecoration {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

macro_rules! properties {
    ($( $(#[$doc:meta])* $field:ident: $ty:ty => $setter:ident, $prop:ident, $convert:expr; )*) => {
        #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
        pub(crate) struct ExtendedStyle { $(pub(crate) $field: Option<$ty>,)* }
        impl ExtendedStyle {
            pub(crate) fn merge(&mut self, other: Self) {
                $(if other.$field.is_some() { self.$field = other.$field; })*
            }
            pub(crate) fn clear(obj: NonNull<lv_obj_t>, selector: u32) {
                // SAFETY: the reconciler supplies a live object and native selectors.
                unsafe { $(rs_lvgl_sys::lv_obj_remove_local_style_prop(obj.as_ptr(),
                    rs_lvgl_sys::$prop as rs_lvgl_sys::lv_style_prop_t, selector);)* }
            }
            pub(crate) fn apply(self, obj: NonNull<lv_obj_t>, selector: u32) {
                // SAFETY: values were validated before reconciliation. These setters
                // copy scalar values; no borrowed data is retained by C.
                unsafe { $(if let Some(value) = self.$field {
                    rs_lvgl_sys::$setter(obj.as_ptr(), ($convert)(value), selector);
                })* }
            }
        }
        impl Style { $(
            $(#[$doc])*
            pub fn $field(mut self, value: $ty) -> Self {
                self.extended.$field = Some(value); self
            }
        )* }
        impl<M, K> crate::node::Node<M, K> { $(
            $(#[$doc])*
            #[inline]
            pub fn $field(mut self, value: $ty) -> Self {
                self.style = self.style.$field(value); self
            }
        )* }
    };
}

properties! {
    /// Shadow blur diameter in pixels. Negative widths are rejected on mount/update.
    shadow_width: i16 => lv_obj_set_style_shadow_width, _lv_style_id_t_LV_STYLE_SHADOW_WIDTH, i32::from;
    /// Shadow color, independent of width and opacity.
    shadow_color: Color => lv_obj_set_style_shadow_color, _lv_style_id_t_LV_STYLE_SHADOW_COLOR, Color::to_raw;
    /// Shadow opacity from transparent (0) to opaque (255).
    shadow_opacity: u8 => lv_obj_set_style_shadow_opa, _lv_style_id_t_LV_STYLE_SHADOW_OPA, |v| v;
    /// Horizontal shadow displacement in pixels; negative values are supported.
    shadow_offset_x: i16 => lv_obj_set_style_shadow_offset_x, _lv_style_id_t_LV_STYLE_SHADOW_OFFSET_X, i32::from;
    /// Vertical shadow displacement in pixels; negative values are supported.
    shadow_offset_y: i16 => lv_obj_set_style_shadow_offset_y, _lv_style_id_t_LV_STYLE_SHADOW_OFFSET_Y, i32::from;
    /// Shadow expansion in pixels; negative values contract it.
    shadow_spread: i16 => lv_obj_set_style_shadow_spread, _lv_style_id_t_LV_STYLE_SHADOW_SPREAD, i32::from;
    /// Outline width in pixels; negative widths are rejected.
    outline_width: i16 => lv_obj_set_style_outline_width, _lv_style_id_t_LV_STYLE_OUTLINE_WIDTH, i32::from;
    /// Outline color.
    outline_color: Color => lv_obj_set_style_outline_color, _lv_style_id_t_LV_STYLE_OUTLINE_COLOR, Color::to_raw;
    /// Outline opacity from 0 to 255.
    outline_opacity: u8 => lv_obj_set_style_outline_opa, _lv_style_id_t_LV_STYLE_OUTLINE_OPA, |v| v;
    /// Signed distance between the object and its outline, in pixels.
    outline_offset: i16 => lv_obj_set_style_outline_pad, _lv_style_id_t_LV_STYLE_OUTLINE_PAD, i32::from;
    /// Color at the second gradient stop; the first uses `Style::background`.
    background_gradient_color: Color => lv_obj_set_style_bg_grad_color, _lv_style_id_t_LV_STYLE_BG_GRAD_COLOR, Color::to_raw;
    /// Native two-stop background gradient direction.
    background_gradient_direction: GradientDirection => lv_obj_set_style_bg_grad_dir, _lv_style_id_t_LV_STYLE_BG_GRAD_DIR, |v: GradientDirection| v as _;
    /// Position of the first gradient stop, from 0 to 255.
    background_gradient_start: u8 => lv_obj_set_style_bg_main_stop, _lv_style_id_t_LV_STYLE_BG_MAIN_STOP, i32::from;
    /// Position of the second gradient stop, from 0 to 255.
    background_gradient_end: u8 => lv_obj_set_style_bg_grad_stop, _lv_style_id_t_LV_STYLE_BG_GRAD_STOP, i32::from;
    /// Opacity of the first gradient stop.
    background_gradient_start_opacity: u8 => lv_obj_set_style_bg_main_opa, _lv_style_id_t_LV_STYLE_BG_MAIN_OPA, |v| v;
    /// Opacity of the second gradient stop.
    background_gradient_end_opacity: u8 => lv_obj_set_style_bg_grad_opa, _lv_style_id_t_LV_STYLE_BG_GRAD_OPA, |v| v;
    /// Signed extra space between letters in pixels. Inherited by descendants.
    letter_spacing: i16 => lv_obj_set_style_text_letter_space, _lv_style_id_t_LV_STYLE_TEXT_LETTER_SPACE, i32::from;
    /// Signed extra space between text lines in pixels. Inherited by descendants.
    line_spacing: i16 => lv_obj_set_style_text_line_space, _lv_style_id_t_LV_STYLE_TEXT_LINE_SPACE, i32::from;
    /// Inherited underline/strikethrough decoration.
    text_decoration: TextDecoration => lv_obj_set_style_text_decor, _lv_style_id_t_LV_STYLE_TEXT_DECOR, |v: TextDecoration| v.0 as _;
    /// Tint color for image pixels; `image_tint_opacity` controls strength.
    image_tint: Color => lv_obj_set_style_image_recolor, _lv_style_id_t_LV_STYLE_IMAGE_RECOLOR, Color::to_raw;
    /// Image tint opacity from 0 to 255.
    image_tint_opacity: u8 => lv_obj_set_style_image_recolor_opa, _lv_style_id_t_LV_STYLE_IMAGE_RECOLOR_OPA, |v| v;
    /// Horizontal scale, e.g. `Scale::percent(150)`. Zero is allowed.
    transform_scale_x: Scale => lv_obj_set_style_transform_scale_x, _lv_style_id_t_LV_STYLE_TRANSFORM_SCALE_X, Scale::to_raw;
    /// Vertical scale, e.g. `Scale::percent(75)`. Zero is allowed.
    transform_scale_y: Scale => lv_obj_set_style_transform_scale_y, _lv_style_id_t_LV_STYLE_TRANSFORM_SCALE_Y, Scale::to_raw;
    /// Clockwise rotation, e.g. `Angle::degrees(90)` or `Angle::tenths(455)`.
    transform_rotation: Angle => lv_obj_set_style_transform_rotation, _lv_style_id_t_LV_STYLE_TRANSFORM_ROTATION, Angle::to_raw;
    /// Horizontal pivot in pixels relative to the object.
    transform_pivot_x: i16 => lv_obj_set_style_transform_pivot_x, _lv_style_id_t_LV_STYLE_TRANSFORM_PIVOT_X, i32::from;
    /// Vertical pivot in pixels relative to the object.
    transform_pivot_y: i16 => lv_obj_set_style_transform_pivot_y, _lv_style_id_t_LV_STYLE_TRANSFORM_PIVOT_Y, i32::from;
    /// Signed expansion of the transformed rectangle in pixels.
    transform_width: i16 => lv_obj_set_style_transform_width, _lv_style_id_t_LV_STYLE_TRANSFORM_WIDTH, i32::from;
    /// Signed vertical expansion of the transformed rectangle in pixels.
    transform_height: i16 => lv_obj_set_style_transform_height, _lv_style_id_t_LV_STYLE_TRANSFORM_HEIGHT, i32::from;
    /// Blend operation used when drawing this object/part.
    blend_mode: BlendMode => lv_obj_set_style_blend_mode, _lv_style_id_t_LV_STYLE_BLEND_MODE, |v: BlendMode| v as _;
}

impl ExtendedStyle {
    pub(crate) fn validate(self) -> Result<(), UiError> {
        if self.shadow_width.is_some_and(|v| v < 0) || self.outline_width.is_some_and(|v| v < 0) {
            return Err(UiError::InvalidProperty);
        }
        for scale in [self.transform_scale_x, self.transform_scale_y]
            .into_iter()
            .flatten()
        {
            scale.validate()?;
        }
        if let Some(angle) = self.transform_rotation {
            angle.validate()?;
        }
        Ok(())
    }
}
