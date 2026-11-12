use crate::display::Display;
use crate::input_device::InputDriver;
use crate::{Event, LvError, LvResult, Obj, Widget};
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU32, Ordering};
use core::time::Duration;
use core::{ptr, result};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CoreError {
    ResourceNotAvailable,
    OperationFailed,
}

type Result<T> = result::Result<T, CoreError>;

pub(crate) fn disp_get_default() -> Result<Display> {
    let disp_ptr = unsafe { rs_lvgl_sys::lv_display_get_default() };
    Ok(Display::from_raw(
        NonNull::new(disp_ptr).ok_or(CoreError::OperationFailed)?,
    ))
}

pub(crate) fn get_str_act(disp: Option<&Display>) -> Result<Obj<'_>> {
    let scr_ptr = unsafe {
        rs_lvgl_sys::lv_display_get_screen_active(
            disp.map(|d| d.disp.as_ptr())
                .unwrap_or(ptr::null_mut() as *mut rs_lvgl_sys::lv_display_t),
        )
    };
    match unsafe { Obj::from_raw(NonNull::new(scr_ptr).ok_or(CoreError::ResourceNotAvailable)?) } {
        Some(o) => Ok(o),
        None => Err(CoreError::OperationFailed),
    }
}

/// Sub-millisecond time not yet handed to LVGL, in nanoseconds (below 1 ms).
static TICK_REMAINDER_NS: AtomicU32 = AtomicU32::new(0);

/// Advance LVGL's tick by `tick_period`.
///
/// LVGL counts whole milliseconds. The sub-millisecond part of each call is
/// carried into the next one instead of being dropped, so a loop that reports
/// its measured elapsed time does not make the LVGL clock run slow. The native
/// tick is a wrapping `u32`; a period beyond `u32::MAX` milliseconds advances
/// it modulo 2^32, as the native counter itself does.
///
/// Call this from a single tick source. The carried remainder is a plain load
/// and store, not a compare-and-swap, so concurrent callers can lose a fraction
/// of a millisecond.
#[inline]
pub fn tick_inc(tick_period: Duration) {
    const NS_PER_MS: u32 = 1_000_000;
    // Both terms are below one second and one millisecond, so the sum fits u32.
    let sub_second = tick_period.subsec_nanos() + TICK_REMAINDER_NS.load(Ordering::Relaxed);
    TICK_REMAINDER_NS.store(sub_second % NS_PER_MS, Ordering::Relaxed);
    // Truncating and wrapping here is the native tick's own modulo 2^32, kept in
    // 32/64-bit arithmetic so a tick interrupt needs no 128-bit division.
    let whole_seconds_ms = tick_period.as_secs().wrapping_mul(1000) as u32;
    let wrapped_ms = whole_seconds_ms.wrapping_add(sub_second / NS_PER_MS);
    unsafe {
        rs_lvgl_sys::lv_tick_inc(wrapped_ms);
    }
}

/// Run LVGL's ready timers, including refresh, animation and input reading.
#[inline]
pub fn task_handler() {
    timer_handler();
}

/// Run LVGL's ready timers and report when the handler is next needed.
///
/// Returns the milliseconds until the next unpaused timer is due, or
/// [`NO_TIMER_READY`] when every timer is paused. A platform loop may sleep for
/// that long, provided it also wakes for input and for anything else that
/// creates, resumes or readies a timer. [`task_handler`] is the same call
/// without the result.
#[inline]
pub fn timer_handler() -> u32 {
    unsafe { rs_lvgl_sys::lv_timer_handler() }
}

/// Value returned by [`timer_handler`] when no unpaused timer exists.
pub const NO_TIMER_READY: u32 = rs_lvgl_sys::LV_NO_TIMER_READY;

#[inline]
pub fn event_send<W: for<'a> Widget<'a>>(
    obj: &mut W,
    event: Event<<W as Widget<'_>>::SpecialEvent>,
) {
    unsafe {
        rs_lvgl_sys::lv_obj_send_event(obj.raw().as_mut(), event.into(), ptr::null_mut());
    };
}

#[allow(dead_code)]
pub(crate) fn indev_drv_register<D>(input_device: &mut impl InputDriver<D>) -> LvResult<()> {
    unsafe {
        let indev = rs_lvgl_sys::lv_indev_create();
        if indev.is_null() {
            return Err(LvError::LvOOMemory);
        }
        input_device.set_descriptor(indev)?;
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn elapsed_since(start: u32) -> u32 {
        unsafe { rs_lvgl_sys::lv_tick_elaps(start) }
    }

    #[test]
    fn tick_inc_carries_sub_millisecond_time() {
        crate::tests::initialize_test(false);
        let start = unsafe { rs_lvgl_sys::lv_tick_get() };
        // Any remainder left by an earlier call is below one millisecond, so
        // eight half-millisecond steps advance the tick by exactly four.
        for _ in 0..8 {
            tick_inc(Duration::from_micros(500));
        }
        assert_eq!(elapsed_since(start), 4);
    }

    #[test]
    fn tick_inc_preserves_whole_milliseconds_and_native_wrap() {
        crate::tests::initialize_test(false);
        let start = unsafe { rs_lvgl_sys::lv_tick_get() };
        tick_inc(Duration::from_millis(30));
        assert_eq!(elapsed_since(start), 30);
        tick_inc(Duration::from_millis((1 << 32) + 5));
        assert_eq!(elapsed_since(start), 35);
    }

    #[test]
    fn timer_handler_reports_time_until_the_next_timer() {
        use core::sync::atomic::AtomicU32;
        static RUNS: AtomicU32 = AtomicU32::new(0);
        unsafe extern "C" fn count(_: *mut rs_lvgl_sys::lv_timer_t) {
            RUNS.fetch_add(1, Ordering::Relaxed);
        }

        crate::tests::initialize_test(false);
        RUNS.store(0, Ordering::Relaxed);
        let timer = unsafe { rs_lvgl_sys::lv_timer_create(Some(count), 50, ptr::null_mut()) };
        assert!(!timer.is_null());

        let next = timer_handler();
        assert!((1..=50).contains(&next), "next timer in {next} ms");
        assert_eq!(RUNS.load(Ordering::Relaxed), 0);

        tick_inc(Duration::from_millis(50));
        task_handler();
        assert_eq!(RUNS.load(Ordering::Relaxed), 1);

        // With the only running timer paused, nothing is due.
        unsafe { rs_lvgl_sys::lv_timer_pause(timer) };
        let idle = timer_handler();
        unsafe { rs_lvgl_sys::lv_timer_delete(timer) };
        assert!(idle > 50, "paused timer still reported as due in {idle} ms");
    }
}
