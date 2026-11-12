//! Custom tick source for LVGL 9.
//!
//! LVGL 8 drove the tick through the `LV_TICK_CUSTOM` compile-time hook
//! (`rs_lv_timer`); LVGL 9 replaces it with the runtime callback
//! [`rs_lvgl_sys::lv_tick_set_cb`]. Calling [`update_clock`] periodically
//! registers that callback and feeds it, so animations
//! and timers advance from your Rust clock.

use core::num::TryFromIntError;
use core::sync::atomic::{AtomicU32, Ordering};
use core::time::Duration;

static TICK_MS: AtomicU32 = AtomicU32::new(0);

extern "C" fn tick_cb() -> u32 {
    TICK_MS.load(Ordering::Relaxed)
}

pub trait LvClock {
    /// Monotonic time elapsed since the clock's origin.
    fn since_init(&self) -> Duration;
}

/// Feed LVGL's tick from a monotonic Rust clock and register the tick callback.
///
/// LVGL's tick is a wrapping `u32` millisecond counter, and its elapsed-time
/// arithmetic is defined across a wrap. The clock's elapsed time is therefore
/// reduced modulo 2^32 ms (about 49.7 days) instead of being rejected, so
/// timers, animations and input timing keep advancing past that uptime. Any
/// sub-millisecond remainder is discarded.
///
/// This function no longer fails. The `Result` is retained for source
/// compatibility and is always `Ok(())`.
pub fn update_clock(clock: &impl LvClock) -> Result<(), TryFromIntError> {
    // The mask is the intended wrap, not a lossy conversion of a small value.
    let wrapped_ms = (clock.since_init().as_millis() & u128::from(u32::MAX)) as u32;
    TICK_MS.store(wrapped_ms, Ordering::Relaxed);
    // lv_init resets the C callback. Register on each update rather than
    // caching a registration flag that outlives an LVGL initialization cycle.
    unsafe { rs_lvgl_sys::lv_tick_set_cb(Some(tick_cb)) };
    Ok(())
}

#[cfg(all(test, not(feature = "custom_allocator")))]
mod tests {
    use super::*;

    struct Clock(u64);
    impl LvClock for Clock {
        fn since_init(&self) -> Duration {
            Duration::from_millis(self.0)
        }
    }

    #[test]
    fn clock_is_registered_again_after_reinitialization() {
        crate::tests::initialize_test(false);
        update_clock(&Clock(123)).unwrap();
        assert_eq!(unsafe { rs_lvgl_sys::lv_tick_get() }, 123);

        // No display, widget, or other LVGL handle is alive here.
        unsafe { crate::deinit() };
        crate::init();
        update_clock(&Clock(456)).unwrap();
        assert_eq!(unsafe { rs_lvgl_sys::lv_tick_get() }, 456);
        update_clock(&Clock(789)).unwrap();
        assert_eq!(unsafe { rs_lvgl_sys::lv_tick_get() }, 789);
    }

    #[test]
    fn clock_wraps_with_the_native_tick_past_u32_milliseconds() {
        const WRAP: u64 = 1 << 32;
        crate::tests::initialize_test(false);
        update_clock(&Clock(WRAP - 10)).unwrap();
        let before = unsafe { rs_lvgl_sys::lv_tick_get() };
        assert_eq!(before, u32::MAX - 9);

        // About 49.7 days of uptime. The native tick is a wrapping u32, so the
        // clock must keep advancing and elapsed time must stay continuous.
        update_clock(&Clock(WRAP + 20)).expect("uptime past u32 milliseconds keeps ticking");
        assert_eq!(unsafe { rs_lvgl_sys::lv_tick_get() }, 20);
        assert_eq!(unsafe { rs_lvgl_sys::lv_tick_elaps(before) }, 30);

        update_clock(&Clock(3 * WRAP + 7)).unwrap();
        assert_eq!(unsafe { rs_lvgl_sys::lv_tick_get() }, 7);
    }
}
