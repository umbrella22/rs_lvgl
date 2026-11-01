//! Stable buffers and private native sub-objects held by mounted nodes.

use alloc::{boxed::Box, vec::Vec};
use core::ptr::NonNull;
use cstr_core::CString;
use rs_lvgl_sys::lv_obj_t;

/// Native buffers LVGL references without copying, owned by a mounted node.
///
/// Heap buffers keep their address when the owning `MountedNode` moves.
/// Replacements are installed in LVGL before the previous buffer is dropped.
#[derive(Default)]
// Descriptors stay boxed: LVGL holds pointers to each one, and boxing keeps
// them stable while the vectors grow.
#[allow(clippy::vec_box)]
pub(crate) enum Retained {
    #[cfg(feature = "widget-image")]
    LoadedImage(#[allow(dead_code)] crate::ImageResource),
    #[cfg(feature = "widget-gif")]
    Gif {
        _data: alloc::rc::Rc<[u8]>,
        _descriptor: Box<rs_lvgl_sys::lv_image_dsc_t>,
    },
    #[cfg(feature = "widget-lottie")]
    Lottie(#[allow(dead_code)] crate::graphics::DrawBuffer),
    #[cfg(feature = "widget-canvas")]
    Canvas(#[allow(dead_code)] crate::graphics::DrawBuffer),
    #[default]
    None,
    // The fields are only held, never read: LVGL reads them through the
    // pointers installed when they were created.
    /// A generated image descriptor (`lv_image_set_src` stores the pointer).
    Image(#[allow(dead_code)] Box<rs_lvgl_sys::lv_image_dsc_t>),
    /// Line points (`lv_line_set_points` stores the pointer).
    Points(#[allow(dead_code)] Vec<rs_lvgl_sys::lv_point_precise_t>),
    /// A button matrix map (`lv_buttonmatrix_set_map` stores the array and
    /// the strings it points to).
    Map {
        _texts: Vec<CString>,
        _pointers: Vec<*const cty::c_char>,
    },
    /// Generated image descriptors referenced by an image button
    /// (`lv_imagebutton_set_src` stores the pointers).
    Images(#[allow(dead_code)] Vec<Box<rs_lvgl_sys::lv_image_dsc_t>>),
    /// Animated image frames (`lv_animimg_set_src` stores the pointer
    /// array, and the image stores the current frame's pointer).
    Frames {
        _descriptors: Vec<Box<rs_lvgl_sys::lv_image_dsc_t>>,
        _pointers: Vec<*const cty::c_void>,
    },
    /// Calendar highlighted dates (`lv_calendar_set_highlighted_dates`
    /// stores the pointer).
    #[cfg(feature = "widget-calendar")]
    Dates(#[allow(dead_code)] Vec<rs_lvgl_sys::lv_calendar_date_t>),
}

/// Native sub-objects created together with a widget.
#[derive(Clone, Copy, Default)]
pub(crate) struct Parts {
    #[cfg(feature = "widget-ime-pinyin")]
    pub(crate) input: Option<NonNull<lv_obj_t>>,
    #[cfg(feature = "widget-ime-pinyin")]
    pub(crate) ime: Option<NonNull<lv_obj_t>>,
    /// A label created by the wrapper and owned by the widget (the text of a
    /// button or message box). It is always the first child of the object
    /// that holds the node's children.
    pub(crate) label: Option<NonNull<lv_obj_t>>,
    /// Where children are mounted when it is not the widget itself.
    pub(crate) content: Option<NonNull<lv_obj_t>>,
}

impl Parts {
    /// The object that holds the node's children.
    pub(crate) fn children_parent(&self, obj: NonNull<lv_obj_t>) -> NonNull<lv_obj_t> {
        self.content.unwrap_or(obj)
    }

    /// Index of the first application child in [`Parts::children_parent`].
    pub(crate) fn first_child_index(&self) -> i32 {
        i32::from(self.label.is_some())
    }
}
