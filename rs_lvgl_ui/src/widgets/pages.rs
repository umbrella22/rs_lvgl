//! Native page creation and controlled tab/tile selection.

use crate::node::Tile;
use crate::UiError;
use alloc::{string::String, vec::Vec};
use core::ptr::NonNull;
use cstr_core::CString;
use rs_lvgl_sys::lv_obj_t;

#[cfg(feature = "widget-tileview")]
pub(crate) fn tileview_pages(
    obj: NonNull<lv_obj_t>,
    tiles: &[Tile],
    active: u32,
) -> Result<Vec<NonNull<lv_obj_t>>, UiError> {
    let mut pages = Vec::with_capacity(tiles.len());
    for tile in tiles {
        // SAFETY: `obj` is a live tileview.
        let page = unsafe {
            rs_lvgl_sys::lv_tileview_add_tile(
                obj.as_ptr(),
                tile.column,
                tile.row,
                tile.directions.to_raw(),
            )
        };
        let Some(page) = NonNull::new(page) else {
            return Err(UiError::Allocation);
        };
        pages.push(page);
    }
    set_tileview_active(obj, &pages, active);
    Ok(pages)
}

/// Show tile `active` without animation.
#[cfg(feature = "widget-tileview")]
pub(crate) fn set_tileview_active(
    obj: NonNull<lv_obj_t>,
    pages: &[NonNull<lv_obj_t>],
    active: u32,
) {
    if let Some(page) = pages.get(active as usize) {
        // SAFETY: `obj` is a live tileview and `page` one of its tiles.
        unsafe { rs_lvgl_sys::lv_tileview_set_tile(obj.as_ptr(), page.as_ptr(), false) };
    }
}

/// Whether tile `active` is the tileview's active tile.
#[cfg(feature = "widget-tileview")]
pub(crate) fn tileview_is_active(
    obj: NonNull<lv_obj_t>,
    pages: &[NonNull<lv_obj_t>],
    active: u32,
) -> bool {
    // SAFETY: `obj` is a live tileview.
    let current = unsafe { rs_lvgl_sys::lv_tileview_get_tile_active(obj.as_ptr()) };
    pages.get(active as usize).map(|page| page.as_ptr()) == Some(current)
}

#[cfg(feature = "widget-tabview")]
pub(crate) fn tabview_pages(
    obj: NonNull<lv_obj_t>,
    tabs: &[String],
    active: u32,
) -> Result<Vec<NonNull<lv_obj_t>>, UiError> {
    let mut pages = Vec::with_capacity(tabs.len());
    for title in tabs {
        let title = CString::new(title.as_str()).map_err(|_| UiError::InvalidProperty)?;
        // SAFETY: `obj` is a live tabview object. LVGL copies the title.
        let page = unsafe { rs_lvgl_sys::lv_tabview_add_tab(obj.as_ptr(), title.as_ptr()) };
        let Some(page) = NonNull::new(page) else {
            return Err(UiError::Allocation);
        };
        pages.push(page);
    }
    // SAFETY: `obj` is a live tabview and `active` was validated.
    unsafe { rs_lvgl_sys::lv_tabview_set_active(obj.as_ptr(), active, false) };
    Ok(pages)
}

#[cfg(feature = "widget-tabview")]
pub(crate) fn set_tabview_titles(obj: NonNull<lv_obj_t>, old: &[String], new: &[String]) {
    for (index, (old, new)) in old.iter().zip(new).enumerate() {
        if old == new {
            continue;
        }
        let Ok(title) = CString::new(new.as_str()) else {
            continue;
        };
        // SAFETY: `obj` is a live tabview and `index` addresses an existing tab.
        unsafe {
            rs_lvgl_sys::lv_tabview_set_tab_text(obj.as_ptr(), index as u32, title.as_ptr());
        }
    }
}

#[cfg(feature = "widget-tabview")]
pub(crate) fn set_tabview_active(obj: NonNull<lv_obj_t>, active: u32) {
    // SAFETY: `obj` is a live tabview and `active` was validated.
    unsafe { rs_lvgl_sys::lv_tabview_set_active(obj.as_ptr(), active, false) };
}
