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

    /// Build the LVGL descriptor for a static RGB565 source.
    pub(crate) fn descriptor(self) -> Option<Box<rs_lvgl_sys::lv_image_dsc_t>> {
        let ImageSource::Rgb565 {
            width,
            height,
            data,
        } = self
        else {
            return None;
        };
        let expected = width as usize * height as usize * 2;
        if width == 0 || height == 0 || data.len() != expected {
            return None;
        }
        let mut descriptor = rs_lvgl_sys::lv_image_dsc_t::default();
        descriptor
            .header
            .set_magic(rs_lvgl_sys::LV_IMAGE_HEADER_MAGIC);
        descriptor
            .header
            .set_cf(rs_lvgl_sys::lv_color_format_t_LV_COLOR_FORMAT_RGB565);
        descriptor.header.set_w(width as u32);
        descriptor.header.set_h(height as u32);
        descriptor.header.set_stride(width as u32 * 2);
        descriptor.data_size = expected as u32;
        descriptor.data = data.as_ptr();
        Some(Box::new(descriptor))
    }

    pub(crate) fn ptr(
        self,
        descriptor: Option<&rs_lvgl_sys::lv_image_dsc_t>,
    ) -> *const cty::c_void {
        match self {
            ImageSource::Text(text) => text.as_ptr().cast(),
            ImageSource::Rgb565 { .. } => descriptor
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
            ImageSource::Raw(raw) => f.debug_tuple("Raw").field(&raw.as_ptr()).finish(),
        }
    }
}
