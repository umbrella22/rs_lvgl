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
    fn since_init(&self) -> Duration;
}

pub fn update_clock(clock: &impl LvClock) -> Result<(), TryFromIntError> {
    TICK_MS.store(
        clock.since_init().as_millis().try_into()?,
        Ordering::Relaxed,
    );
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
}
