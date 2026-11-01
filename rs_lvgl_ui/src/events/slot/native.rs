//! C callback entry points and exactly-once handler-slot cleanup.

use super::HandlerSlot;
use crate::events::types::input_event;
use crate::UiError;
use alloc::boxed::Box;
use core::ptr::NonNull;
use rs_lvgl_sys::lv_obj_t;

/// # Safety
///
/// `obj` must be live and `data` must point to the slot attached to it.
pub(super) unsafe fn add_callback<M: Clone + 'static>(
    obj: NonNull<lv_obj_t>,
    code: rs_lvgl_sys::lv_event_code_t,
    data: *mut cty::c_void,
) -> Result<(), UiError> {
    // SAFETY: guaranteed by the caller.
    let dsc = unsafe {
        rs_lvgl_sys::lv_obj_add_event_cb(obj.as_ptr(), Some(event_trampoline::<M>), code, data)
    };
    if dsc.is_null() {
        Err(UiError::Allocation)
    } else {
        Ok(())
    }
}

/// # Safety
///
/// `user_data` must point to a live `HandlerSlot<M>` whose native object is
/// still alive. The callback never unwinds; it only enqueues a message.
pub(super) unsafe extern "C" fn event_trampoline<M: Clone + 'static>(
    event: *mut rs_lvgl_sys::lv_event_t,
) {
    // SAFETY: LVGL invokes this callback with the user data registered on a
    // live object. The pointer was produced by `Box::into_raw` and is released
    // exactly once by `delete_trampoline`.
    unsafe {
        let code = rs_lvgl_sys::lv_event_get_code(event);
        if code == rs_lvgl_sys::lv_event_code_t_LV_EVENT_DELETE {
            // The delete callback may already have released the slot.
            return;
        }
        let slot = rs_lvgl_sys::lv_event_get_user_data(event) as *const HandlerSlot<M>;
        if slot.is_null() {
            return;
        }
        let slot = &*slot;
        let target = rs_lvgl_sys::lv_event_get_target_obj(event);
        let current = rs_lvgl_sys::lv_event_get_current_target_obj(event);
        let bubbled =
            target != current && (slot.inner_target.is_null() || target != slot.inner_target);
        if !bubbled && slot.take_suppression(code) {
            return;
        }
        slot.dispatch(
            code,
            current,
            rs_lvgl_sys::lv_event_get_param(event),
            input_event(event),
            bubbled,
        );
    }
}

/// # Safety
///
/// `user_data` must be a pointer previously produced by `Box::into_raw` for a
/// `HandlerSlot<M>` and not yet dropped.
pub(super) unsafe extern "C" fn delete_trampoline<M: Clone + 'static>(
    event: *mut rs_lvgl_sys::lv_event_t,
) {
    // SAFETY: the delete event is delivered once; reconstructing the box turns
    // the raw pointer back into its unique owner.
    unsafe {
        if rs_lvgl_sys::lv_event_get_target_obj(event)
            != rs_lvgl_sys::lv_event_get_current_target_obj(event)
        {
            return;
        }
        let slot = rs_lvgl_sys::lv_event_get_user_data(event) as *mut HandlerSlot<M>;
        if !slot.is_null() {
            drop(Box::from_raw(slot));
        }
    }
}
