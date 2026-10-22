use crate::{reconciler::MountedNode, ScrollDir, ScrollOffset};
use core::ptr::NonNull;

/// Native scrollbar visibility policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScrollbarMode {
    Off,
    On,
    Active,
    Auto,
}

impl ScrollbarMode {
    fn to_raw(self) -> rs_lvgl_sys::lv_scrollbar_mode_t {
        match self {
            Self::Off => rs_lvgl_sys::lv_scrollbar_mode_t_LV_SCROLLBAR_MODE_OFF,
            Self::On => rs_lvgl_sys::lv_scrollbar_mode_t_LV_SCROLLBAR_MODE_ON,
            Self::Active => rs_lvgl_sys::lv_scrollbar_mode_t_LV_SCROLLBAR_MODE_ACTIVE,
            Self::Auto => rs_lvgl_sys::lv_scrollbar_mode_t_LV_SCROLLBAR_MODE_AUTO,
        }
    }
}

/// Alignment of snappable children when scrolling settles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScrollSnap {
    None,
    Start,
    End,
    Center,
}

impl ScrollSnap {
    fn to_raw(self) -> rs_lvgl_sys::lv_scroll_snap_t {
        match self {
            Self::None => rs_lvgl_sys::lv_scroll_snap_t_LV_SCROLL_SNAP_NONE,
            Self::Start => rs_lvgl_sys::lv_scroll_snap_t_LV_SCROLL_SNAP_START,
            Self::End => rs_lvgl_sys::lv_scroll_snap_t_LV_SCROLL_SNAP_END,
            Self::Center => rs_lvgl_sys::lv_scroll_snap_t_LV_SCROLL_SNAP_CENTER,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Interaction {
    pub event_bubble: Option<bool>,
    pub gesture_bubble: Option<bool>,
    pub dir: Option<ScrollDir>,
    pub snap_x: Option<ScrollSnap>,
    pub snap_y: Option<ScrollSnap>,
    pub scrollbar: Option<ScrollbarMode>,
    pub momentum: Option<bool>,
    pub elastic: Option<bool>,
    pub offset: Option<ScrollOffset>,
}

impl Interaction {
    pub(crate) fn apply(self, obj: NonNull<rs_lvgl_sys::lv_obj_t>, old: Self) {
        macro_rules! apply {
            ($field:ident, $set:path $(, $convert:ident)?) => {
                if let Some(value) = self.$field {
                    if old.$field != Some(value) {
                        // SAFETY: the reconciler supplies a live object and
                        // typed values map exactly to native enums/flags.
                        unsafe { $set(obj.as_ptr(), value$(.$convert())?) };
                    }
                }
            };
        }
        apply!(event_bubble, rs_lvgl_sys::lv_obj_set_event_bubble);
        apply!(gesture_bubble, rs_lvgl_sys::lv_obj_set_gesture_bubble);
        apply!(dir, rs_lvgl_sys::lv_obj_set_scroll_dir, to_raw);
        apply!(snap_x, rs_lvgl_sys::lv_obj_set_scroll_snap_x, to_raw);
        apply!(snap_y, rs_lvgl_sys::lv_obj_set_scroll_snap_y, to_raw);
        apply!(scrollbar, rs_lvgl_sys::lv_obj_set_scrollbar_mode, to_raw);
        apply!(momentum, rs_lvgl_sys::lv_obj_set_scroll_momentum);
        apply!(elastic, rs_lvgl_sys::lv_obj_set_scroll_elastic);
    }
}

pub(crate) fn apply_offsets<M: Clone + 'static>(node: &MountedNode<M>) {
    for child in &node.children {
        apply_offsets(child);
    }
    if let Some(offset) = node.interaction.offset {
        // SAFETY: the full subtree is mounted. LVGL resolves layout before
        // applying an absolute offset, constrained by actual content bounds.
        unsafe {
            rs_lvgl_sys::lv_obj_update_layout(node.obj.as_ptr());
            rs_lvgl_sys::lv_obj_scroll_to(node.obj.as_ptr(), offset.x, offset.y, false);
        }
    }
}
