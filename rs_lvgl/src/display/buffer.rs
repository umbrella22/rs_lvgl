use crate::sys;
use core::{fmt, ptr::NonNull};

/// Native pixel layout. RGB888 bytes are B,G,R; swapped RGB565 bytes are big endian.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorFormat {
    Rgb565,
    Rgb565Swapped,
    Rgb888,
}

impl ColorFormat {
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgb565 | Self::Rgb565Swapped => 2,
            Self::Rgb888 => 3,
        }
    }

    pub const fn to_raw(self) -> sys::lv_color_format_t {
        match self {
            Self::Rgb565 => sys::lv_color_format_t_LV_COLOR_FORMAT_RGB565,
            Self::Rgb565Swapped => sys::lv_color_format_t_LV_COLOR_FORMAT_RGB565_SWAPPED,
            Self::Rgb888 => sys::lv_color_format_t_LV_COLOR_FORMAT_RGB888,
        }
    }

    pub(crate) fn from_raw(raw: sys::lv_color_format_t) -> Option<Self> {
        match raw {
            sys::lv_color_format_t_LV_COLOR_FORMAT_RGB565 => Some(Self::Rgb565),
            sys::lv_color_format_t_LV_COLOR_FORMAT_RGB565_SWAPPED => Some(Self::Rgb565Swapped),
            sys::lv_color_format_t_LV_COLOR_FORMAT_RGB888 => Some(Self::Rgb888),
            _ => None,
        }
    }
}

/// Initialized storage retained until native display/Canvas deletion.
///
/// An owned allocation is freed by its Rust allocator after native references
/// retire. A static borrow is never freed. Neither form is released on ordinary
/// display-handle drop. Native pointer alignment is checked before attachment.
///
/// ```compile_fail
/// fn short_borrow(bytes: &mut [u8]) -> rs_lvgl::PixelBuffer {
///     rs_lvgl::PixelBuffer::Static(bytes)
/// }
/// ```
pub enum PixelBuffer {
    Static(&'static mut [u8]),
    #[cfg(feature = "alloc")]
    Owned(alloc::boxed::Box<[u8]>),
}

impl PixelBuffer {
    pub fn len(&self) -> usize {
        match self {
            Self::Static(bytes) => bytes.len(),
            #[cfg(feature = "alloc")]
            Self::Owned(bytes) => bytes.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut u8 {
        match self {
            Self::Static(bytes) => bytes.as_mut_ptr(),
            #[cfg(feature = "alloc")]
            Self::Owned(bytes) => bytes.as_mut_ptr(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisplayConfigError {
    InvalidDimensions,
    InvalidStride,
    InvalidInvalidationAlignment,
    BufferTooSmall,
    UnalignedBuffer,
    MissingBuffer,
    UnsupportedFormat,
    NativeAllocation,
    NativeConfiguration,
}

impl fmt::Display for DisplayConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "display configuration: {:?}", self)
    }
}

/// Metadata for a native buffer. A pointer is not a borrow or proof of liveness.
pub struct BufferInfo {
    pub(crate) data: NonNull<u8>,
    pub(crate) len: usize,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) stride: usize,
    pub(crate) format: ColorFormat,
}

impl BufferInfo {
    pub fn data(&self) -> NonNull<u8> {
        self.data
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn stride(&self) -> usize {
        self.stride
    }
    pub fn format(&self) -> ColorFormat {
        self.format
    }

    /// View the initialized storage, including row padding.
    ///
    /// # Safety
    /// The native display/object and storage must remain alive for the returned
    /// borrow. No LVGL rendering, DMA writer, buffer replacement or deletion may
    /// mutate/invalidate it during the borrow. Do not reenter LVGL while reading
    /// it. Metadata retained after a callback does not extend the native lifetime.
    pub unsafe fn as_bytes(&self) -> &[u8] {
        // SAFETY: the caller establishes native liveness and exclusive writers.
        unsafe { core::slice::from_raw_parts(self.data.as_ptr(), self.len) }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BufferLayout {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: ColorFormat,
}

impl BufferLayout {
    pub fn new(
        width: u32,
        height: u32,
        format: ColorFormat,
        stride: Option<u32>,
    ) -> Result<Self, DisplayConfigError> {
        // All three fields are 16-bit in lv_image_header_t, including draw buffers.
        if width == 0 || height == 0 || width > u16::MAX.into() || height > u16::MAX.into() {
            return Err(DisplayConfigError::InvalidDimensions);
        }
        let row = width
            .checked_mul(format.bytes_per_pixel() as u32)
            .ok_or(DisplayConfigError::InvalidStride)?;
        // SAFETY: initialized LVGL supplies its draw-buffer stride handler.
        let minimum = unsafe { sys::lv_draw_buf_width_to_stride(width, format.to_raw()) };
        let stride = stride.unwrap_or(minimum);
        let pixel_alignment = if format == ColorFormat::Rgb888 { 1 } else { 2 };
        if minimum < row
            || stride < minimum
            || stride > u16::MAX.into()
            || stride % sys::LV_DRAW_BUF_STRIDE_ALIGN != 0
            || stride % pixel_alignment != 0
        {
            return Err(DisplayConfigError::InvalidStride);
        }
        let layout = Self {
            width,
            height,
            stride,
            format,
        };
        layout.len()?;
        Ok(layout)
    }

    pub fn len(self) -> Result<usize, DisplayConfigError> {
        self.stride
            .checked_mul(self.height)
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| *n <= isize::MAX as usize)
            .ok_or(DisplayConfigError::BufferTooSmall)
    }

    pub fn validate(self, buffer: &mut PixelBuffer) -> Result<(), DisplayConfigError> {
        if buffer.len() < self.len()? {
            return Err(DisplayConfigError::BufferTooSmall);
        }
        let ptr = buffer.as_mut_ptr();
        // SAFETY: this only computes the aligned address, without accessing data.
        let aligned = unsafe { sys::lv_draw_buf_align(ptr.cast(), self.format.to_raw()) };
        if aligned.cast::<u8>() != ptr {
            return Err(DisplayConfigError::UnalignedBuffer);
        }
        Ok(())
    }
}
