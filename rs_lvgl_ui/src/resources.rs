//! Loaded, immutable resources for synchronous software rendering without image
//! caches. Native cache/renderer reconfiguration while resources are mounted is
//! outside this contract. All native operations remain on the UI thread.
use alloc::{boxed::Box, rc::Rc};
use core::{fmt, ptr::NonNull};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceError {
    InvalidLayout,
    UnsupportedConfiguration,
}

impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidLayout => "invalid loaded image layout",
            Self::UnsupportedConfiguration => {
                "loaded resource is unsupported by the effective native drawing/cache configuration"
            }
        })
    }
}

/// Capability of the effective compiled native configuration. Runtime changes
/// through raw LVGL APIs must also preserve these constraints.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceSupport {
    pub synchronous_software: bool,
    pub image_caches_disabled: bool,
}

impl ResourceSupport {
    pub const fn supported(self) -> bool {
        self.synchronous_software && self.image_caches_disabled
    }
}

pub fn resource_support() -> ResourceSupport {
    // SAFETY: this shim reads compile-time constants and requires no LVGL state.
    let bits = unsafe { rs_lvgl_sys::lv_rs_loaded_resource_support() };
    ResourceSupport {
        synchronous_software: bits & 1 != 0,
        image_caches_disabled: bits & 2 != 0,
    }
}

pub(crate) fn validate_support() -> Result<(), crate::UiError> {
    if resource_support().supported() {
        Ok(())
    } else {
        Err(crate::UiError::InvalidProperty)
    }
}

#[cfg(feature = "widget-image")]
struct ImageData {
    descriptor: rs_lvgl_sys::lv_image_dsc_t,
    _pixels: Rc<[u8]>,
}

/// A stable native pixel descriptor and shared immutable pixels. Clones share the
/// allocation; a mounted node retains a clone independently of App::view.
#[cfg(feature = "widget-image")]
#[derive(Clone)]
pub struct ImageResource(Rc<ImageData>);

#[cfg(feature = "widget-image")]
impl ImageResource {
    /// Accept already loaded RGB565 rows. `stride` is bytes per row, including
    /// padding; the slice contains exactly `stride * height` bytes. Data and
    /// stride must satisfy the effective native alignment configuration.
    pub fn rgb565(
        width: u16,
        height: u16,
        stride: u32,
        pixels: Rc<[u8]>,
    ) -> Result<Self, ResourceError> {
        if rs_lvgl_sys::LV_DRAW_SW_SUPPORT_RGB565 == 0 {
            return Err(ResourceError::UnsupportedConfiguration);
        }
        Self::from_pixels(width, height, stride, pixels, rs_lvgl::ColorFormat::Rgb565)
    }

    /// Accept native RGB888 bytes (B,G,R) with an explicit byte stride.
    /// Pixels and the stable descriptor are shared across clones and native users.
    pub fn rgb888(width: u16, height: u16, stride: u32, pixels: Rc<[u8]>) -> Result<Self, ResourceError> {
        if rs_lvgl_sys::LV_DRAW_SW_SUPPORT_RGB888 == 0 {
            return Err(ResourceError::UnsupportedConfiguration);
        }
        Self::from_pixels(width, height, stride, pixels, rs_lvgl::ColorFormat::Rgb888)
    }

    fn from_pixels(width: u16, height: u16, stride: u32, pixels: Rc<[u8]>, format: rs_lvgl::ColorFormat)
        -> Result<Self, ResourceError>
    {
        if !resource_support().supported() { return Err(ResourceError::UnsupportedConfiguration); }
        let bytes = (stride as usize)
            .checked_mul(height as usize)
            .ok_or(ResourceError::InvalidLayout)?;
        let alignment = rs_lvgl_sys::LV_DRAW_BUF_ALIGN as usize;
        let row_alignment = rs_lvgl_sys::LV_DRAW_BUF_STRIDE_ALIGN as u32;
        if width == 0
            || height == 0
            || stride < u32::from(width) * format.bytes_per_pixel() as u32
            || stride > u16::MAX as u32
            || (format == rs_lvgl::ColorFormat::Rgb565 && stride % 2 != 0)
            || row_alignment == 0
            || stride % row_alignment != 0
            || alignment == 0
            || pixels.as_ptr() as usize % alignment != 0
            || pixels.len() != bytes
            || bytes > u32::MAX as usize
        {
            return Err(ResourceError::InvalidLayout);
        }
        let mut descriptor = rs_lvgl_sys::lv_image_dsc_t::default();
        descriptor
            .header
            .set_magic(rs_lvgl_sys::LV_IMAGE_HEADER_MAGIC);
        descriptor
            .header
            .set_cf(format.to_raw());
        descriptor.header.set_w(u32::from(width));
        descriptor.header.set_h(u32::from(height));
        descriptor.header.set_stride(stride);
        descriptor.data_size = bytes as u32;
        descriptor.data = pixels.as_ptr();
        Ok(Self(Rc::new(ImageData {
            descriptor,
            _pixels: pixels,
        })))
    }

    /// Non-owning descriptor pointer. Native users must retain a clone until
    /// all use finishes; passing this pointer to C remains an unsafe operation.
    pub fn as_ptr(&self) -> *const rs_lvgl_sys::lv_image_dsc_t {
        &self.0.descriptor
    }
}

#[cfg(feature = "widget-image")]
impl PartialEq for ImageResource {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
#[cfg(feature = "widget-image")]
impl Eq for ImageResource {}
#[cfg(feature = "widget-image")]
impl fmt::Debug for ImageResource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ImageResource")
            .field(&self.as_ptr())
            .finish()
    }
}

struct FontData {
    raw: NonNull<rs_lvgl_sys::lv_font_t>,
    destroy: unsafe extern "C" fn(*mut rs_lvgl_sys::lv_font_t),
    _backing: Box<dyn core::any::Any>,
}

impl Drop for FontData {
    fn drop(&mut self) {
        // SAFETY: from_raw_owned transferred the descriptor with its matching
        // destructor. Rc releases it once, while _backing is still alive.
        unsafe { (self.destroy)(self.raw.as_ptr()) };
    }
}

/// A loaded native font with a matching destructor and retained backing data.
/// Static Font/Style/Theme remain Copy; this resource is Clone and thread-local.
#[derive(Clone)]
pub struct FontResource(Rc<FontData>);

impl FontResource {
    /// Transfer an already loaded native font and its backing data.
    ///
    /// # Safety
    /// `raw` must be a live initialized font on this UI thread, with valid
    /// callbacks/tables for every glyph it can expose. `backing` must retain all
    /// non-static data those callbacks or tables use at stable addresses. The
    /// caller transfers sole destruction responsibility: aliases must neither
    /// free nor invalidate it. `destroy` must match its native allocator/loader,
    /// release exactly that font, and never unwind or reenter UI operations.
    /// The backing value's destructor must not unwind or reenter UI either.
    /// For a binfont, use lv_binfont_destroy and retain its buffer when required
    /// by its loading mode. Release all resource clones before native deinit.
    /// No binary parser or filesystem driver is implied by this adapter.
    pub unsafe fn from_raw_owned<T: 'static>(
        raw: NonNull<rs_lvgl_sys::lv_font_t>,
        backing: T,
        destroy: unsafe extern "C" fn(*mut rs_lvgl_sys::lv_font_t),
    ) -> Self {
        Self(Rc::new(FontData {
            raw,
            destroy,
            _backing: Box::new(backing),
        }))
    }

    pub fn as_ptr(&self) -> *const rs_lvgl_sys::lv_font_t {
        self.0.raw.as_ptr()
    }
}
impl PartialEq for FontResource {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for FontResource {}
impl fmt::Debug for FontResource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("FontResource").field(&self.as_ptr()).finish()
    }
}
