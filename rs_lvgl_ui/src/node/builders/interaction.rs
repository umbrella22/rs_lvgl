//! Focus, object interaction flags and scroll control.

use crate::node::{Node, ScrollDir};

impl<M, K> Node<M, K> {
    /// Control `LV_STATE_CHECKED`, which selects [`crate::State::CHECKED`] styles.
    ///
    /// The state is controlled: it is compared with the native object on
    /// every render. Left unset, the state is not touched. Switches and
    /// checkboxes ignore this; their checked value is part of the widget.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    /// Set LVGL's `LV_OBJ_FLAG_CLICKABLE` explicitly.
    ///
    /// Left unset, the widget keeps LVGL's default for its class (plain
    /// objects and containers are clickable). Handlers never change the flag.
    pub fn clickable(mut self, clickable: bool) -> Self {
        self.clickable = Some(clickable);
        self
    }

    /// Set LVGL's `LV_OBJ_FLAG_SCROLLABLE` explicitly.
    ///
    /// Left unset, the widget keeps LVGL's default for its class (plain
    /// objects and containers are scrollable when content overflows).
    pub fn scrollable(mut self, scrollable: bool) -> Self {
        self.scrollable = Some(scrollable);
        self
    }

    /// Override the native class's default focus-group membership. Hidden
    /// or disabled nodes (including descendants of such nodes) are excluded.
    /// Unset restores the class default on the next render.
    #[cfg(feature = "input-keypad")]
    pub fn focusable(mut self, enabled: bool) -> Self {
        self.focusable = Some(enabled);
        self
    }

    /// Restrict keyboard/encoder focus to this subtree while it is visible and
    /// enabled. The last modal in view/overlay preorder wins. Closing restores
    /// the previous mounted node when still eligible, including auto-keyed nodes.
    /// Pointer blocking requires a backdrop (provided by components::Dialog).
    /// Has no effect until the UI's focus group is explicitly activated.
    #[cfg(feature = "input-keypad")]
    pub fn modal(mut self, enabled: bool) -> Self {
        self.modal = enabled;
        self
    }

    /// Navigate the direct children of an Obj/Row/Column/Grid with arrow keys.
    /// The container occupies one Tab position after explicit focus activation.
    /// Nested navigation or modal scopes inside a grid are rejected; place
    /// independent grids/modals in sibling containers or in App::overlay.
    #[cfg(feature = "input-gridnav")]
    pub fn grid_navigation(mut self, navigation: crate::GridNavigation) -> Self {
        self.grid_navigation = Some(navigation);
        self
    }

    /// Enable native event propagation to the parent. Use on_bubbled_event on ancestors.
    /// Unset leaves the existing native setting unchanged.
    pub fn event_bubble(mut self, value: bool) -> Self {
        self.interaction.event_bubble = Some(value);
        self
    }

    /// Let LVGL choose a gesture target above this node, following its gesture-bubble flags.
    /// Unset leaves the existing native setting unchanged.
    pub fn gesture_bubble(mut self, value: bool) -> Self {
        self.interaction.gesture_bubble = Some(value);
        self
    }

    /// Override allowed native scroll directions.
    /// Unset leaves the existing native setting unchanged.
    pub fn scroll_dir(mut self, value: ScrollDir) -> Self {
        self.interaction.dir = Some(value);
        self
    }

    /// Set horizontal snapping alignment for snappable children.
    /// Unset leaves the existing native setting unchanged.
    pub fn scroll_snap_x(mut self, value: crate::ScrollSnap) -> Self {
        self.interaction.snap_x = Some(value);
        self
    }

    /// Set vertical snapping alignment for snappable children.
    /// Unset leaves the existing native setting unchanged.
    pub fn scroll_snap_y(mut self, value: crate::ScrollSnap) -> Self {
        self.interaction.snap_y = Some(value);
        self
    }

    /// Override native scrollbar visibility.
    /// Unset leaves the existing native setting unchanged.
    pub fn scrollbar_mode(mut self, value: crate::ScrollbarMode) -> Self {
        self.interaction.scrollbar = Some(value);
        self
    }

    /// Enable or disable native momentum scrolling.
    /// Unset leaves the existing native setting unchanged.
    pub fn scroll_momentum(mut self, value: bool) -> Self {
        self.interaction.momentum = Some(value);
        self
    }

    /// Enable or disable native elastic scrolling.
    /// Unset leaves the existing native setting unchanged.
    pub fn scroll_elastic(mut self, value: bool) -> Self {
        self.interaction.elastic = Some(value);
        self
    }

    /// Control the absolute scroll offset in pixels after layout. LVGL
    /// clips it to content bounds. Unset releases control. Native scroll
    /// events, including programmatic ones, still reach on_scroll.
    /// Values outside LVGL's LV_COORD_MIN..=LV_COORD_MAX fail validation.
    pub fn scroll_to(mut self, x: i32, y: i32) -> Self {
        self.interaction.offset = Some(crate::ScrollOffset { x, y });
        self
    }
}
