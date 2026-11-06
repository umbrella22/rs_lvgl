//! Style/transition replacement while font resources remain retained.

use super::MountedNode;
use crate::node::{NativePart, Selector, State};
use crate::widgets::{self, KindTag};
use crate::Style;
use alloc::vec::Vec;
use core::ptr::NonNull;
use rs_lvgl_sys::lv_obj_t;

/// Re-apply style and part styles when either changed.
pub(super) fn update_styles<M: Clone + 'static>(
    mounted: &mut MountedNode<M>,
    style: Style,
    part_styles: Vec<(Selector, Style)>,
    font_resource: Option<crate::FontResource>,
) {
    let tag = mounted.tag;
    if tag == KindTag::Custom {
        mounted.style = style;
        mounted.part_styles = part_styles;
        return;
    }
    let part_styles_changed = part_styles != mounted.part_styles;
    let styles_changed = style != mounted.style || part_styles_changed;
    if styles_changed {
        crate::animation::clear_transitions(mounted.obj, &mut mounted.transitions);
        widgets::apply_style(mounted.obj, &style, tag);
    }
    if part_styles_changed {
        for part in [
            NativePart::Scrollbar,
            NativePart::Indicator,
            NativePart::Knob,
            NativePart::Selected,
            NativePart::Items,
            NativePart::Cursor,
        ] {
            widgets::clear_selector_style(mounted.obj, Selector::new(part, State::DEFAULT));
        }
        // State styles live on their own selectors, including the main part.
        for (selector, _) in &mounted.part_styles {
            if selector.state != State::DEFAULT {
                widgets::clear_selector_style(mounted.obj, *selector);
            }
        }
        for (selector, native_part_style) in &part_styles {
            widgets::apply_style_for_selector(mounted.obj, native_part_style, tag, *selector);
        }
        mounted.part_styles = part_styles;
    }
    if styles_changed {
        mounted.transitions =
            crate::animation::install_transitions(mounted.obj, &style, &mounted.part_styles);
    }
    if (font_resource != mounted.font_resource || styles_changed)
        && (font_resource.is_some() || mounted.font_resource.is_some())
    {
        apply_resource_font(mounted.obj, &style, font_resource.as_ref());
    }
    mounted.font_resource = font_resource;
    mounted.style = style;
}

pub(super) fn apply_resource_font(
    obj: NonNull<lv_obj_t>,
    style: &Style,
    font: Option<&crate::FontResource>,
) {
    // SAFETY: the caller retains old and new font resources during assignment;
    // obj is live and rendering is synchronous under validate_support.
    unsafe {
        if let Some(font) = font {
            rs_lvgl_sys::lv_obj_set_style_text_font(obj.as_ptr(), font.as_ptr(), 0);
        } else if style.is_set(crate::theme::STYLE_FONT) {
            rs_lvgl_sys::lv_obj_set_style_text_font(obj.as_ptr(), style.font.0, 0);
        } else {
            rs_lvgl_sys::lv_obj_remove_local_style_prop(
                obj.as_ptr(),
                rs_lvgl_sys::_lv_style_id_t_LV_STYLE_TEXT_FONT as _,
                0,
            );
        }
    }
}
