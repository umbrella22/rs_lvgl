pub fn pct(x: i32) -> i32 {
    unsafe { rs_lvgl_sys::lv_pct(x) }
}
