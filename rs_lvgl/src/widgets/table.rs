use crate::lv_core::obj::NativeObject;
use crate::widgets::Table;
use core::mem::MaybeUninit;

impl Table<'_> {
    pub fn set_col_width(&mut self, column: u32, width: i32) {
        unsafe { rs_lvgl_sys::lv_table_set_column_width(self.core.raw().as_ptr(), column, width) }
    }

    pub fn get_col_width(&self, column: u32) -> i32 {
        unsafe { rs_lvgl_sys::lv_table_get_column_width(self.core.raw().as_ptr(), column) }
    }

    pub fn set_selected_cell(&mut self, row: u16, col: u16) {
        unsafe { rs_lvgl_sys::lv_table_set_selected_cell(self.core.raw().as_ptr(), row, col) }
    }

    pub fn get_selected_cell(&self) -> (u32, u32) {
        let mut row = MaybeUninit::<u32>::uninit();
        let mut col = MaybeUninit::<u32>::uninit();
        unsafe {
            rs_lvgl_sys::lv_table_get_selected_cell(
                self.core.raw().as_ptr(),
                row.as_mut_ptr(),
                col.as_mut_ptr(),
            );
            (row.assume_init(), col.assume_init())
        }
    }
}
