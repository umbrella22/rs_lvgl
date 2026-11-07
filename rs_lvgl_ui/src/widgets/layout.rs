//! Stable native grid templates and grid cell placement.

use crate::node::{GridCell, GridTrack};
use alloc::{boxed::Box, vec::Vec};
use core::ptr::NonNull;
use rs_lvgl_sys::lv_obj_t;

/// Build the LVGL template arrays for a grid and keep them alive in the
/// mounted node.
pub(crate) fn grid_descriptors(
    columns: &[GridTrack],
    rows: &[GridTrack],
) -> (Box<[i32]>, Box<[i32]>) {
    fn tracks(tracks: &[GridTrack]) -> Box<[i32]> {
        let mut values: Vec<i32> = tracks.iter().map(|track| track.to_raw()).collect();
        // LV_GRID_TEMPLATE_LAST is LV_COORD_MAX.
        values.push(0x1fff_ffff);
        values.into_boxed_slice()
    }
    (tracks(columns), tracks(rows))
}

pub(crate) fn set_grid_descriptors(obj: NonNull<lv_obj_t>, columns: &[i32], rows: &[i32]) {
    // SAFETY: `obj` is a live object and both arrays are terminated and kept
    // alive by the mounted node for as long as LVGL references them.
    unsafe {
        rs_lvgl_sys::lv_obj_set_grid_dsc_array(obj.as_ptr(), columns.as_ptr(), rows.as_ptr());
    }
}

pub(crate) fn apply_grid_cell(obj: NonNull<lv_obj_t>, cell: GridCell) {
    // SAFETY: `obj` is a live object. Out-of-range cells are clamped by LVGL.
    unsafe {
        rs_lvgl_sys::lv_obj_set_grid_cell(
            obj.as_ptr(),
            cell.column_align.to_raw(),
            cell.column as i32,
            cell.column_span as i32,
            cell.row_align.to_raw(),
            cell.row as i32,
            cell.row_span as i32,
        );
    }
}

/// Apply independent row/column flow flags without changing native identity.
pub(crate) fn apply_flex_flow(
    obj: NonNull<lv_obj_t>,
    tag: super::KindTag,
    (wrap, reverse): (bool, bool),
) {
    use rs_lvgl_sys as sys;
    let flow = match (tag, wrap, reverse) {
        (super::KindTag::Row, false, false) => sys::lv_flex_flow_t_LV_FLEX_FLOW_ROW,
        (super::KindTag::Row, true, false) => sys::lv_flex_flow_t_LV_FLEX_FLOW_ROW_WRAP,
        (super::KindTag::Row, false, true) => sys::lv_flex_flow_t_LV_FLEX_FLOW_ROW_REVERSE,
        (super::KindTag::Row, true, true) => sys::lv_flex_flow_t_LV_FLEX_FLOW_ROW_WRAP_REVERSE,
        (super::KindTag::Column, false, false) => sys::lv_flex_flow_t_LV_FLEX_FLOW_COLUMN,
        (super::KindTag::Column, true, false) => sys::lv_flex_flow_t_LV_FLEX_FLOW_COLUMN_WRAP,
        (super::KindTag::Column, false, true) => sys::lv_flex_flow_t_LV_FLEX_FLOW_COLUMN_REVERSE,
        (super::KindTag::Column, true, true) => {
            sys::lv_flex_flow_t_LV_FLEX_FLOW_COLUMN_WRAP_REVERSE
        }
        _ => return,
    };
    // SAFETY: the reconciler supplies its live row/column native object. These
    // scalar style values retain no Rust pointers; equal flow avoids a layout write.
    unsafe {
        if sys::lv_obj_get_style_flex_flow(obj.as_ptr(), sys::lv_part_t_LV_PART_MAIN) != flow {
            sys::lv_obj_set_flex_flow(obj.as_ptr(), flow);
        }
    }
}
