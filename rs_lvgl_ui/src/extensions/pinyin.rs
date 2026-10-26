//! The native IME deletes its keyboard and candidate panel. Put all three in
//! one private container so reconciliation deletes them as a single subtree.
use crate::{widgets::Parts, UiError};
use core::ptr::NonNull;

pub(crate) fn create(
    parent: NonNull<rs_lvgl_sys::lv_obj_t>,
) -> Result<(NonNull<rs_lvgl_sys::lv_obj_t>, Parts), UiError> {
    // SAFETY: parent is live; all native helpers stay inside the private root.
    // Creation order puts the IME before its keyboard so the IME destructor
    // can delete its dependents before root deletion continues.
    unsafe {
        let root =
            NonNull::new(rs_lvgl_sys::lv_obj_create(parent.as_ptr())).ok_or(UiError::Allocation)?;
        crate::widgets::remove_main_theme(root);
        let ime = rs_lvgl_sys::lv_ime_pinyin_create(root.as_ptr());
        if ime.is_null() {
            rs_lvgl_sys::lv_obj_delete(root.as_ptr());
            return Err(UiError::Allocation);
        }
        let keyboard = rs_lvgl_sys::lv_keyboard_create(root.as_ptr());
        if keyboard.is_null() {
            rs_lvgl_sys::lv_obj_delete(root.as_ptr());
            return Err(UiError::Allocation);
        }
        rs_lvgl_sys::lv_ime_pinyin_set_keyboard(ime, keyboard);
        rs_lvgl_sys::lv_obj_set_width(keyboard, rs_lvgl_sys::lv_pct(100));
        rs_lvgl_sys::lv_obj_set_height(keyboard, rs_lvgl_sys::lv_pct(80));
        rs_lvgl_sys::lv_obj_align(keyboard, rs_lvgl_sys::lv_align_t_LV_ALIGN_BOTTOM_MID, 0, 0);
        let panel = rs_lvgl_sys::lv_ime_pinyin_get_cand_panel(ime);
        if panel.is_null() {
            rs_lvgl_sys::lv_obj_delete(root.as_ptr());
            return Err(UiError::Allocation);
        }
        rs_lvgl_sys::lv_obj_set_height(panel, rs_lvgl_sys::lv_pct(20));
        rs_lvgl_sys::lv_obj_align(panel, rs_lvgl_sys::lv_align_t_LV_ALIGN_TOP_MID, 0, 0);
        Ok((
            root,
            Parts {
                input: NonNull::new(keyboard),
                ime: NonNull::new(ime),
                ..Parts::default()
            },
        ))
    }
}
