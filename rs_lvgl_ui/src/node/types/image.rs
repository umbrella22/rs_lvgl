//! Static image sources and stable native descriptors.

use alloc::boxed::Box;
use core::fmt;
use cstr_core::CStr;

/// Where a static image comes from.
#[derive(Clone, Copy)]
pub enum ImageSource {
    /// A null-terminated LVGL path or symbol string that remains valid for the
    /// program's lifetime.
    Text(&'static CStr),
    /// A static RGB565 image. The descriptor is built when the node is
    /// mounted and kept alive by the mounted tree.
    Rgb565 {
        width: u16,
        height: u16,
        data: &'static [u8],
    },
    /// Static B,G,R pixels, with byte stride including row padding.
    /// The data contains exactly `stride * height` initialized bytes.
    Rgb888 {
        width: u16,
        height: u16,
        stride: u32,
        data: &'static [u8],
    },
    /// A raw LVGL image descriptor, only constructible through the unsafe
    /// [`ImageSource::from_raw`].
    Raw(RawImage),
}

/// An LVGL image descriptor pointer whose validity was asserted by
/// [`ImageSource::from_raw`].
///
/// The pointer field is private so a safe caller cannot construct a raw
/// image source with an arbitrary pointer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RawImage(*const rs_lvgl_sys::lv_image_dsc_t);

impl RawImage {
    /// The wrapped descriptor pointer.
    pub const fn as_ptr(self) -> *const rs_lvgl_sys::lv_image_dsc_t {
        self.0
    }
}

impl ImageSource {
    /// Wrap an LVGL image descriptor.
    ///
    /// # Safety
    ///
    /// The descriptor and its pixel data must stay valid and immutable for as
    /// long as an image object references them, including after this `Node` has
    /// been reconciled away.
    pub const unsafe fn from_raw(raw: *const rs_lvgl_sys::lv_image_dsc_t) -> Self {
        Self::Raw(RawImage(raw))
    }

    /// A static RGB565 image. `data` must contain `width * height * 2` bytes.
    pub const fn rgb565(width: u16, height: u16, data: &'static [u8]) -> Self {
        Self::Rgb565 {
            width,
            height,
            data,
        }
    }

    /// Packed static native RGB888 (B,G,R) pixels.
    pub const fn rgb888(width: u16, height: u16, data: &'static [u8]) -> Self {
        Self::rgb888_with_stride(width, height, width as u32 * 3, data)
    }

    /// Static native RGB888 (B,G,R) pixels with explicit bytes per row.
    pub const fn rgb888_with_stride(
        width: u16,
        height: u16,
        stride: u32,
        data: &'static [u8],
    ) -> Self {
        Self::Rgb888 {
            width,
            height,
            stride,
            data,
        }
    }

    pub(crate) fn is_pixels(self) -> bool {
        matches!(self, Self::Rgb565 { .. } | Self::Rgb888 { .. })
    }

    pub(crate) fn valid_pixels(self) -> bool {
        self.pixel_layout().is_some()
    }

    fn pixel_layout(
        self,
    ) -> Option<(u16, u16, u32, &'static [u8], rs_lvgl_sys::lv_color_format_t)> {
        let (width, height, stride, data, format, bpp) = match self {
            Self::Rgb565 {
                width,
                height,
                data,
            } => (
                width,
                height,
                width as u32 * 2,
                data,
                rs_lvgl_sys::lv_color_format_t_LV_COLOR_FORMAT_RGB565,
                2,
            ),
            Self::Rgb888 {
                width,
                height,
                stride,
                data,
            } => (
                width,
                height,
                stride,
                data,
                rs_lvgl_sys::lv_color_format_t_LV_COLOR_FORMAT_RGB888,
                3,
            ),
            _ => return None,
        };
        let expected = (stride as usize).checked_mul(height as usize)?;
        if width == 0
            || height == 0
            || stride < width as u32 * bpp
            || stride > u16::MAX as u32
            || expected != data.len()
            || expected > u32::MAX as usize
        {
            return None;
        }
        Some((width, height, stride, data, format))
    }

    /// Build a stable native descriptor only after layout validation.
    pub(crate) fn descriptor(self) -> Option<Box<rs_lvgl_sys::lv_image_dsc_t>> {
        let (width, height, stride, data, format) = self.pixel_layout()?;
        let mut descriptor = rs_lvgl_sys::lv_image_dsc_t::default();
        descriptor
            .header
            .set_magic(rs_lvgl_sys::LV_IMAGE_HEADER_MAGIC);
        descriptor.header.set_cf(format);
        descriptor.header.set_w(width as u32);
        descriptor.header.set_h(height as u32);
        descriptor.header.set_stride(stride);
        descriptor.data_size = data.len() as u32;
        descriptor.data = data.as_ptr();
        Some(Box::new(descriptor))
    }

    pub(crate) fn ptr(
        self,
        descriptor: Option<&rs_lvgl_sys::lv_image_dsc_t>,
    ) -> *const cty::c_void {
        match self {
            ImageSource::Text(text) => text.as_ptr().cast(),
            ImageSource::Rgb565 { .. } | ImageSource::Rgb888 { .. } => descriptor
                .map(|descriptor| descriptor as *const _ as *const cty::c_void)
                .unwrap_or(core::ptr::null()),
            ImageSource::Raw(raw) => raw.as_ptr().cast(),
        }
    }
}

impl PartialEq for ImageSource {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ImageSource::Text(left), ImageSource::Text(right)) => left.as_ptr() == right.as_ptr(),
            (
                ImageSource::Rgb565 {
                    width: left_width,
                    height: left_height,
                    data: left_data,
                },
                ImageSource::Rgb565 {
                    width: right_width,
                    height: right_height,
                    data: right_data,
                },
            ) => {
                left_width == right_width
                    && left_height == right_height
                    && left_data.as_ptr() == right_data.as_ptr()
                    && left_data.len() == right_data.len()
            }
            (
                ImageSource::Rgb888 {
                    width: lw,
                    height: lh,
                    stride: ls,
                    data: ld,
                },
                ImageSource::Rgb888 {
                    width: rw,
                    height: rh,
                    stride: rs,
                    data: rd,
                },
            ) => {
                lw == rw
                    && lh == rh
                    && ls == rs
                    && ld.as_ptr() == rd.as_ptr()
                    && ld.len() == rd.len()
            }
            (ImageSource::Raw(left), ImageSource::Raw(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for ImageSource {}

impl fmt::Debug for ImageSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageSource::Text(text) => f.debug_tuple("Text").field(text).finish(),
            ImageSource::Rgb565 {
                width,
                height,
                data,
            } => f
                .debug_struct("Rgb565")
                .field("width", width)
                .field("height", height)
                .field("bytes", &data.len())
                .finish(),
            ImageSource::Rgb888 {
                width,
                height,
                stride,
                data,
            } => f
                .debug_struct("Rgb888")
                .field("width", width)
                .field("height", height)
                .field("stride", stride)
                .field("bytes", &data.len())
                .finish(),
            ImageSource::Raw(raw) => f.debug_tuple("Raw").field(&raw.as_ptr()).finish(),
        }
    }
}
