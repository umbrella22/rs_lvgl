//! Native event-to-message builder methods.

use crate::node::Node;
use alloc::boxed::Box;

impl<M> Node<M> {
    ///
    /// Messages are cloned for every click, so `M` must implement `Clone`.
    /// Handlers are independent: this can be combined with the other `on_*`
    /// builders; calling the same builder twice replaces that handler only.
    pub fn on_click(mut self, action: M) -> Self
    where
        M: Clone,
    {
        self.handlers.click = Some(action);
        self
    }

    /// Map a value-changed event to a message.
    ///
    /// The value is read from the native widget after the change: the value
    /// of sliders, bars and arcs; `1`/`0` for checked/unchecked switches and
    /// checkboxes; the selected index of dropdowns and rollers; the value of
    /// spinboxes; the active tab index of tabviews; the active tile index of
    /// tileviews (`-1` before any tile is active); the pressed date of
    /// calendars packed by [`CalendarDate::to_value`](crate::CalendarDate::to_value) (`-1` when none); the
    /// clicked footer button index of message boxes. Other kinds pass the
    /// value documented on their [`WidgetKind`](crate::WidgetKind) variant, or `0`.
    pub fn on_value_changed<F>(mut self, map: F) -> Self
    where
        F: FnMut(i32) -> M + 'static,
    {
        self.handlers.value_changed = Some(Box::new(map));
        self
    }

    /// Map a textarea's current text to a message.
    ///
    /// The string is borrowed from LVGL for the duration of the call; copy it
    /// if the message must retain it. Non-UTF-8 native text is skipped.
    pub fn on_text_changed<F>(mut self, map: F) -> Self
    where
        F: FnMut(&str) -> M + 'static,
    {
        self.handlers.text_changed = Some(Box::new(map));
        self
    }

    /// Map the full 32-bit LVGL key code. NEXT/PREV are consumed by focus navigation.
    pub fn on_key<F>(mut self, map: F) -> Self
    where
        F: FnMut(u32) -> M + 'static,
    {
        self.handlers.key = Some(Box::new(map));
        self
    }

    /// Map the signed native rotary delta without truncation.
    pub fn on_rotary<F>(mut self, map: F) -> Self
    where
        F: FnMut(i32) -> M + 'static,
    {
        self.handlers.rotary = Some(Box::new(map));
        self
    }

    /// Map one directional gesture event (the generic Gesture event is separate).
    pub fn on_gesture<F>(mut self, map: F) -> Self
    where
        F: FnMut(crate::GestureDirection) -> M + 'static,
    {
        self.handlers.gesture = Some(Box::new(map));
        self
    }

    /// Map the actual offset in pixels on continuous LV_EVENT_SCROLL, including programmatic scrolling.
    pub fn on_scroll<F>(mut self, map: F) -> Self
    where
        F: FnMut(crate::ScrollOffset) -> M + 'static,
    {
        self.handlers.scroll = Some(Box::new(map));
        self
    }

    /// Map press/click/release pointer events with their display coordinates.
    pub fn on_pointer<F>(mut self, map: F) -> Self
    where
        F: FnMut(crate::PointerEvent) -> M + 'static,
    {
        self.handlers.pointer = Some(Box::new(map));
        self
    }

    /// Enqueue once on LV_EVENT_LONG_PRESSED (not its repeat event).
    pub fn on_long_press(mut self, action: M) -> Self {
        self.handlers.long_press = Some(action);
        self
    }

    /// Observe events that LVGL bubbles from descendants. Direct handlers
    /// remain local. Enable event_bubble on each intervening child for
    /// ordinary events; gesture_bubble follows LVGL gesture target rules.
    pub fn on_bubbled_event<F>(mut self, map: F) -> Self
    where
        F: FnMut(&crate::InputEvent) -> Option<M> + 'static,
    {
        self.handlers.bubbled = Some(Box::new(map));
        self
    }

    /// Map any supported event to an optional message.
    ///
    /// Only events whose target is this node are delivered; events bubbled
    /// from descendants are ignored.
    pub fn on_event<F>(mut self, map: F) -> Self
    where
        F: FnMut(&crate::Event) -> Option<M> + 'static,
    {
        self.handlers.event = Some(Box::new(map));
        self
    }
}
