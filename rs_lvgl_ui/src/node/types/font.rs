//! Static native font handles and feature-selected fonts.

use core::fmt;

/// A build-time LVGL font.
///
/// Fonts are static LVGL objects. `Font::montserrat_14` is always available;
/// `Font::montserrat_<size>` exists for every size enabled with a
/// `font-montserrat-<size>` feature. Custom fonts enter through
/// [`Font::from_raw`].
#[derive(Clone, Copy)]
pub struct Font(pub(crate) *const rs_lvgl_sys::lv_font_t);

impl Font {
    /// Wrap an LVGL font descriptor that remains valid for the program's
    /// lifetime.
    ///
    /// # Safety
    ///
    /// `raw` must point to a fully initialized, immutable `lv_font_t` that
    /// outlives every object using it. LVGL retains the pointer in its styles.
    pub const unsafe fn from_raw(raw: *const rs_lvgl_sys::lv_font_t) -> Self {
        Self(raw)
    }

    /// Vendored Source Han Sans SC CJK subset, 14 pixels.
    #[cfg(feature = "font-cjk-14")]
    pub const fn cjk_14() -> Self {
        Self(core::ptr::addr_of!(
            rs_lvgl_sys::lv_font_source_han_sans_sc_14_cjk
        ))
    }
    /// Vendored Source Han Sans SC CJK subset, 16 pixels.
    #[cfg(feature = "font-cjk-16")]
    pub const fn cjk_16() -> Self {
        Self(core::ptr::addr_of!(
            rs_lvgl_sys::lv_font_source_han_sans_sc_16_cjk
        ))
    }

    /// Montserrat 14, LVGL's default font. Always available.
    pub const fn montserrat_14() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_14))
    }

    #[cfg(feature = "font-montserrat-8")]
    pub const fn montserrat_8() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_8))
    }

    #[cfg(feature = "font-montserrat-10")]
    pub const fn montserrat_10() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_10))
    }

    #[cfg(feature = "font-montserrat-12")]
    pub const fn montserrat_12() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_12))
    }

    #[cfg(feature = "font-montserrat-16")]
    pub const fn montserrat_16() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_16))
    }

    #[cfg(feature = "font-montserrat-18")]
    pub const fn montserrat_18() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_18))
    }

    #[cfg(feature = "font-montserrat-20")]
    pub const fn montserrat_20() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_20))
    }

    #[cfg(feature = "font-montserrat-22")]
    pub const fn montserrat_22() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_22))
    }

    #[cfg(feature = "font-montserrat-24")]
    pub const fn montserrat_24() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_24))
    }

    #[cfg(feature = "font-montserrat-26")]
    pub const fn montserrat_26() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_26))
    }

    #[cfg(feature = "font-montserrat-28")]
    pub const fn montserrat_28() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_28))
    }

    #[cfg(feature = "font-montserrat-30")]
    pub const fn montserrat_30() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_30))
    }

    #[cfg(feature = "font-montserrat-32")]
    pub const fn montserrat_32() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_32))
    }

    #[cfg(feature = "font-montserrat-34")]
    pub const fn montserrat_34() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_34))
    }

    #[cfg(feature = "font-montserrat-36")]
    pub const fn montserrat_36() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_36))
    }

    #[cfg(feature = "font-montserrat-38")]
    pub const fn montserrat_38() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_38))
    }

    #[cfg(feature = "font-montserrat-40")]
    pub const fn montserrat_40() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_40))
    }

    #[cfg(feature = "font-montserrat-42")]
    pub const fn montserrat_42() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_42))
    }

    #[cfg(feature = "font-montserrat-44")]
    pub const fn montserrat_44() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_44))
    }

    #[cfg(feature = "font-montserrat-46")]
    pub const fn montserrat_46() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_46))
    }

    #[cfg(feature = "font-montserrat-48")]
    pub const fn montserrat_48() -> Self {
        Self(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_48))
    }
}

impl Default for Font {
    fn default() -> Self {
        Self::montserrat_14()
    }
}

impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Font").field(&self.0).finish()
    }
}

impl PartialEq for Font {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Font {}
