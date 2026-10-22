use crate::{font::Font, Align, Box, Color, TextAlign};
use core::fmt;
use core::fmt::Debug;
use core::mem;
use cty::c_uint;
use paste::paste;

pub enum Themes {
    Pretty,
}

#[derive(Clone)]
pub struct Style {
    pub(crate) raw: Box<rs_lvgl_sys::lv_style_t>,
}

impl Debug for Style {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Style")
            .field("raw", &"!! LVGL lv_style_t ptr !!")
            .finish()
    }
}

impl Default for Style {
    fn default() -> Self {
        let raw = unsafe {
            let mut style = mem::MaybeUninit::<rs_lvgl_sys::lv_style_t>::uninit();
            rs_lvgl_sys::lv_style_init(style.as_mut_ptr());
            Box::new(style.assume_init())
        };
        Self { raw }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct Opacity: u32 {
        const OPA_TRANSP = rs_lvgl_sys::_lv_opacity_level_t_LV_OPA_TRANSP as u32;
        const OPA_0 = rs_lvgl_sys::_lv_opacity_level_t_LV_OPA_0 as u32;
        const OPA_10 = rs_lvgl_sys::_lv_opacity_level_t_LV_OPA_10 as u32;
        const OPA_20 = rs_lvgl_sys::_lv_opacity_level_t_LV_OPA_20 as u32;
        const OPA_30 = rs_lvgl_sys::_lv_opacity_level_t_LV_OPA_30 as u32;
        const OPA_40 = rs_lvgl_sys::_lv_opacity_level_t_LV_OPA_40 as u32;
        const OPA_50 = rs_lvgl_sys::_lv_opacity_level_t_LV_OPA_50 as u32;
        const OPA_60 = rs_lvgl_sys::_lv_opacity_level_t_LV_OPA_60 as u32;
        const OPA_70 = rs_lvgl_sys::_lv_opacity_level_t_LV_OPA_70 as u32;
        const OPA_80 = rs_lvgl_sys::_lv_opacity_level_t_LV_OPA_80 as u32;
        const OPA_90 = rs_lvgl_sys::_lv_opacity_level_t_LV_OPA_90 as u32;
        const OPA_100 = rs_lvgl_sys::_lv_opacity_level_t_LV_OPA_100 as u32;
        const OPA_COVER = rs_lvgl_sys::_lv_opacity_level_t_LV_OPA_COVER as u32;
    }
}

impl From<Opacity> for u8 {
    fn from(value: Opacity) -> u8 {
        value.bits() as u8
    }
}

bitflags! {
    pub struct GridAlign: c_uint {
        const START = rs_lvgl_sys::lv_grid_align_t_LV_GRID_ALIGN_START;
        const CENTER = rs_lvgl_sys::lv_grid_align_t_LV_GRID_ALIGN_CENTER;
        const END = rs_lvgl_sys::lv_grid_align_t_LV_GRID_ALIGN_END;
        const STRETCH = rs_lvgl_sys::lv_grid_align_t_LV_GRID_ALIGN_STRETCH;
        const SPACE_AROUND = rs_lvgl_sys::lv_grid_align_t_LV_GRID_ALIGN_SPACE_AROUND;
        const SPACE_BETWEEN = rs_lvgl_sys::lv_grid_align_t_LV_GRID_ALIGN_SPACE_BETWEEN;
        const SPACE_EVENLY = rs_lvgl_sys::lv_grid_align_t_LV_GRID_ALIGN_SPACE_EVENLY;
    }
}

impl From<GridAlign> for c_uint {
    fn from(value: GridAlign) -> Self {
        value.bits() as c_uint
    }
}

bitflags! {
    pub struct FlexAlign: c_uint {
        const START = rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_START;
        const CENTER = rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_CENTER;
        const END = rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_END;
        const SPACE_AROUND = rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_SPACE_AROUND;
        const SPACE_BETWEEN = rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_SPACE_BETWEEN;
        const SPACE_EVENLY = rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_SPACE_EVENLY;
    }
}

impl From<FlexAlign> for c_uint {
    fn from(value: FlexAlign) -> Self {
        value.bits() as c_uint
    }
}

bitflags! {
    pub struct FlexFlow: c_uint {
        const COLUMN = rs_lvgl_sys::lv_flex_flow_t_LV_FLEX_FLOW_COLUMN;
        const COLUMN_REVERSE = rs_lvgl_sys::lv_flex_flow_t_LV_FLEX_FLOW_COLUMN_REVERSE;
        const COLUMN_WRAP = rs_lvgl_sys::lv_flex_flow_t_LV_FLEX_FLOW_COLUMN_WRAP;
        const COLUMN_WRAP_REVERSE = rs_lvgl_sys::lv_flex_flow_t_LV_FLEX_FLOW_COLUMN_WRAP_REVERSE;
        const ROW = rs_lvgl_sys::lv_flex_flow_t_LV_FLEX_FLOW_ROW;
        const ROW_REVERSE = rs_lvgl_sys::lv_flex_flow_t_LV_FLEX_FLOW_ROW_REVERSE;
        const ROW_WRAP = rs_lvgl_sys::lv_flex_flow_t_LV_FLEX_FLOW_ROW_WRAP;
        const ROW_WRAP_REVERSE = rs_lvgl_sys::lv_flex_flow_t_LV_FLEX_FLOW_ROW_WRAP_REVERSE;
    }
}

impl From<FlexFlow> for c_uint {
    fn from(value: FlexFlow) -> Self {
        value.bits() as c_uint
    }
}

pub struct Layout {
    inner: u16,
}

impl Layout {
    pub fn flex() -> Self {
        Self {
            inner: rs_lvgl_sys::lv_layout_t_LV_LAYOUT_FLEX as u16,
        }
    }

    pub fn grid() -> Self {
        Self {
            inner: rs_lvgl_sys::lv_layout_t_LV_LAYOUT_GRID as u16,
        }
    }
}

impl From<Layout> for u16 {
    fn from(value: Layout) -> Self {
        value.inner
    }
}

#[derive(Clone)]
#[repr(C)]
pub struct CoordDesc<const N: usize> {
    inner: [i32; N],
    tail: i32,
}

impl<const N: usize> CoordDesc<N> {
    pub unsafe fn from_values(values: [i32; N], is_grid: bool) -> Self {
        Self {
            inner: values,
            tail: if is_grid {
                rs_lvgl_sys::LV_GRID_TEMPLATE_LAST as i32
            } else {
                0
            },
        }
    }

    pub fn values(&self) -> [i32; N] {
        self.clone().inner
    }
}

impl<const N: usize> From<&CoordDesc<N>> for *const i32 {
    fn from(value: &CoordDesc<N>) -> Self {
        value as *const _ as *const i32
    }
}

#[derive(Clone)]
pub enum StyleValues {
    Num(i32),
    Color(Color),
    Opacity(Opacity),
    None,
}

impl StyleValues {
    pub fn is_some(&self) -> bool {
        !matches!(self, StyleValues::None)
    }
}

macro_rules! gen_lv_style {
    ($func_name:ident,$vty:ty) => {
        paste! {
            #[inline]
            pub fn $func_name(&mut self, value: $vty) {
                unsafe {
                    rs_lvgl_sys::[<lv_style_ $func_name>](
                        self.raw.as_mut(),
                        value.into(),
                    );
                }
            }
        }
    };
}

macro_rules! gen_lv_style_generic {
    ($func_name:ident,$vty:ty) => {
        paste! {
            #[inline]
            pub fn $func_name<const N: usize>(&mut self, value: &$vty<N>) {
                unsafe {
                    rs_lvgl_sys::[<lv_style_ $func_name>](
                        self.raw.as_mut(),
                        value.into(),
                    );
                }
            }
        }
    };
}

impl Style {
    gen_lv_style!(set_align, Align);
    gen_lv_style!(set_arc_color, Color);
    gen_lv_style!(set_arc_opa, Opacity);
    gen_lv_style!(set_arc_rounded, bool);
    gen_lv_style!(set_arc_width, i32);
    gen_lv_style!(set_bg_color, Color);
    gen_lv_style!(set_bg_grad_color, Color);
    gen_lv_style!(set_bg_grad_stop, i32);
    gen_lv_style!(set_bg_image_opa, Opacity);
    gen_lv_style!(set_bg_image_recolor, Color);
    gen_lv_style!(set_bg_image_recolor_opa, Opacity);
    gen_lv_style!(set_bg_image_tiled, bool);
    gen_lv_style!(set_bg_main_stop, i32);
    gen_lv_style!(set_bg_opa, Opacity);
    gen_lv_style!(set_blend_mode, u8);
    gen_lv_style!(set_border_color, Color);
    gen_lv_style!(set_border_opa, Opacity);
    gen_lv_style!(set_border_post, bool);
    gen_lv_style!(set_border_side, u8);
    gen_lv_style!(set_border_width, i32);
    gen_lv_style!(set_clip_corner, bool);
    gen_lv_style!(set_color_filter_opa, Opacity);
    gen_lv_style!(set_flex_flow, FlexFlow);
    gen_lv_style!(set_flex_grow, u8);
    gen_lv_style!(set_flex_main_place, FlexAlign);
    gen_lv_style!(set_flex_cross_place, FlexAlign);
    gen_lv_style!(set_flex_track_place, FlexAlign);
    gen_lv_style!(set_grid_cell_column_pos, i32);
    gen_lv_style!(set_grid_cell_column_span, i32);
    gen_lv_style!(set_grid_cell_row_pos, i32);
    gen_lv_style!(set_grid_cell_row_span, i32);
    gen_lv_style!(set_grid_cell_x_align, GridAlign);
    gen_lv_style!(set_grid_cell_y_align, GridAlign);
    gen_lv_style!(set_grid_column_align, GridAlign);
    gen_lv_style_generic!(set_grid_column_dsc_array, CoordDesc);
    gen_lv_style!(set_grid_row_align, GridAlign);
    gen_lv_style_generic!(set_grid_row_dsc_array, CoordDesc);
    gen_lv_style!(set_height, i32);
    gen_lv_style!(set_image_opa, Opacity);
    gen_lv_style!(set_image_recolor, Color);
    gen_lv_style!(set_image_recolor_opa, Opacity);
    gen_lv_style!(set_layout, Layout);
    gen_lv_style!(set_length, i32);
    gen_lv_style!(set_line_color, Color);
    gen_lv_style!(set_line_dash_gap, i32);
    gen_lv_style!(set_line_dash_width, i32);
    gen_lv_style!(set_line_opa, Opacity);
    gen_lv_style!(set_line_rounded, bool);
    gen_lv_style!(set_line_width, i32);
    gen_lv_style!(set_max_height, i32);
    gen_lv_style!(set_max_width, i32);
    gen_lv_style!(set_min_height, i32);
    gen_lv_style!(set_min_width, i32);
    gen_lv_style!(set_opa, Opacity);
    gen_lv_style!(set_outline_color, Color);
    gen_lv_style!(set_outline_opa, Opacity);
    gen_lv_style!(set_outline_pad, i32);
    gen_lv_style!(set_outline_width, i32);
    gen_lv_style!(set_pad_bottom, i32);
    gen_lv_style!(set_pad_column, i32);
    gen_lv_style!(set_pad_left, i32);
    gen_lv_style!(set_pad_right, i32);
    gen_lv_style!(set_pad_row, i32);
    gen_lv_style!(set_pad_top, i32);
    gen_lv_style!(set_radius, i32);
    gen_lv_style!(set_shadow_color, Color);
    gen_lv_style!(set_shadow_offset_x, i32);
    gen_lv_style!(set_shadow_offset_y, i32);
    gen_lv_style!(set_shadow_opa, Opacity);
    gen_lv_style!(set_shadow_spread, i32);
    gen_lv_style!(set_shadow_width, i32);
    gen_lv_style!(set_text_align, TextAlign);
    gen_lv_style!(set_text_color, Color);
    gen_lv_style!(set_text_decor, u8);
    gen_lv_style!(set_text_font, Font);
    gen_lv_style!(set_text_letter_space, i32);
    gen_lv_style!(set_text_line_space, i32);
    gen_lv_style!(set_text_opa, Opacity);
    gen_lv_style!(set_transform_height, i32);
    gen_lv_style!(set_transform_pivot_x, i32);
    gen_lv_style!(set_transform_pivot_y, i32);
    gen_lv_style!(set_transform_rotation, i32);
    gen_lv_style!(set_transform_scale_x, i32);
    gen_lv_style!(set_transform_scale_y, i32);
    gen_lv_style!(set_transform_skew_x, i32);
    gen_lv_style!(set_transform_skew_y, i32);
    gen_lv_style!(set_transform_width, i32);
    gen_lv_style!(set_translate_x, i32);
    gen_lv_style!(set_translate_y, i32);
    gen_lv_style!(set_width, i32);
    gen_lv_style!(set_x, i32);
    gen_lv_style!(set_y, i32);
}
