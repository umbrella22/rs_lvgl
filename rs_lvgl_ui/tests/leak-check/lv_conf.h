/* Isolated host acceptance configuration. Production defaults are unchanged. */
#include "../../../rs_lvgl_sys/vendor/include-9.6/lv_conf.h"

#undef LV_USE_STDLIB_MALLOC
#define LV_USE_STDLIB_MALLOC LV_STDLIB_BUILTIN
#define LV_MEM_SIZE (8U * 1024U * 1024U)
#define LV_MEM_ADR 0
#define LV_MEM_POOL_EXPAND_SIZE 0

#undef LV_USE_GRIDNAV
#define LV_USE_GRIDNAV 1
#undef LV_USE_QRCODE
#define LV_USE_QRCODE 1
#undef LV_USE_BARCODE
#define LV_USE_BARCODE 1
#undef LV_USE_GIF
#define LV_USE_GIF 1
#undef LV_USE_LOTTIE
#define LV_USE_LOTTIE 1
#undef LV_USE_IME_PINYIN
#define LV_USE_IME_PINYIN 1
#undef LV_FONT_SOURCE_HAN_SANS_SC_16_CJK
#define LV_FONT_SOURCE_HAN_SANS_SC_16_CJK 1
#undef LV_USE_THORVG
#define LV_USE_THORVG 1
#define LV_USE_THORVG_INTERNAL 1
#undef LV_USE_VECTOR_GRAPHIC
#define LV_USE_VECTOR_GRAPHIC 1
#undef LV_USE_MATRIX
#define LV_USE_MATRIX 1
#undef LV_USE_FLOAT
#define LV_USE_FLOAT 1
