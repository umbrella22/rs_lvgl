//! Independent handlers retained by each descriptor and message mapping.

use alloc::{boxed::Box, rc::Rc};

/// Boxed value-changed mapping registered with [`crate::Node::on_value_changed`].
pub type ValueChangedHandler<M> = Box<dyn FnMut(i32) -> M>;

pub(crate) type ControlHandler<M> = Box<dyn FnMut(crate::events::ControlValue) -> Option<M>>;

/// Boxed event mapping registered with [`crate::Node::on_event`].
pub type EventHandler<M> = Box<dyn FnMut(&crate::Event) -> Option<M>>;

/// Boxed text-changed mapping registered with [`crate::Node::on_text_changed`].
pub type TextChangedHandler<M> = Box<dyn FnMut(&str) -> M>;

pub(crate) type BubbledHandler<M> = Box<dyn FnMut(&crate::InputEvent) -> Option<M>>;

/// Legacy single-handler representation.
///
/// Nodes no longer store this enum: each `Node::on_*` builder now sets an
/// independent handler, so `on_click` and `on_event` can be combined. The type
/// stays exported for source compatibility and is not read by the runtime.
#[derive(Default)]
pub enum Handlers<M> {
    #[default]
    None,
    Click(M),
    ValueChanged(ValueChangedHandler<M>),
    TextChanged(TextChangedHandler<M>),
    Event(EventHandler<M>),
}

/// Handlers retained by one mounted node. Every handler is independent.
pub(crate) struct HandlerSet<M> {
    pub(crate) click: Option<M>,
    pub(crate) value_changed: Option<ControlHandler<M>>,
    pub(crate) text_changed: Option<TextChangedHandler<M>>,
    pub(crate) event: Option<EventHandler<M>>,
    pub(crate) key: Option<Box<dyn FnMut(u32) -> M>>,
    pub(crate) rotary: Option<Box<dyn FnMut(i32) -> M>>,
    pub(crate) gesture: Option<Box<dyn FnMut(crate::GestureDirection) -> M>>,
    pub(crate) scroll: Option<Box<dyn FnMut(crate::ScrollOffset) -> M>>,
    pub(crate) pointer: Option<Box<dyn FnMut(crate::PointerEvent) -> M>>,
    pub(crate) long_press: Option<M>,
    pub(crate) bubbled: Option<BubbledHandler<M>>,
}

impl<M> Default for HandlerSet<M> {
    fn default() -> Self {
        Self {
            click: None,
            value_changed: None,
            text_changed: None,
            event: None,
            key: None,
            rotary: None,
            gesture: None,
            scroll: None,
            pointer: None,
            long_press: None,
            bubbled: None,
        }
    }
}

impl<M> HandlerSet<M> {
    pub(crate) fn has_input(&self) -> bool {
        self.key.is_some()
            || self.rotary.is_some()
            || self.gesture.is_some()
            || self.scroll.is_some()
            || self.pointer.is_some()
            || self.long_press.is_some()
            || self.bubbled.is_some()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.click.is_none()
            && self.value_changed.is_none()
            && self.text_changed.is_none()
            && self.event.is_none()
            && !self.has_input()
    }
}

impl<M: 'static> HandlerSet<M> {
    pub(super) fn map<N: 'static>(self, map: &Rc<dyn Fn(M) -> N>) -> HandlerSet<N> {
        let HandlerSet {
            click,
            value_changed,
            text_changed,
            event,
            key,
            rotary,
            gesture,
            scroll,
            pointer,
            long_press,
            bubbled,
        } = self;
        HandlerSet {
            key: map_value_handler(key, map),
            rotary: map_value_handler(rotary, map),
            gesture: map_value_handler(gesture, map),
            scroll: map_value_handler(scroll, map),
            pointer: map_value_handler(pointer, map),
            long_press: long_press.map(|msg| map(msg)),
            bubbled: bubbled.map(|mut handler| {
                let map = map.clone();
                Box::new(move |event: &crate::InputEvent| handler(event).map(|msg| map(msg)))
                    as BubbledHandler<N>
            }),
            click: click.map(|msg| map(msg)),
            value_changed: value_changed.map(|mut handler| {
                let map = map.clone();
                Box::new(move |value| handler(value).map(|action| map(action))) as ControlHandler<N>
            }),
            text_changed: text_changed.map(|mut handler| {
                let map = map.clone();
                Box::new(move |text: &str| map(handler(text))) as TextChangedHandler<N>
            }),
            event: event.map(|mut handler| {
                let map = map.clone();
                Box::new(move |event: &crate::Event| handler(event).map(|msg| map(msg)))
                    as EventHandler<N>
            }),
        }
    }
}

/// Adapt owned payloads with one shared message conversion per subtree.
fn map_value_handler<T: 'static, M: 'static, N: 'static>(
    handler: Option<Box<dyn FnMut(T) -> M>>,
    map: &Rc<dyn Fn(M) -> N>,
) -> Option<Box<dyn FnMut(T) -> N>> {
    handler.map(|mut handler| {
        let map = map.clone();
        Box::new(move |value| map(handler(value))) as Box<dyn FnMut(T) -> N>
    })
}
