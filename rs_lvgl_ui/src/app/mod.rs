//! Application contract and per-UI runtime state.

mod controller;
mod guard;
#[cfg(feature = "input-keypad")]
mod input;
mod lifecycle;
mod render;

use crate::events::EventQueue;
use crate::reconciler::{Counters, MountedNode};
use crate::{Theme, View};
use alloc::rc::Rc;
use core::{cell::Cell, ptr::NonNull};
use rs_lvgl_sys::lv_obj_t;

/// Default capacity of the message queue.
pub const DEFAULT_QUEUE_CAPACITY: usize = 64;

/// Visual work required after a message. Skip is an explicit application
/// promise: neither view/overlay changes nor controlled-native correction are
/// required. An unchanged model alone does not establish this promise.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderDecision {
    Reconcile,
    Skip,
}

/// An application screen.
///
/// `update` changes application state only. `view` reads state and returns a
/// descriptor tree. Neither method may call LVGL or block.
///
/// `Msg` must be `Clone` because a click handler stores its message and clones
/// it for every click.
pub trait App {
    type Msg: Clone + 'static;

    fn update(&mut self, msg: Self::Msg);

    /// Process a message with optional reconciliation feedback. Existing apps
    /// call update and reconcile by default. A batch reconciles if any message
    /// requests it; dirty/theme/first-mount requirements always take priority.
    ///
    /// A user may change a native slider from 50 to 80 while update clamps its
    /// model back to 50. That still requires Reconcile to restore the widget.
    fn update_with_render(&mut self, msg: Self::Msg) -> RenderDecision {
        self.update(msg);
        RenderDecision::Reconcile
    }
    fn view(&self, theme: &Theme) -> View<Self::Msg>;

    /// Optional tree mounted above the view, on the display's top layer.
    ///
    /// Dialogs, toasts and other overlays render here so they cover the whole
    /// screen regardless of the view's layout. The overlay root fills the
    /// display, has no style and is not clickable: input reaches the view
    /// except where an overlay widget (or a clickable backdrop) is hit.
    /// Returning `None` deletes the overlay tree.
    ///
    /// The overlay is reconciled together with the view and shares its
    /// message queue. Keyboard ids are resolved across both trees, so they
    /// should be unique across the view and the overlay.
    fn overlay(&self, theme: &Theme) -> Option<View<Self::Msg>> {
        let _ = theme;
        None
    }
}

/// Diagnostic counters exposed by [`Ui::stats`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiStats {
    /// Number of application nodes in the mounted view and overlay trees.
    /// Implicit button labels are not counted.
    pub mounted_nodes: usize,
    pub messages_processed: u64,
    pub messages_dropped: u64,
    pub reconciled_updates: u64,
    pub created_nodes: u64,
    pub deleted_nodes: u64,
    pub moved_nodes: u64,
}

/// Result of one [`Ui::drain`] call.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DrainStats {
    pub processed: usize,
    pub dropped: u64,
    /// Whether this drain called `App::view` and reconciled. A drain that
    /// processed no message skips reconciliation unless [`Ui::mark_dirty`] or
    /// [`Ui::set_theme`] requested one, or no tree is mounted.
    pub reconciled: bool,
}

/// A mounted declarative UI.
///
/// `Ui` is neither `Send` nor `Sync`. One instance owns one root container on
/// one LVGL screen and, while [`App::overlay`] returns a tree, one overlay
/// root on that display's top layer.
pub struct Ui<M: Clone + 'static> {
    screen: NonNull<lv_obj_t>,
    theme: Theme,
    queue: Rc<EventQueue<M>>,
    root: Option<NonNull<lv_obj_t>>,
    mounted: Option<MountedNode<M>>,
    overlay_root: Option<NonNull<lv_obj_t>>,
    overlay: Option<MountedNode<M>>,
    busy: Cell<bool>,
    counters: Counters,
    messages_processed: u64,
    reconciled_updates: u64,
    dirty: bool,
    #[cfg(feature = "input-keypad")]
    focus: Option<crate::focus::FocusGroup>,
}
