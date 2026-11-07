//! Semantic endpoint values for the optional controlled range slider.

/// Ordered endpoints of a controlled slider. Construction never sorts or clamps.
/// The descriptor validates `min <= lower <= upper <= max` before native writes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SliderRange {
    pub lower: i32,
    pub upper: i32,
}
impl SliderRange {
    pub const fn new(lower: i32, upper: i32) -> Self {
        Self { lower, upper }
    }

    pub(crate) fn validate_native(self, min: i32, max: i32) -> Result<(), crate::UiError> {
        // Native bar drawing subtracts the domain in int32_t; native keypad
        // handling computes an endpoint +/- 1 before clamping. Reject values
        // those operations cannot represent rather than silently narrowing them.
        if min > self.lower
            || self.lower > self.upper
            || self.upper > max
            || max.checked_sub(min).is_none()
            || min.checked_sub(1).is_none()
            || max.checked_add(1).is_none()
        {
            return Err(crate::UiError::InvalidProperty);
        }
        Ok(())
    }
}
