//! Stable native handler slots and lazy event registration.

mod dispatch;
mod native;

use super::{EventQueue, ValueSource};
use crate::node::HandlerSet;
use crate::UiError;
use alloc::{boxed::Box, rc::Rc};
use core::{
    cell::{Cell, RefCell},
    ptr::NonNull,
};
use native::{add_callback, delete_trampoline, event_trampoline};
use rs_lvgl_sys::lv_obj_t;

const NEEDS_CLICK: u8 = 1 << 0;
const NEEDS_VALUE: u8 = 1 << 1;
const NEEDS_ALL: u8 = 1 << 2;

fn needed_codes<M>(handlers: &HandlerSet<M>) -> u8 {
    let mut needed = 0;
    if handlers.click.is_some() {
        needed |= NEEDS_CLICK;
    }
    if handlers.value_changed.is_some() || handlers.text_changed.is_some() {
        needed |= NEEDS_VALUE;
    }
    if handlers.event.is_some() || handlers.has_input() {
        needed |= NEEDS_ALL;
    }
    needed
}

/// One native callback slot per mounted node that has handlers.
///
/// The slot has a stable address for the native object's lifetime. Updating a
/// node replaces the handlers in place. Native registrations are added lazily
/// for the event codes the handlers need: `on_click` listens to
/// `LV_EVENT_CLICKED`, value/text handlers to `LV_EVENT_VALUE_CHANGED`, and
/// `on_event` and input payload handlers listen to `LV_EVENT_ALL`. Registrations are never
/// narrowed again, which keeps an update from touching the native callback
/// list in the common case.
pub(crate) struct HandlerSlot<M: Clone + 'static> {
    handlers: RefCell<HandlerSet<M>>,
    suppress_next: Cell<bool>,
    queue: Rc<EventQueue<M>>,
    value_source: ValueSource,
    /// An inner object whose bubbled events count as this node's own (for
    /// example a calendar's button matrix); null when there is none.
    inner_target: *mut lv_obj_t,
    registered: Cell<u8>,
}

impl<M: Clone + 'static> HandlerSlot<M> {
    /// Allocate a slot for `obj` and register the callbacks `handlers` need.
    ///
    /// # Safety
    ///
    /// `obj` must be a live LVGL object without a slot. `inner_target` must be
    /// null or a descendant of `obj` that lives as long as `obj`. On success the slot is
    /// owned by `obj`'s delete callback. On error the slot is either already
    /// released or owned by that callback; the caller must not use it and
    /// should delete `obj` (or let the next delete of `obj` release it).
    pub(crate) unsafe fn attach(
        obj: NonNull<lv_obj_t>,
        queue: Rc<EventQueue<M>>,
        value_source: ValueSource,
        inner_target: *mut lv_obj_t,
        handlers: HandlerSet<M>,
    ) -> Result<NonNull<Self>, UiError> {
        let slot = Box::into_raw(Box::new(Self {
            handlers: RefCell::new(handlers),
            suppress_next: Cell::new(false),
            queue,
            value_source,
            inner_target,
            registered: Cell::new(0),
        }));
        // SAFETY: `obj` is live and `slot` is a fresh, uniquely owned box. The
        // delete callback is registered first so that it owns the slot before
        // any event callback can observe it.
        unsafe {
            let delete = rs_lvgl_sys::lv_obj_add_event_cb(
                obj.as_ptr(),
                Some(delete_trampoline::<M>),
                rs_lvgl_sys::lv_event_code_t_LV_EVENT_DELETE,
                slot.cast(),
            );
            if delete.is_null() {
                drop(Box::from_raw(slot));
                return Err(UiError::Allocation);
            }
            let slot = NonNull::new_unchecked(slot);
            slot.as_ref().register_needed(obj)?;
            Ok(slot)
        }
    }

    /// Replace the handlers and register any newly needed event codes.
    ///
    /// `obj` must be the live object this slot is attached to.
    pub(crate) fn set_handlers(
        &self,
        obj: NonNull<lv_obj_t>,
        handlers: HandlerSet<M>,
    ) -> Result<(), UiError> {
        *self.handlers.borrow_mut() = handlers;
        self.register_needed(obj)
    }

    fn register_needed(&self, obj: NonNull<lv_obj_t>) -> Result<(), UiError> {
        let needed = needed_codes(&self.handlers.borrow());
        let registered = self.registered.get();
        if registered & NEEDS_ALL != 0 || needed & !registered == 0 {
            return Ok(());
        }
        let data = self as *const Self as *mut cty::c_void;
        // SAFETY: `obj` is the live object owning this slot; the slot outlives
        // every registration because the delete callback releases it last.
        unsafe {
            if needed & NEEDS_ALL != 0 {
                // One `LV_EVENT_ALL` registration replaces the specific ones
                // so that no event is dispatched twice.
                rs_lvgl_sys::lv_obj_remove_event_cb_with_user_data(
                    obj.as_ptr(),
                    Some(event_trampoline::<M>),
                    data,
                );
                self.registered.set(0);
                add_callback::<M>(obj, rs_lvgl_sys::lv_event_code_t_LV_EVENT_ALL, data)?;
                self.registered.set(NEEDS_ALL);
                return Ok(());
            }
            if needed & NEEDS_CLICK != 0 && registered & NEEDS_CLICK == 0 {
                add_callback::<M>(obj, rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED, data)?;
                self.registered.set(self.registered.get() | NEEDS_CLICK);
            }
            if needed & NEEDS_VALUE != 0 && registered & NEEDS_VALUE == 0 {
                add_callback::<M>(
                    obj,
                    rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
                    data,
                )?;
                self.registered.set(self.registered.get() | NEEDS_VALUE);
            }
        }
        Ok(())
    }

    pub(crate) fn suppress(&self) {
        self.suppress_next.set(true);
    }

    pub(crate) fn clear_suppress(&self) {
        self.suppress_next.set(false);
    }

    fn take_suppression(&self, code: u32) -> bool {
        // Only a value-changed event consumes the flag. Other events emitted
        // by the same programmatic write (layout/style/child changes) must not
        // swallow the suppression meant for the value event.
        if code == rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED && self.suppress_next.get() {
            self.suppress_next.set(false);
            return true;
        }
        false
    }
}
