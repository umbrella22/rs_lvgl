//! Map direct/bubbled native events to queued messages.

use super::HandlerSlot;
use crate::events::value::read_value;
#[cfg(feature = "widget-textarea")]
use crate::events::value::with_text;
use crate::events::{map_event, InputEvent};
use rs_lvgl_sys::lv_obj_t;

impl<M: Clone + 'static> HandlerSlot<M> {
    pub(super) fn dispatch(
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
            self.queue.post_event(message);
        }
    }
}
