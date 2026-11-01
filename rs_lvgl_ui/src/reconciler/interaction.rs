//! Controlled object flags and lazy handler attachment.

use crate::events::{EventQueue, HandlerSlot};
use crate::node::HandlerSet;
use crate::widgets::{self, KindTag};
use crate::UiError;
use alloc::rc::Rc;
use core::ptr::NonNull;
use rs_lvgl_sys::lv_obj_t;

/// Apply explicit clickable/scrollable flags that changed.
pub(super) fn apply_flags(
    obj: NonNull<lv_obj_t>,
    clickable: Option<bool>,
    scrollable: Option<bool>,
    old_clickable: Option<bool>,
    old_scrollable: Option<bool>,
) {
    // LVGL 9.6 deprecates the generic flag functions in favour of dedicated
    // setters.
    let setters: [(
        Option<bool>,
        Option<bool>,
        unsafe extern "C" fn(*mut lv_obj_t, bool),
    ); 2] = [
        (clickable, old_clickable, rs_lvgl_sys::lv_obj_set_clickable),
        (
            scrollable,
            old_scrollable,
            rs_lvgl_sys::lv_obj_set_scrollable,
        ),
    ];
    for (value, old, set) in setters {
        let Some(value) = value else {
            continue;
        };
        if old == Some(value) {
            continue;
        }
        // SAFETY: `obj` is a live object owned by the mounted tree.
        unsafe { set(obj.as_ptr(), value) };
    }
}

/// Write a controlled checked state that differs from the native object.
pub(super) fn apply_checked(obj: NonNull<lv_obj_t>, tag: KindTag, checked: Option<bool>) {
    let Some(checked) = checked else {
        return;
    };
    if tag.owns_checked() {
        return;
    }
    let state = rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED;
    // SAFETY: `obj` is a live object owned by the mounted tree. Changing a
    // state sends no value event.
    unsafe {
        if rs_lvgl_sys::lv_obj_has_state(obj.as_ptr(), state) != checked {
            if checked {
                rs_lvgl_sys::lv_obj_add_state(obj.as_ptr(), state);
            } else {
                rs_lvgl_sys::lv_obj_remove_state(obj.as_ptr(), state);
            }
        }
    }
}

/// Install `handlers` on a mounted object, allocating the slot lazily.
pub(super) fn sync_handlers<M: Clone + 'static>(
    slot: &mut Option<NonNull<HandlerSlot<M>>>,
    obj: NonNull<lv_obj_t>,
    tag: KindTag,
    handlers: HandlerSet<M>,
    queue: &Rc<EventQueue<M>>,
) -> Result<(), UiError> {
    match slot {
        Some(existing) => {
            // SAFETY: the slot is owned by the live native object and is
            // released only when that object is deleted.
            unsafe { existing.as_ref() }.set_handlers(obj, handlers)
        }
        None if handlers.is_empty() => Ok(()),
        None => {
            // SAFETY: `obj` is live and has no slot yet. On error the slot is
            // owned by the object's delete callback or already released.
            let attached = unsafe {
                HandlerSlot::attach(
                    obj,
                    queue.clone(),
                    widgets::value_source(tag),
                    widgets::event_inner_target(tag, obj),
                    handlers,
                )
            }?;
            *slot = Some(attached);
            Ok(())
        }
    }
}
