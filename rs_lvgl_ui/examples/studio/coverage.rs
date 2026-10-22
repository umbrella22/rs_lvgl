//! Native class coverage: every widget must exist in the actual mounted view.
use rs_lvgl_sys::lv_obj_class_t;
unsafe extern "C" {
    static lv_animimg_class: lv_obj_class_t;
    static lv_arc_class: lv_obj_class_t;
    static lv_arclabel_class: lv_obj_class_t;
    static lv_bar_class: lv_obj_class_t;
    static lv_barcode_class: lv_obj_class_t;
    static lv_buttonmatrix_class: lv_obj_class_t;
    static lv_calendar_class: lv_obj_class_t;
    static lv_canvas_class: lv_obj_class_t;
    static lv_chart_class: lv_obj_class_t;
    static lv_checkbox_class: lv_obj_class_t;
    static lv_dropdown_class: lv_obj_class_t;
    static lv_gif_class: lv_obj_class_t;
    static lv_image_class: lv_obj_class_t;
    static lv_imagebutton_class: lv_obj_class_t;
    static lv_ime_pinyin_class: lv_obj_class_t;
    static lv_keyboard_class: lv_obj_class_t;
    static lv_led_class: lv_obj_class_t;
    static lv_line_class: lv_obj_class_t;
    static lv_list_class: lv_obj_class_t;
    static lv_lottie_class: lv_obj_class_t;
    static lv_menu_class: lv_obj_class_t;
    static lv_msgbox_class: lv_obj_class_t;
    static lv_qrcode_class: lv_obj_class_t;
    static lv_roller_class: lv_obj_class_t;
    static lv_scale_class: lv_obj_class_t;
    static lv_slider_class: lv_obj_class_t;
    static lv_spangroup_class: lv_obj_class_t;
    static lv_spinbox_class: lv_obj_class_t;
    static lv_spinner_class: lv_obj_class_t;
    static lv_switch_class: lv_obj_class_t;
    static lv_table_class: lv_obj_class_t;
    static lv_tabview_class: lv_obj_class_t;
    static lv_textarea_class: lv_obj_class_t;
    static lv_tileview_class: lv_obj_class_t;
    static lv_win_class: lv_obj_class_t;
}
pub(super) fn expected(page: usize) -> Vec<(&'static str, *const lv_obj_class_t)> {
    match page {
        1 => vec![
            ("slider", &raw const lv_slider_class),
            ("bar", &raw const lv_bar_class),
            ("switch", &raw const lv_switch_class),
            ("checkbox", &raw const lv_checkbox_class),
            ("led", &raw const lv_led_class),
            ("arc", &raw const lv_arc_class),
            ("spinner", &raw const lv_spinner_class),
            ("dropdown", &raw const lv_dropdown_class),
            ("roller", &raw const lv_roller_class),
            ("spinbox", &raw const lv_spinbox_class),
            ("buttonmatrix", &raw const lv_buttonmatrix_class),
        ],
        2 => vec![
            ("textarea", &raw const lv_textarea_class),
            ("keyboard", &raw const lv_keyboard_class),
            ("ime_pinyin", &raw const lv_ime_pinyin_class),
            ("calendar", &raw const lv_calendar_class),
            ("spangroup", &raw const lv_spangroup_class),
        ],
        3 => vec![
            ("chart", &raw const lv_chart_class),
            ("table", &raw const lv_table_class),
            ("scale", &raw const lv_scale_class),
            ("line", &raw const lv_line_class),
        ],
        4 => vec![
            ("tabview", &raw const lv_tabview_class),
            ("tileview", &raw const lv_tileview_class),
        ],
        5 => vec![
            ("image", &raw const lv_image_class),
            ("imagebutton", &raw const lv_imagebutton_class),
            ("animimg", &raw const lv_animimg_class),
            ("gif", &raw const lv_gif_class),
            ("lottie", &raw const lv_lottie_class),
            ("qrcode", &raw const lv_qrcode_class),
            ("barcode", &raw const lv_barcode_class),
        ],
        6 => vec![
            ("canvas", &raw const lv_canvas_class),
            ("arclabel", &raw const lv_arclabel_class),
        ],
        7 => vec![("msgbox", &raw const lv_msgbox_class)],
        8 => vec![
            ("list", &raw const lv_list_class),
            ("menu", &raw const lv_menu_class),
            ("win", &raw const lv_win_class),
        ],
        _ => vec![],
    }
}
