use crate::{sys, Display};
use core::{fmt, ptr::NonNull, time::Duration};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PeriodError {
    InvalidPeriod,
    TimerUnavailable,
}
impl fmt::Display for PeriodError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "timer period: {:?}", self)
    }
}

pub(crate) fn period_millis(period: Duration) -> Result<u32, PeriodError> {
    if period.is_zero() || period.subsec_nanos() % 1_000_000 != 0 {
        return Err(PeriodError::InvalidPeriod);
    }
    u32::try_from(period.as_millis()).map_err(|_| PeriodError::InvalidPeriod)
}

impl Display {
    /// Set software refresh scheduling; this neither guarantees FPS nor changes
    /// panel scan-out. Periods must be nonzero whole milliseconds within u32.
    pub fn set_refresh_period(&mut self, period: Duration) -> Result<(), PeriodError> {
        let millis = period_millis(period)?;
        // SAFETY: Display is live and all native calls use the serialized UI thread.
        let timer = NonNull::new(unsafe { sys::lv_display_get_refr_timer(self.disp.as_ptr()) })
            .ok_or(PeriodError::TimerUnavailable)?;
        unsafe { sys::lv_timer_set_period(timer.as_ptr(), millis) };
        Ok(())
    }

    pub fn refresh_period(&self) -> Result<Duration, PeriodError> {
        let timer = NonNull::new(unsafe { sys::lv_display_get_refr_timer(self.disp.as_ptr()) })
            .ok_or(PeriodError::TimerUnavailable)?;
        Ok(Duration::from_millis(
            unsafe { sys::lv_rs_timer_get_period(timer.as_ptr()) }.into(),
        ))
    }
}

// Call only for a live concrete device; arbitrary pointers retain an unsafe boundary.
pub(crate) unsafe fn set_input_period(
    device: *mut sys::lv_indev_t,
    period: Duration,
) -> Result<(), PeriodError> {
    let millis = period_millis(period)?;
    if device.is_null() {
        return Err(PeriodError::TimerUnavailable);
    }
    let timer = NonNull::new(unsafe { sys::lv_indev_get_read_timer(device) })
        .ok_or(PeriodError::TimerUnavailable)?;
    unsafe { sys::lv_timer_set_period(timer.as_ptr(), millis) };
    Ok(())
}

pub(crate) unsafe fn input_period(device: *mut sys::lv_indev_t) -> Result<Duration, PeriodError> {
    if device.is_null() {
        return Err(PeriodError::TimerUnavailable);
    }
    let timer = NonNull::new(unsafe { sys::lv_indev_get_read_timer(device) })
        .ok_or(PeriodError::TimerUnavailable)?;
    Ok(Duration::from_millis(
        unsafe { sys::lv_rs_timer_get_period(timer.as_ptr()) }.into(),
    ))
}
