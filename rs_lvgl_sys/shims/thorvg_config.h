/* ThorVG 0.15.3 configuration for the vendored LVGL 9.6 provider.
 * Equivalent to LVGL's Meson-generated config.h, which is not a package input.
 * LVGL feature values come from the same effective lv_conf.h as the C build.
 */
#ifndef TVG_CONFIG_H
#define TVG_CONFIG_H

#include "lvgl_public.h"

#define THORVG_SW_RASTER_SUPPORT 1
#define THORVG_SVG_LOADER_SUPPORT LV_USE_LOTTIE
#define THORVG_LOTTIE_LOADER_SUPPORT LV_USE_LOTTIE
#define THORVG_VERSION_STRING "0.15.3"

#if LV_DRAW_SW_DRAW_UNIT_CNT > 1
#define THORVG_THREAD_SUPPORT
#endif

#endif /* TVG_CONFIG_H */
