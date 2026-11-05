//! Recursive message conversion without changing descriptor properties.

use super::Node;
use alloc::rc::Rc;

impl<M, K> Node<M, K> {
    /// Convert every message this subtree produces with `map`.
    ///
    /// This is how a component with its own message type is embedded in an
    /// application: `menu::view(&state).map(ParentAction::Menu)`. Click messages are
    /// converted once, when `map` is called; value, text and event mappings
    /// are converted when the event fires. `map` must be a pure conversion.
    pub fn map<N: 'static>(self, map: impl Fn(M) -> N + 'static) -> Node<N, K>
    where
        M: 'static,
    {
        let map: Rc<dyn Fn(M) -> N> = Rc::new(map);
        self.map_with(&map)
    }

    fn map_with<N: 'static>(self, map: &Rc<dyn Fn(M) -> N>) -> Node<N, K>
    where
        M: 'static,
    {
        let handlers = self.handlers.map(map);
        Node {
            marker: core::marker::PhantomData,
            #[cfg(any(feature = "diagnostics", feature = "strict-properties"))]
            invalid_property: self.invalid_property,
            static_text: self.static_text,
            font_resource: self.font_resource,
            #[cfg(feature = "widget-image")]
            image_resource: self.image_resource,
            key: self.key,
            kind: self.kind,
            style: self.style,
            part_styles: self.part_styles,
            keyboard_id: self.keyboard_id,
            grid_cell: self.grid_cell,
            children: self
                .children
                .into_iter()
                .map(|child| child.map_with(map))
                .collect(),
            handlers,
            clickable: self.clickable,
            scrollable: self.scrollable,
            checked: self.checked,
            interaction: self.interaction,
            animations: self.animations,
            #[cfg(feature = "input-keypad")]
            focusable: self.focusable,
            #[cfg(feature = "input-keypad")]
            modal: self.modal,
            #[cfg(feature = "input-gridnav")]
            grid_navigation: self.grid_navigation,
        }
    }
}
