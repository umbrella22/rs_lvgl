//! Semantic transform units. Native representations are checked before mutation.

use crate::UiError;

/// A scale factor, quantized to LVGL's 1/256 increments.
///
/// Percentages round to the nearest increment, with ties rounded upward. Values
/// above the native maximum are rejected during tree validation. Zero is valid
/// for style transforms; image constructors require a strictly positive scale.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Scale(u32);

impl Scale {
    /// Construct a percentage: 100 is original size, 50 is half size.
    pub const fn percent(percent: u16) -> Self {
        Self((percent as u32 * 256 + 50) / 100)
    }

    /// Construct a ratio with the same nearest-increment rounding as percent.
    /// A zero denominator or a rounded result above 65535 is an error.
    /// `ratio(old_native_value, 256)` preserves an old native scale exactly.
    pub const fn ratio(numerator: u32, denominator: u32) -> Result<Self, UiError> {
        if denominator == 0 {
            return Err(UiError::InvalidProperty);
        }
        let value = (numerator as u64 * 256 + denominator as u64 / 2) / denominator as u64;
        if value > u16::MAX as u64 {
            return Err(UiError::InvalidProperty);
        }
        Ok(Self(value as u32))
    }

    pub(crate) const fn to_raw(self) -> i32 {
        // percent(u16) and checked ratio cannot exceed i32::MAX.
        self.0 as i32
    }

    pub(crate) fn validate(self) -> Result<(), UiError> {
        if self.0 > u16::MAX as u32 {
            Err(UiError::InvalidProperty)
        } else {
            Ok(())
        }
    }

    #[cfg(feature = "widget-image")]
    pub(crate) fn validate_image(self) -> Result<(), UiError> {
        self.validate()?;
        if self.0 == 0 {
            Err(UiError::InvalidProperty)
        } else {
            Ok(())
        }
    }
}

/// A clockwise rotation, quantized to tenths of a degree.
///
/// Negative angles rotate counterclockwise. Values outside -3276.8..=3276.7
/// degrees fail tree validation rather than wrapping into the native i16 range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Angle(i32);

impl Angle {
    /// Construct a whole-degree angle. Multiplication takes place in i32.
    pub const fn degrees(degrees: i16) -> Self {
        Self(degrees as i32 * 10)
    }

    /// Construct a precise angle, e.g. 455 means 45.5 degrees.
    pub const fn tenths(tenths: i32) -> Self {
        Self(tenths)
    }

    pub(crate) const fn to_raw(self) -> i32 {
        self.0
    }

    pub(crate) fn validate(self) -> Result<(), UiError> {
        if i16::try_from(self.0).is_err() {
            Err(UiError::InvalidProperty)
        } else {
            Ok(())
        }
    }
}
