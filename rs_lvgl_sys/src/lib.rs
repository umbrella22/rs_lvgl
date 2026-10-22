#![no_std]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::redundant_static_lifetimes)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub fn _bindgen_raw_src() -> &'static str {
    include_str!(concat!(env!("OUT_DIR"), "/bindings.rs"))
}

mod string_impl;

#[cfg(test)]
mod tests {
    use super::*;

    /// LVGL 9 sanity: lv_init succeeds and a created display reports the
    /// requested resolution through the real (non-NULL) display path.
    #[test]
    fn basic_sanity_check() {
        unsafe {
            lv_init();

            let disp = lv_display_create(320, 240);
            assert!(!disp.is_null());
            assert_eq!(lv_display_get_horizontal_resolution(disp), 320);
            assert_eq!(lv_display_get_vertical_resolution(disp), 240);
            lv_display_delete(disp);
            assert!(lv_display_get_default().is_null());
        }
    }
}
