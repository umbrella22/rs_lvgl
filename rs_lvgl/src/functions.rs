use crate::display::Display;
use crate::input_device::InputDriver;
use crate::{Event, LvError, LvResult, Obj, Widget};
use core::ptr::NonNull;
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

#[inline]
pub fn tick_inc(tick_period: Duration) {
    unsafe {
        rs_lvgl_sys::lv_tick_inc(tick_period.as_millis() as u32);
    }
}

#[inline]
pub fn task_handler() {
    unsafe { rs_lvgl_sys::lv_timer_handler() };
}

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
