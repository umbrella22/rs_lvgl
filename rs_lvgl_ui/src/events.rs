use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::rc::Rc;
use core::cell::{Cell, RefCell};

use rs_lvgl_sys::lv_obj_t;

use core::ptr::NonNull;

use crate::node::HandlerSet;
use crate::UiError;

/// A supported LVGL event delivered to application handlers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    Pressed,
    Pressing,
    PressLost,
    ShortClicked,
    Clicked,
    LongPressed,
    LongPressedRepeat,
    Released,
    ValueChanged,
    Checked,
    Unchecked,
    Focused,
    Defocused,
    HoverOver,
    HoverLeave,
    Key,
    Rotary,
    Ready,
    Cancel,
    ScrollBegin,
    ScrollEnd,
    Gesture,
    GestureUp,
    GestureDown,
    GestureLeft,
    GestureRight,
}

/// Direction of one LVGL directional gesture event. The generic Gesture
/// event is not delivered a second time through `on_gesture`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GestureDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Actual native scroll offset in pixels; positive values move into content.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ScrollOffset {
    pub x: i32,
    pub y: i32,
}

/// Pointer coordinates in display pixels, copied during the native event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PointerEvent {
    pub event: Event,
    pub x: i32,
    pub y: i32,
}

/// Copied data from a native input event. Descendant observers receive the
/// original target's data. No native pointer or borrowed event escapes C.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputEvent {
    Key(u32),
    Rotary(i32),
    Gesture(GestureDirection),
    Scroll(ScrollOffset),
    Pointer(PointerEvent),
    Other(Event),
}

/// # Safety
/// `event` and its original target/parameter are live and follow LVGL's
/// event-code-specific parameter contract for this synchronous callback.
unsafe fn input_event(event: *mut rs_lvgl_sys::lv_event_t) -> Option<InputEvent> {
    // SAFETY: accessors below are selected by code, so a key pointer is never
    // interpreted as an input device and a rotary delta remains signed.
    unsafe {
        let code = rs_lvgl_sys::lv_event_get_code(event);
        let param = rs_lvgl_sys::lv_event_get_param(event);
        match code {
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_KEY if !param.is_null() => {
                return Some(InputEvent::Key(rs_lvgl_sys::lv_event_get_key(event)))
            }
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_ROTARY if !param.is_null() => {
                return Some(InputEvent::Rotary(rs_lvgl_sys::lv_event_get_rotary_diff(
                    event,
                )))
            }
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_GESTURE_UP => {
                return Some(InputEvent::Gesture(GestureDirection::Up))
            }
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_GESTURE_DOWN => {
                return Some(InputEvent::Gesture(GestureDirection::Down))
            }
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_GESTURE_LEFT => {
                return Some(InputEvent::Gesture(GestureDirection::Left))
            }
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_GESTURE_RIGHT => {
                return Some(InputEvent::Gesture(GestureDirection::Right))
            }
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_SCROLL => {
                let obj = rs_lvgl_sys::lv_event_get_target_obj(event);
                return Some(InputEvent::Scroll(ScrollOffset {
                    x: rs_lvgl_sys::lv_obj_get_scroll_x(obj),
                    y: rs_lvgl_sys::lv_obj_get_scroll_y(obj),
                }));
            }
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED
            | rs_lvgl_sys::lv_event_code_t_LV_EVENT_PRESSING
            | rs_lvgl_sys::lv_event_code_t_LV_EVENT_PRESS_LOST
            | rs_lvgl_sys::lv_event_code_t_LV_EVENT_SHORT_CLICKED
            | rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED
            | rs_lvgl_sys::lv_event_code_t_LV_EVENT_LONG_PRESSED
            | rs_lvgl_sys::lv_event_code_t_LV_EVENT_LONG_PRESSED_REPEAT
            | rs_lvgl_sys::lv_event_code_t_LV_EVENT_RELEASED => {
                let indev = rs_lvgl_sys::lv_event_get_indev(event);
                if !indev.is_null()
                    && rs_lvgl_sys::lv_indev_get_type(indev)
                        == rs_lvgl_sys::lv_indev_type_t_LV_INDEV_TYPE_POINTER
                {
                    let mut point = rs_lvgl_sys::lv_point_t::default();
                    rs_lvgl_sys::lv_indev_get_point(indev, &mut point);
                    return Some(InputEvent::Pointer(PointerEvent {
                        event: map_event(code)?,
                        x: point.x,
                        y: point.y,
                    }));
                }
            }
            _ => {}
        }
        map_event(code).map(InputEvent::Other)
    }
}

pub(crate) fn map_event(code: u32) -> Option<Event> {
    let event = match code {
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED => Event::Pressed,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_PRESSING => Event::Pressing,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_PRESS_LOST => Event::PressLost,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_SHORT_CLICKED => Event::ShortClicked,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED => Event::Clicked,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_LONG_PRESSED => Event::LongPressed,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_LONG_PRESSED_REPEAT => Event::LongPressedRepeat,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_RELEASED => Event::Released,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED => Event::ValueChanged,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_CHECKED => Event::Checked,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_UNCHECKED => Event::Unchecked,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_FOCUSED => Event::Focused,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_DEFOCUSED => Event::Defocused,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_HOVER_OVER => Event::HoverOver,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_HOVER_LEAVE => Event::HoverLeave,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_KEY => Event::Key,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_ROTARY => Event::Rotary,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_READY => Event::Ready,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_CANCEL => Event::Cancel,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_SCROLL_BEGIN => Event::ScrollBegin,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_SCROLL_END => Event::ScrollEnd,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_GESTURE => Event::Gesture,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_GESTURE_UP => Event::GestureUp,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_GESTURE_DOWN => Event::GestureDown,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_GESTURE_LEFT => Event::GestureLeft,
        rs_lvgl_sys::lv_event_code_t_LV_EVENT_GESTURE_RIGHT => Event::GestureRight,
        _ => return None,
    };
    Some(event)
}

/// Bounded message queue shared by the `Ui` and every mounted handler slot.
pub(crate) struct EventQueue<M> {
    items: RefCell<VecDeque<M>>,
    capacity: usize,
    dropped: Cell<u64>,
}

impl<M> EventQueue<M> {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            items: RefCell::new(VecDeque::with_capacity(capacity)),
            capacity,
            dropped: Cell::new(0),
        }
    }

    pub(crate) fn post(&self, msg: M) {
        let mut items = self.items.borrow_mut();
        if items.len() == self.capacity {
            items.pop_front();
            self.dropped.set(self.dropped.get() + 1);
        }
        items.push_back(msg);
    }

    pub(crate) fn pop(&self) -> Option<M> {
        self.items.borrow_mut().pop_front()
    }

    pub(crate) fn dropped(&self) -> u64 {
        self.dropped.get()
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ValueSource {
    None,
    #[cfg(feature = "widget-slider")]
    Slider,
    #[cfg(feature = "widget-bar")]
    Bar,
    #[cfg(feature = "widget-switch")]
    Switch,
    #[cfg(feature = "widget-checkbox")]
    Checkbox,
    #[cfg(feature = "widget-dropdown")]
    Dropdown,
    #[cfg(feature = "widget-arc")]
    Arc,
    #[cfg(feature = "widget-tabview")]
    Tabview,
    #[cfg(feature = "widget-roller")]
    Roller,
    #[cfg(feature = "widget-spinbox")]
    Spinbox,
    #[cfg(feature = "widget-buttonmatrix")]
    ButtonMatrix,
    #[cfg(feature = "widget-table")]
    Table,
    #[cfg(feature = "widget-tileview")]
    Tileview,
    #[cfg(feature = "widget-calendar")]
    Calendar,
    /// An index carried in the event parameter as `index + 1` (message box
    /// footer buttons); `-1` when the parameter is null.
    EventIndex,
}

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

    fn dispatch(
        &self,
        code: u32,
        target: *mut lv_obj_t,
        param: *mut cty::c_void,
        input: Option<InputEvent>,
        bubbled: bool,
    ) {
        // Collect before posting so handler execution cannot borrow the queue.
        let mut messages: [Option<M>; 6] = core::array::from_fn(|_| None);
        {
            let Ok(mut handlers) = self.handlers.try_borrow_mut() else {
                return;
            };
            if bubbled {
                if let (Some(map), Some(input)) = (&mut handlers.bubbled, &input) {
                    messages[0] = map(input);
                }
            } else {
                if code == rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED {
                    if let Some(msg) = &handlers.click {
                        messages[0] = Some(msg.clone());
                    }
                }
                if code == rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED {
                    if let Some(map) = &mut handlers.value_changed {
                        messages[1] = Some(map(read_value(target, self.value_source, param)));
                    }
                    #[cfg(feature = "widget-textarea")]
                    if let Some(map) = &mut handlers.text_changed {
                        // SAFETY: the slot's value source was validated as a
                        // textarea. Copy before its native text can change.
                        messages[2] = unsafe { with_text(target, |text| map(text)) };
                    }
                }
                if let (Some(map), Some(event)) = (&mut handlers.event, map_event(code)) {
                    messages[3] = map(&event);
                }
                messages[4] = match input {
                    Some(InputEvent::Key(value)) => handlers.key.as_mut().map(|f| f(value)),
                    Some(InputEvent::Rotary(value)) => handlers.rotary.as_mut().map(|f| f(value)),
                    Some(InputEvent::Gesture(value)) => handlers.gesture.as_mut().map(|f| f(value)),
                    Some(InputEvent::Scroll(value)) => handlers.scroll.as_mut().map(|f| f(value)),
                    Some(InputEvent::Pointer(value)) => handlers.pointer.as_mut().map(|f| f(value)),
                    _ => None,
                };
                if code == rs_lvgl_sys::lv_event_code_t_LV_EVENT_LONG_PRESSED {
                    messages[5] = handlers.long_press.clone();
                }
            }
        }
        for message in messages.into_iter().flatten() {
            self.queue.post(message);
        }
    }
}

/// # Safety
///
/// `obj` must be live and `data` must point to the slot attached to it.
unsafe fn add_callback<M: Clone + 'static>(
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

fn read_value(obj: *mut lv_obj_t, source: ValueSource, param: *mut cty::c_void) -> i32 {
    if obj.is_null() {
        return 0;
    }
    // SAFETY: the target object is the object this slot is registered on and is
    // valid for the duration of the event dispatch. The value getters accept
    // the concrete class created for `ValueSource`.
    unsafe {
        match source {
            ValueSource::None => 0,
            #[cfg(feature = "widget-slider")]
            ValueSource::Slider => rs_lvgl_sys::lv_slider_get_value(obj),
            #[cfg(feature = "widget-bar")]
            ValueSource::Bar => rs_lvgl_sys::lv_bar_get_value(obj),
            #[cfg(feature = "widget-switch")]
            ValueSource::Switch => {
                rs_lvgl_sys::lv_obj_has_state(obj, rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED) as i32
            }
            #[cfg(feature = "widget-checkbox")]
            ValueSource::Checkbox => {
                rs_lvgl_sys::lv_obj_has_state(obj, rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED) as i32
            }
            #[cfg(feature = "widget-dropdown")]
            ValueSource::Dropdown => rs_lvgl_sys::lv_dropdown_get_selected(obj) as i32,
            #[cfg(feature = "widget-arc")]
            ValueSource::Arc => rs_lvgl_sys::lv_arc_get_value(obj),
            #[cfg(feature = "widget-tabview")]
            ValueSource::Tabview => rs_lvgl_sys::lv_tabview_get_tab_active(obj) as i32,
            #[cfg(feature = "widget-roller")]
            ValueSource::Roller => rs_lvgl_sys::lv_roller_get_selected(obj) as i32,
            #[cfg(feature = "widget-spinbox")]
            ValueSource::Spinbox => rs_lvgl_sys::lv_spinbox_get_value(obj),
            #[cfg(feature = "widget-buttonmatrix")]
            ValueSource::ButtonMatrix => {
                let button = rs_lvgl_sys::lv_buttonmatrix_get_selected_button(obj);
                if button == rs_lvgl_sys::LV_BUTTONMATRIX_BUTTON_NONE {
                    -1
                } else {
                    button as i32
                }
            }
            #[cfg(feature = "widget-table")]
            ValueSource::Table => {
                let mut row = 0;
                let mut column = 0;
                rs_lvgl_sys::lv_table_get_selected_cell(obj, &mut row, &mut column);
                if row == rs_lvgl_sys::LV_TABLE_CELL_NONE
                    || column == rs_lvgl_sys::LV_TABLE_CELL_NONE
                {
                    -1
                } else {
                    ((row << 16) | column) as i32
                }
            }
            #[cfg(feature = "widget-tileview")]
            ValueSource::Tileview => {
                let tile = rs_lvgl_sys::lv_tileview_get_tile_active(obj);
                if tile.is_null() {
                    -1
                } else {
                    rs_lvgl_sys::lv_obj_get_index(tile)
                }
            }
            #[cfg(feature = "widget-calendar")]
            ValueSource::Calendar => {
                let mut date = rs_lvgl_sys::lv_calendar_date_t {
                    year: 0,
                    month: 0,
                    day: 0,
                };
                // Day-name cells and the adjacent months' days are disabled
                // buttons; LVGL would report them with the shown month.
                let matrix = rs_lvgl_sys::lv_calendar_get_btnmatrix(obj);
                let selected = rs_lvgl_sys::lv_buttonmatrix_get_selected_button(matrix);
                let enabled = selected != rs_lvgl_sys::LV_BUTTONMATRIX_BUTTON_NONE
                    && !rs_lvgl_sys::lv_buttonmatrix_has_button_ctrl(
                        matrix,
                        selected,
                        rs_lvgl_sys::lv_buttonmatrix_ctrl_t_LV_BUTTONMATRIX_CTRL_DISABLED,
                    );
                if enabled
                    && rs_lvgl_sys::lv_calendar_get_pressed_date(obj, &mut date)
                        == rs_lvgl_sys::lv_result_t_LV_RESULT_OK
                    && date.day != 0
                {
                    crate::node::CalendarDate::new(date.year, date.month, date.day).to_value()
                } else {
                    -1
                }
            }
            ValueSource::EventIndex => (param as usize as i32) - 1,
        }
    }
}

/// Borrow the textarea's current text for one synchronous callback.
///
/// # Safety
///
/// `obj` must be a live textarea object. The borrowed text is only passed to
/// `f` and is not returned, so it cannot outlive LVGL's buffer.
#[cfg(feature = "widget-textarea")]
unsafe fn with_text<R>(obj: *mut lv_obj_t, f: impl FnOnce(&str) -> R) -> Option<R> {
    if obj.is_null() {
        return None;
    }
    let ptr = unsafe { rs_lvgl_sys::lv_textarea_get_text(obj) };
    if ptr.is_null() {
        return None;
    }
    // SAFETY: the textarea returns a NUL-terminated string owned by LVGL and
    // valid until its text changes; `f` runs before this function returns.
    let text = unsafe { cstr_core::CStr::from_ptr(ptr) }.to_str().ok()?;
    Some(f(text))
}

/// # Safety
///
/// `user_data` must point to a live `HandlerSlot<M>` whose native object is
/// still alive. The callback never unwinds; it only enqueues a message.
unsafe extern "C" fn event_trampoline<M: Clone + 'static>(event: *mut rs_lvgl_sys::lv_event_t) {
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
unsafe extern "C" fn delete_trampoline<M: Clone + 'static>(event: *mut rs_lvgl_sys::lv_event_t) {
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
