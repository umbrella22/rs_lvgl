/* Currently a pure include point for the vendored LVGL build; place any
 * additional C shims the Rust side needs here. */
#include "rs_lvgl_sys.h"

#if LV_USE_LOTTIE
#include "widgets/lottie/lv_lottie_private.h"
#include "misc/lv_anim_private.h"
#include "misc/cache/lv_cache.h"
#include <math.h>
#include <limits.h>

/* LVGL's void source setter ignores decoder errors and uses unchecked frame
 * metadata. Check every result before changing its native animation clock. */
bool lv_rs_lottie_set_source(lv_obj_t *obj, const char *source, uint32_t length)
{
    lv_lottie_t *lottie = (lv_lottie_t *)obj;
    if (!source || !length || !lottie->tvg_anim || !lottie->tvg_paint ||
        !lottie->tvg_canvas || !lottie->anim) return false;
    if (tvg_picture_load_data(lottie->tvg_paint, source, length, "lottie", true) != TVG_RESULT_SUCCESS)
        return false;
    float frames = 0.0f, duration = 0.0f;
    if (tvg_animation_get_total_frame(lottie->tvg_anim, &frames) != TVG_RESULT_SUCCESS ||
        tvg_animation_get_duration(lottie->tvg_anim, &duration) != TVG_RESULT_SUCCESS ||
        !isfinite(frames) || !isfinite(duration) || frames < 1.0f ||
        (double)frames > INT32_MAX || duration <= 0.0f ||
        (double)duration * 1000.0 + 0.5 > INT32_MAX) return false;
    lv_draw_buf_t *buffer = lv_canvas_get_draw_buf(obj);
    if (!buffer || tvg_picture_set_size(lottie->tvg_paint, buffer->header.w,
                                       buffer->header.h) != TVG_RESULT_SUCCESS) return false;
    uint32_t ms = (uint32_t)((double)duration * 1000.0 + 0.5);
    if (!ms) return false;
    lv_anim_set_duration(lottie->anim, ms);
    lottie->anim->act_time = 0;
    lottie->anim->end_value = (int32_t)frames;
    lottie->anim->reverse_play_in_progress = false;
    lottie->last_rendered_time = 0;
    /* A new widget has not been laid out yet, so the animation callback's
     * visibility check can suppress frame zero. Finish the initial draw here,
     * matching the upstream source setter, and report renderer failures. */
    lv_draw_buf_clear(buffer, NULL);
    lv_image_cache_drop(lv_image_get_src(obj));
    tvg_animation_set_frame(lottie->tvg_anim, 0);
    if (tvg_canvas_update(lottie->tvg_canvas) != TVG_RESULT_SUCCESS ||
        tvg_canvas_draw(lottie->tvg_canvas) != TVG_RESULT_SUCCESS ||
        tvg_canvas_sync(lottie->tvg_canvas) != TVG_RESULT_SUCCESS) return false;
    lv_obj_invalidate(obj);
    return true;
}
#endif

#if LV_USE_BARCODE
#include "libs/barcode/code128.h"
#include <limits.h>
uint32_t lv_rs_barcode_width(const char *data, lv_barcode_encoding_t encoding, uint16_t scale)
{
    if (!data || !data[0] || !scale) return 0;
    /* The encoder estimate uses 11 * (len * 11 / 10) + 55, and its internal
     * indices are int. Bound the input before doing that arithmetic. */
    if (lv_strlen(data) > (INT32_MAX - 55) / 121) return 0;
    size_t capacity = code128_estimate_len(data);
    char *pattern = lv_malloc(capacity);
    if (!pattern) return 0;
    size_t bars = encoding == LV_BARCODE_ENCODING_CODE128_RAW
        ? code128_encode_raw(data, pattern, capacity)
        : code128_encode_gs1(data, pattern, capacity);
    lv_free(pattern);
    /* Native image width is a 16-bit field. Reject before canvas allocation. */
    if (!bars || bars > UINT16_MAX / scale) return 0;
    return (uint32_t)bars * scale;
}
#endif
