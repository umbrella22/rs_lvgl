//! Native local styles and selector restoration.

use super::KindTag;
use crate::node::{NativePart, Selector, Size};
use crate::theme::{
    Style, STYLE_ALIGN, STYLE_ALIGN_CONTENT, STYLE_ALIGN_ITEMS, STYLE_ANIM_DURATION,
    STYLE_ARC_COLOR, STYLE_ARC_WIDTH, STYLE_BG, STYLE_BG_OPA, STYLE_BORDER, STYLE_DISABLED,
    STYLE_FG, STYLE_FLOATING, STYLE_FONT, STYLE_GAP, STYLE_GROW, STYLE_HEIGHT, STYLE_HIDDEN,
    STYLE_JUSTIFY_CONTENT, STYLE_MAX_HEIGHT, STYLE_MAX_WIDTH, STYLE_MIN_HEIGHT, STYLE_MIN_WIDTH,
    STYLE_OFFSET, STYLE_OPACITY, STYLE_PAD_BOTTOM, STYLE_PAD_LEFT, STYLE_PAD_RIGHT, STYLE_PAD_TOP,
    STYLE_RADIUS, STYLE_TEXT_ALIGN, STYLE_WIDTH,
};
use core::ptr::NonNull;
use rs_lvgl_sys::lv_obj_t;

const REMOVE_PROPS: [u32; 29] = [
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_BG_COLOR,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_BG_OPA,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_BORDER_COLOR,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_BORDER_WIDTH,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_RADIUS,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_PAD_TOP,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_PAD_BOTTOM,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_PAD_LEFT,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_PAD_RIGHT,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_PAD_ROW,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_PAD_COLUMN,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_WIDTH,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_HEIGHT,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_MAX_WIDTH,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_MAX_HEIGHT,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_MIN_WIDTH,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_MIN_HEIGHT,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_TEXT_ALIGN,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_FLEX_GROW,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_ALIGN,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_OPA,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_TEXT_COLOR,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_TEXT_FONT,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_ANIM_DURATION,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_X,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_Y,
    // Removing the flex placement restores `FlexAlign::Start`, the value the
    // containers are created with.
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_FLEX_MAIN_PLACE,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_FLEX_CROSS_PLACE,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_FLEX_TRACK_PLACE,
];

/// Apply a resolved style to the main part as local properties.
///
/// Local properties are removed first so that resetting a field restores the
/// widget class default (for example a slider's track) instead of overriding it
/// with a neutral value.
pub(crate) fn apply_style(obj: NonNull<lv_obj_t>, style: &Style, tag: KindTag) {
    apply_style_selector(obj, style, tag, 0);
}

pub(crate) fn apply_style_for_selector(
    obj: NonNull<lv_obj_t>,
    style: &Style,
    tag: KindTag,
    selector: Selector,
) {
    apply_style_selector(obj, style, tag, selector.to_raw());
}

pub(crate) fn clear_selector_style(obj: NonNull<lv_obj_t>, selector: Selector) {
    let part = selector.part;
    let selector = selector.to_raw();
    crate::style_properties::ExtendedStyle::clear(obj, selector);
    // SAFETY: `obj` is a live object; every property id belongs to LVGL's
    // built-in style property set.
    unsafe {
        for prop in REMOVE_PROPS {
            rs_lvgl_sys::lv_obj_remove_local_style_prop(
                obj.as_ptr(),
                prop as rs_lvgl_sys::lv_style_prop_t,
                selector,
            );
        }
        if part == NativePart::Indicator {
            rs_lvgl_sys::lv_obj_remove_local_style_prop(
                obj.as_ptr(),
                rs_lvgl_sys::_lv_style_id_t_LV_STYLE_ARC_COLOR as rs_lvgl_sys::lv_style_prop_t,
                selector,
            );
            rs_lvgl_sys::lv_obj_remove_local_style_prop(
                obj.as_ptr(),
                rs_lvgl_sys::_lv_style_id_t_LV_STYLE_ARC_WIDTH as rs_lvgl_sys::lv_style_prop_t,
                selector,
            );
        }
    }
}

fn apply_style_selector(
    obj: NonNull<lv_obj_t>,
    style: &Style,
    tag: KindTag,
    selector: rs_lvgl_sys::lv_style_selector_t,
) {
    crate::style_properties::ExtendedStyle::clear(obj, selector);
    style.extended.apply(obj, selector);
    // SAFETY: `obj` is a live object; every property id belongs to LVGL's
    // built-in style property set.
    unsafe {
        for prop in REMOVE_PROPS {
            rs_lvgl_sys::lv_obj_remove_local_style_prop(
                obj.as_ptr(),
                prop as rs_lvgl_sys::lv_style_prop_t,
                selector,
            );
        }

        if style.is_set(STYLE_BG) {
            rs_lvgl_sys::lv_obj_set_style_bg_color(obj.as_ptr(), style.bg.to_raw(), selector);
        }
        if style.is_set(STYLE_BG_OPA) {
            rs_lvgl_sys::lv_obj_set_style_bg_opa(obj.as_ptr(), style.bg_opa, selector);
        }
        if style.is_set(STYLE_FG) {
            rs_lvgl_sys::lv_obj_set_style_text_color(obj.as_ptr(), style.fg.to_raw(), selector);
        }
        if style.is_set(STYLE_FONT) {
            rs_lvgl_sys::lv_obj_set_style_text_font(obj.as_ptr(), style.font.0, selector);
        }
        if style.is_set(STYLE_BORDER) {
            rs_lvgl_sys::lv_obj_set_style_border_color(
                obj.as_ptr(),
                style.border.to_raw(),
                selector,
            );
            rs_lvgl_sys::lv_obj_set_style_border_width(
                obj.as_ptr(),
                style.border_width as i32,
                selector,
            );
        }
        if style.is_set(STYLE_RADIUS) {
            rs_lvgl_sys::lv_obj_set_style_radius(obj.as_ptr(), style.radius as i32, selector);
        }
        if style.is_set(STYLE_PAD_LEFT) {
            rs_lvgl_sys::lv_obj_set_style_pad_left(obj.as_ptr(), style.pad_left as i32, selector);
        }
        if style.is_set(STYLE_PAD_RIGHT) {
            rs_lvgl_sys::lv_obj_set_style_pad_right(obj.as_ptr(), style.pad_right as i32, selector);
        }
        if style.is_set(STYLE_PAD_TOP) {
            rs_lvgl_sys::lv_obj_set_style_pad_top(obj.as_ptr(), style.pad_top as i32, selector);
        }
        if style.is_set(STYLE_PAD_BOTTOM) {
            rs_lvgl_sys::lv_obj_set_style_pad_bottom(
                obj.as_ptr(),
                style.pad_bottom as i32,
                selector,
            );
        }
        if style.is_set(STYLE_WIDTH) {
            set_size(obj, style.width, true, selector);
        }
        if style.is_set(STYLE_HEIGHT) {
            set_size(obj, style.height, false, selector);
        }
        if style.is_set(STYLE_MAX_WIDTH) {
            set_bound(obj, style.max_width, true, true, selector);
        }
        if style.is_set(STYLE_MAX_HEIGHT) {
            set_bound(obj, style.max_height, false, true, selector);
        }
        if style.is_set(STYLE_MIN_WIDTH) {
            set_bound(obj, style.min_width, true, false, selector);
        }
        if style.is_set(STYLE_MIN_HEIGHT) {
            set_bound(obj, style.min_height, false, false, selector);
        }
        if style.is_set(STYLE_TEXT_ALIGN) {
            rs_lvgl_sys::lv_obj_set_style_text_align(
                obj.as_ptr(),
                style.text_align.to_raw(),
                selector,
            );
        }
        if style.is_set(STYLE_ANIM_DURATION) {
            rs_lvgl_sys::lv_obj_set_style_anim_duration(
                obj.as_ptr(),
                style.animation_duration,
                selector,
            );
        }
        if style.is_set(STYLE_GROW) {
            rs_lvgl_sys::lv_obj_set_style_flex_grow(obj.as_ptr(), style.flex_grow, selector);
        }
        if style.is_set(STYLE_ALIGN) {
            rs_lvgl_sys::lv_obj_set_style_align(obj.as_ptr(), style.align.to_raw(), selector);
        }
        if style.is_set(STYLE_OPACITY) {
            rs_lvgl_sys::lv_obj_set_style_opa(obj.as_ptr(), style.opacity, selector);
        }
        if style.is_set(STYLE_OFFSET) {
            rs_lvgl_sys::lv_obj_set_style_x(obj.as_ptr(), style.offset_x as i32, selector);
            rs_lvgl_sys::lv_obj_set_style_y(obj.as_ptr(), style.offset_y as i32, selector);
        }

        // Object-wide state and layout properties only make sense on the main
        // part; a part style must not change the whole widget's visibility or
        // container layout.
        if selector == 0 {
            // Mounted nodes are never floating by class default, so an unset
            // field clears the flag.
            rs_lvgl_sys::lv_obj_set_floating(
                obj.as_ptr(),
                style.is_set(STYLE_FLOATING) && style.floating,
            );
            if style.is_set(STYLE_HIDDEN) {
                rs_lvgl_sys::lv_obj_set_hidden(obj.as_ptr(), style.hidden);
            }
            if style.is_set(STYLE_DISABLED) {
                if style.disabled {
                    rs_lvgl_sys::lv_obj_add_state(
                        obj.as_ptr(),
                        rs_lvgl_sys::lv_state_t_LV_STATE_DISABLED,
                    );
                } else {
                    rs_lvgl_sys::lv_obj_remove_state(
                        obj.as_ptr(),
                        rs_lvgl_sys::lv_state_t_LV_STATE_DISABLED,
                    );
                }
            }
            #[cfg(feature = "widget-textarea")]
            if tag == KindTag::Textarea
                && !style.is_set(STYLE_HEIGHT)
                && rs_lvgl_sys::lv_textarea_get_one_line(obj.as_ptr())
            {
                rs_lvgl_sys::lv_obj_set_height(obj.as_ptr(), LV_SIZE_CONTENT);
            }
            if matches!(tag, KindTag::Row | KindTag::Column) {
                if style.is_set(STYLE_JUSTIFY_CONTENT) {
                    rs_lvgl_sys::lv_obj_set_style_flex_main_place(
                        obj.as_ptr(),
                        style.flex_main.to_raw(),
                        0,
                    );
                }
                if style.is_set(STYLE_ALIGN_ITEMS) {
                    rs_lvgl_sys::lv_obj_set_style_flex_cross_place(
                        obj.as_ptr(),
                        style.flex_cross.to_raw(),
                        0,
                    );
                }
                if style.is_set(STYLE_ALIGN_CONTENT) {
                    rs_lvgl_sys::lv_obj_set_style_flex_track_place(
                        obj.as_ptr(),
                        style.flex_track.to_raw(),
                        0,
                    );
                }
            }
            if style.is_set(STYLE_GAP) {
                match tag {
                    KindTag::Row => {
                        rs_lvgl_sys::lv_obj_set_style_pad_column(obj.as_ptr(), style.gap as i32, 0);
                    }
                    KindTag::Column => {
                        rs_lvgl_sys::lv_obj_set_style_pad_row(obj.as_ptr(), style.gap as i32, 0);
                    }
                    KindTag::Grid => {
                        rs_lvgl_sys::lv_obj_set_style_pad_row(obj.as_ptr(), style.gap as i32, 0);
                        rs_lvgl_sys::lv_obj_set_style_pad_column(obj.as_ptr(), style.gap as i32, 0);
                    }
                    _ => {}
                }
            }
        }

        #[cfg(feature = "widget-arc")]
        if tag == KindTag::Arc
            && (selector == 0 || selector == rs_lvgl_sys::lv_part_t_LV_PART_INDICATOR)
        {
            rs_lvgl_sys::lv_obj_remove_local_style_prop(
                obj.as_ptr(),
                rs_lvgl_sys::_lv_style_id_t_LV_STYLE_ARC_COLOR as rs_lvgl_sys::lv_style_prop_t,
                rs_lvgl_sys::lv_part_t_LV_PART_INDICATOR,
            );
            rs_lvgl_sys::lv_obj_remove_local_style_prop(
                obj.as_ptr(),
                rs_lvgl_sys::_lv_style_id_t_LV_STYLE_ARC_WIDTH as rs_lvgl_sys::lv_style_prop_t,
                rs_lvgl_sys::lv_part_t_LV_PART_INDICATOR,
            );
            if style.is_set(STYLE_ARC_COLOR) {
                rs_lvgl_sys::lv_obj_set_style_arc_color(
                    obj.as_ptr(),
                    style.arc_color.to_raw(),
                    rs_lvgl_sys::lv_part_t_LV_PART_INDICATOR,
                );
            }
            if style.is_set(STYLE_ARC_WIDTH) {
                rs_lvgl_sys::lv_obj_set_style_arc_width(
                    obj.as_ptr(),
                    style.arc_width as i32,
                    rs_lvgl_sys::lv_part_t_LV_PART_INDICATOR,
                );
            }
        }
    }
}

/// `LV_SIZE_CONTENT` is a C macro; LVGL encodes it as a special coordinate.
const LV_SIZE_CONTENT: i32 = 0x3fff_ffff;

fn size_value(size: Size) -> Option<i32> {
    match size {
        Size::Px(value) => Some(value as i32),
        Size::Percent(percent) => {
            // SAFETY: `lv_pct` is a pure coordinate conversion helper.
            Some(unsafe { rs_lvgl_sys::lv_pct(percent as i32) })
        }
        Size::Content => None,
    }
}

fn set_size(
    obj: NonNull<lv_obj_t>,
    size: Size,
    horizontal: bool,
    selector: rs_lvgl_sys::lv_style_selector_t,
) {
    let value = size_value(size).unwrap_or(LV_SIZE_CONTENT);
    // SAFETY: `obj` is a live object.
    unsafe {
        if horizontal {
            rs_lvgl_sys::lv_obj_set_style_width(obj.as_ptr(), value, selector);
        } else {
            rs_lvgl_sys::lv_obj_set_style_height(obj.as_ptr(), value, selector);
        }
    }
}

fn set_bound(
    obj: NonNull<lv_obj_t>,
    size: Size,
    horizontal: bool,
    maximum: bool,
    selector: rs_lvgl_sys::lv_style_selector_t,
) {
    let Some(value) = size_value(size) else {
        return;
    };
    // SAFETY: `obj` is a live object.
    unsafe {
        match (horizontal, maximum) {
            (true, true) => rs_lvgl_sys::lv_obj_set_style_max_width(obj.as_ptr(), value, selector),
            (false, true) => {
                rs_lvgl_sys::lv_obj_set_style_max_height(obj.as_ptr(), value, selector)
            }
            (true, false) => rs_lvgl_sys::lv_obj_set_style_min_width(obj.as_ptr(), value, selector),
            (false, false) => {
                rs_lvgl_sys::lv_obj_set_style_min_height(obj.as_ptr(), value, selector)
            }
        }
    }
}
