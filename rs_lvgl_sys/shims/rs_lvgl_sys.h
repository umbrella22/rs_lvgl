#ifndef LVGL_API_H
#define LVGL_API_H

#ifdef __cplusplus
extern "C" {
#endif

/* Vendored LVGL main header (rs_lvgl_sys/vendor/lvgl-9.6/lvgl.h, on the
 * include path through build.rs). */
#include "lvgl.h"

#if LV_USE_BARCODE
/* Returns the encoded pixel width, or zero for invalid/unrepresentable data. */
uint32_t lv_rs_barcode_width(const char *data, lv_barcode_encoding_t encoding, uint16_t scale);
#endif

#if LV_USE_LOTTIE
/* The source must be NUL terminated; length excludes that terminator. */
bool lv_rs_lottie_set_source(lv_obj_t *obj, const char *source, uint32_t length);
#endif

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /*LVGL_API*/
