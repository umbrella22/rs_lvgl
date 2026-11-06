//! Native widget creation and initial class/layout setup.

use super::{KindTag, Parts};
use crate::UiError;
use core::ptr::NonNull;
use rs_lvgl_sys::lv_obj_t;

/// An inner object whose events count as the widget's own.
///
/// A calendar is a container around a button matrix whose events bubble to
/// the calendar; the handlers of the calendar node must observe them.
pub(crate) fn event_inner_target(tag: KindTag, obj: NonNull<lv_obj_t>) -> *mut lv_obj_t {
    match tag {
        // SAFETY: `obj` is a live calendar; the button matrix is its child.
        #[cfg(feature = "widget-calendar")]
        KindTag::Calendar => unsafe { rs_lvgl_sys::lv_calendar_get_btnmatrix(obj.as_ptr()) },
        _ => core::ptr::null_mut(),
    }
}

/// Create the native object for a typed kind.
///
/// Custom kinds are created by the caller because their factory owns the
/// object.
pub(crate) fn create(
    tag: KindTag,
    parent: NonNull<lv_obj_t>,
) -> Result<(NonNull<lv_obj_t>, Parts), UiError> {
    // SAFETY: `parent` is a live LVGL object supplied by the mounted tree. Each
    // create function returns either a new child object or null.
    unsafe {
        let raw = match tag {
            KindTag::Container | KindTag::Row | KindTag::Column | KindTag::Grid => {
                rs_lvgl_sys::lv_obj_create(parent.as_ptr())
            }
            #[cfg(feature = "widget-tabview")]
            KindTag::Tabview => rs_lvgl_sys::lv_tabview_create(parent.as_ptr()),
            #[cfg(feature = "widget-keyboard")]
            KindTag::Keyboard => rs_lvgl_sys::lv_keyboard_create(parent.as_ptr()),
            #[cfg(feature = "widget-canvas")]
            KindTag::Canvas => rs_lvgl_sys::lv_canvas_create(parent.as_ptr()),
            #[cfg(feature = "widget-qrcode")]
            KindTag::QrCode => rs_lvgl_sys::lv_qrcode_create(parent.as_ptr()),
            #[cfg(feature = "widget-barcode")]
            KindTag::Barcode => rs_lvgl_sys::lv_barcode_create(parent.as_ptr()),
            #[cfg(feature = "widget-gif")]
            KindTag::Gif => rs_lvgl_sys::lv_gif_create(parent.as_ptr()),
            #[cfg(feature = "widget-lottie")]
            KindTag::Lottie => rs_lvgl_sys::lv_lottie_create(parent.as_ptr()),
            #[cfg(feature = "widget-ime-pinyin")]
            KindTag::PinyinKeyboard => return crate::extensions::pinyin::create(parent),
            KindTag::Label => rs_lvgl_sys::lv_label_create(parent.as_ptr()),
            KindTag::Button => rs_lvgl_sys::lv_button_create(parent.as_ptr()),
            #[cfg(feature = "widget-slider")]
            KindTag::Slider => rs_lvgl_sys::lv_slider_create(parent.as_ptr()),
            #[cfg(feature = "widget-bar")]
            KindTag::ProgressBar => rs_lvgl_sys::lv_bar_create(parent.as_ptr()),
            #[cfg(feature = "widget-switch")]
            KindTag::Switch => rs_lvgl_sys::lv_switch_create(parent.as_ptr()),
            #[cfg(feature = "widget-image")]
            KindTag::Image => rs_lvgl_sys::lv_image_create(parent.as_ptr()),
            #[cfg(feature = "widget-checkbox")]
            KindTag::Checkbox => rs_lvgl_sys::lv_checkbox_create(parent.as_ptr()),
            #[cfg(feature = "widget-textarea")]
            KindTag::Textarea => rs_lvgl_sys::lv_textarea_create(parent.as_ptr()),
            #[cfg(feature = "widget-dropdown")]
            KindTag::Dropdown => rs_lvgl_sys::lv_dropdown_create(parent.as_ptr()),
            #[cfg(feature = "widget-arc")]
            KindTag::Arc => {
                let arc = rs_lvgl_sys::lv_arc_create(parent.as_ptr());
                if !arc.is_null() {
                    // Gauge defaults: hide the background arc and the knob,
                    // keep a visible indicator that the style can recolor.
                    rs_lvgl_sys::lv_obj_set_style_arc_opa(
                        arc,
                        0,
                        rs_lvgl_sys::lv_part_t_LV_PART_MAIN,
                    );
                    rs_lvgl_sys::lv_obj_set_style_bg_opa(
                        arc,
                        0,
                        rs_lvgl_sys::lv_part_t_LV_PART_KNOB,
                    );
                    rs_lvgl_sys::lv_obj_set_style_arc_width(
                        arc,
                        10,
                        rs_lvgl_sys::lv_part_t_LV_PART_INDICATOR,
                    );
                    rs_lvgl_sys::lv_obj_set_clickable(arc, false);
                }
                arc
            }
            #[cfg(feature = "widget-scale")]
            KindTag::Scale => rs_lvgl_sys::lv_scale_create(parent.as_ptr()),
            #[cfg(feature = "widget-chart")]
            KindTag::Chart => rs_lvgl_sys::lv_chart_create(parent.as_ptr()),
            #[cfg(feature = "widget-roller")]
            KindTag::Roller => rs_lvgl_sys::lv_roller_create(parent.as_ptr()),
            #[cfg(feature = "widget-spinbox")]
            KindTag::Spinbox => rs_lvgl_sys::lv_spinbox_create(parent.as_ptr()),
            #[cfg(feature = "widget-spinner")]
            KindTag::Spinner => rs_lvgl_sys::lv_spinner_create(parent.as_ptr()),
            #[cfg(feature = "widget-led")]
            KindTag::Led => rs_lvgl_sys::lv_led_create(parent.as_ptr()),
            #[cfg(feature = "widget-line")]
            KindTag::Line => rs_lvgl_sys::lv_line_create(parent.as_ptr()),
            #[cfg(feature = "widget-buttonmatrix")]
            KindTag::ButtonMatrix => rs_lvgl_sys::lv_buttonmatrix_create(parent.as_ptr()),
            #[cfg(feature = "widget-table")]
            KindTag::Table => rs_lvgl_sys::lv_table_create(parent.as_ptr()),
            #[cfg(feature = "widget-tileview")]
            KindTag::Tileview => rs_lvgl_sys::lv_tileview_create(parent.as_ptr()),
            // A non-null parent keeps the message box inline: a null parent
            // would create a modal backdrop on the top layer that the
            // mounted tree does not own.
            #[cfg(feature = "widget-msgbox")]
            KindTag::MessageBox => rs_lvgl_sys::lv_msgbox_create(parent.as_ptr()),
            #[cfg(feature = "widget-calendar")]
            KindTag::Calendar => rs_lvgl_sys::lv_calendar_create(parent.as_ptr()),
            #[cfg(feature = "widget-span")]
            KindTag::Spans => rs_lvgl_sys::lv_spangroup_create(parent.as_ptr()),
            #[cfg(feature = "widget-imagebutton")]
            KindTag::ImageButton => rs_lvgl_sys::lv_imagebutton_create(parent.as_ptr()),
            #[cfg(feature = "widget-animimage")]
            KindTag::AnimatedImage => rs_lvgl_sys::lv_animimg_create(parent.as_ptr()),
            #[cfg(feature = "widget-arclabel")]
            KindTag::ArcLabel => rs_lvgl_sys::lv_arclabel_create(parent.as_ptr()),
            KindTag::Custom => unreachable!("custom widgets are created by their factory"),
        };
        let Some(obj) = NonNull::new(raw) else {
            return Err(UiError::Allocation);
        };

        if matches!(
            tag,
            KindTag::Container | KindTag::Row | KindTag::Column | KindTag::Grid
        ) {
            remove_main_theme(obj);
        }

        if matches!(tag, KindTag::Row | KindTag::Column) {
            rs_lvgl_sys::lv_obj_set_layout(obj.as_ptr(), rs_lvgl_sys::lv_layout_t_LV_LAYOUT_FLEX);
            rs_lvgl_sys::lv_obj_set_flex_flow(
                obj.as_ptr(),
                if tag == KindTag::Row {
                    rs_lvgl_sys::lv_flex_flow_t_LV_FLEX_FLOW_ROW
                } else {
                    rs_lvgl_sys::lv_flex_flow_t_LV_FLEX_FLOW_COLUMN
                },
            );
            rs_lvgl_sys::lv_obj_set_flex_align(
                obj.as_ptr(),
                rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_START,
                rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_START,
                rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_START,
            );
        }

        let mut parts = Parts::default();
        match tag {
            KindTag::Button => {
                let label = rs_lvgl_sys::lv_label_create(obj.as_ptr());
                let Some(label) = NonNull::new(label) else {
                    rs_lvgl_sys::lv_obj_delete(obj.as_ptr());
                    return Err(UiError::Allocation);
                };
                rs_lvgl_sys::lv_obj_center(label.as_ptr());
                parts.label = Some(label);
            }
            #[cfg(feature = "widget-msgbox")]
            KindTag::MessageBox => {
                // The text label is the first child of the content area;
                // the node's children follow it.
                let label = rs_lvgl_sys::lv_msgbox_add_text(obj.as_ptr(), c"".as_ptr().cast());
                let content = rs_lvgl_sys::lv_msgbox_get_content(obj.as_ptr());
                let (Some(label), Some(content)) = (NonNull::new(label), NonNull::new(content))
                else {
                    rs_lvgl_sys::lv_obj_delete(obj.as_ptr());
                    return Err(UiError::Allocation);
                };
                parts.label = Some(label);
                parts.content = Some(content);
            }
            _ => {}
        }

        Ok((obj, parts))
    }
}

/// Drop the LVGL theme styles from a plain container's main part.
///
/// Plain containers are layout primitives. The default theme gives every
/// `lv_obj` a card style (background, border, padding and a text color);
/// that text color would stop text properties set on an ancestor from
/// reaching descendants. Scrollbar part styles are kept.
pub(crate) fn remove_main_theme(obj: NonNull<lv_obj_t>) {
    // SAFETY: `obj` is a live object; removing theme styles only detaches
    // shared style references from this object.
    unsafe {
        rs_lvgl_sys::lv_obj_remove_theme(
            obj.as_ptr(),
            rs_lvgl_sys::lv_part_t_LV_PART_MAIN | rs_lvgl_sys::lv_state_t_LV_STATE_ANY,
        );
    }
}
