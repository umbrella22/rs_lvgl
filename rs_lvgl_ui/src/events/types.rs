//! Supported events and copied native input payloads.

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
pub(super) unsafe fn input_event(event: *mut rs_lvgl_sys::lv_event_t) -> Option<InputEvent> {
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
