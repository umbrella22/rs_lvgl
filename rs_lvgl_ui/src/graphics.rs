//! Stable native drawing storage and owned pixel snapshots.
use crate::UiError;
use alloc::vec::Vec;
use core::ptr::NonNull;

pub(crate) struct DrawBuffer(pub(crate) NonNull<rs_lvgl_sys::lv_draw_buf_t>);
impl DrawBuffer {
    pub(crate) fn stride(width: u16, height: u16) -> Result<u32, UiError> {
        if width == 0 || height == 0 {
            return Err(UiError::InvalidProperty);
        }
        // SAFETY: LVGL is initialized; the platform's draw-buffer handlers
        // define the alignment/stride required by its renderer.
        let stride = unsafe {
            rs_lvgl_sys::lv_draw_buf_width_to_stride(
                u32::from(width),
                rs_lvgl_sys::lv_color_format_t_LV_COLOR_FORMAT_ARGB8888,
            )
        };
        // lv_image_header_t stores stride in 16 bits. Reject truncation and
        // leave room for the native allocation's alignment padding.
        if stride < u32::from(width) * 4
            || stride > u16::MAX as u32
            || (stride as usize)
                .checked_mul(height as usize)
                .is_none_or(|n| n > isize::MAX as usize - u16::MAX as usize)
        {
            return Err(UiError::InvalidProperty);
        }
        Ok(stride)
    }
    #[cfg(any(feature = "widget-canvas", feature = "widget-lottie"))]
    pub(crate) fn new(width: u16, height: u16) -> Result<Self, UiError> {
        let stride = Self::stride(width, height)?;
        // SAFETY: dimensions, stride and capacity are checked before native
        // allocation. LVGL provides the required alignment and allocator pair.
        NonNull::new(unsafe {
            rs_lvgl_sys::lv_draw_buf_create(
                width.into(),
                height.into(),
                rs_lvgl_sys::lv_color_format_t_LV_COLOR_FORMAT_ARGB8888,
                stride,
            )
        })
        .map(Self)
        .ok_or(UiError::Allocation)
    }
}
impl Drop for DrawBuffer {
    fn drop(&mut self) {
        // SAFETY: mounted users detach/delete their object before dropping the
        // buffer. Snapshot buffers have no retained native users.
        unsafe { rs_lvgl_sys::lv_draw_buf_destroy(self.0.as_ptr()) };
    }
}

/// Packed RGBA8888 pixels copied from LVGL. This value survives unmount and
/// native deinitialization. Rows contain exactly `width * 4` bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}
impl Snapshot {
    pub(crate) fn take(obj: NonNull<rs_lvgl_sys::lv_obj_t>) -> Result<Self, UiError> {
        // SAFETY: Ui borrows a live root, and snapshot completes all draw tasks
        // before returning. The native allocation is freed after copying.
        let buffer = DrawBuffer(
            NonNull::new(unsafe {
                rs_lvgl_sys::lv_obj_update_layout(obj.as_ptr());
                rs_lvgl_sys::lv_snapshot_take(
                    obj.as_ptr(),
                    rs_lvgl_sys::lv_color_format_t_LV_COLOR_FORMAT_ARGB8888,
                )
            })
            .ok_or(UiError::Allocation)?,
        );
        unsafe {
            let raw = buffer.0.as_ref();
            let width = raw.header.w();
            let height = raw.header.h();
            let stride = raw.header.stride() as usize;
            let row = width as usize * 4;
            let len = stride
                .checked_mul(height as usize)
                .ok_or(UiError::InvalidProperty)?;
            if row > stride
                || len > raw.data_size as usize
                || len > isize::MAX as usize
                || raw.data.is_null()
            {
                return Err(UiError::InvalidProperty);
            }
            let data = core::slice::from_raw_parts(raw.data, len);
            let mut pixels = Vec::with_capacity(row * height as usize);
            for y in 0..height as usize {
                for bgra in data[y * stride..y * stride + row].as_chunks::<4>().0 {
                    pixels.extend_from_slice(&[bgra[2], bgra[1], bgra[0], bgra[3]]);
                }
            }
            Ok(Self {
                width,
                height,
                pixels,
            })
        }
    }
}
