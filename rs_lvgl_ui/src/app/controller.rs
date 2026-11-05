//! Public UI construction, message processing and explicit runtime operations.

use super::guard::enter;
use super::{App, DrainStats, RenderDecision, Ui, UiStats, DEFAULT_QUEUE_CAPACITY};
use crate::events::EventQueue;
use crate::reconciler::Counters;
use crate::{Theme, UiError};
use alloc::rc::Rc;
use core::{cell::Cell, ptr::NonNull};
use rs_lvgl::{Display, NativeObject};
use rs_lvgl_sys::lv_obj_t;

impl<M: Clone + 'static> Ui<M> {
    /// Create a UI on the display's active screen.
    ///
    /// LVGL must be initialized and the display must outlive this `Ui` under
    /// [`Ui::unmount`]'s lifecycle contract.
    pub fn new(display: &Display, theme: Theme) -> Result<Self, UiError> {
        Self::with_capacity(display, theme, DEFAULT_QUEUE_CAPACITY)
    }

    /// Create a UI with a bounded message queue.
    pub fn with_capacity(
        display: &Display,
        theme: Theme,
        capacity: usize,
    ) -> Result<Self, UiError> {
        if capacity == 0 {
            return Err(UiError::InvalidProperty);
        }
        let screen = display
            .get_scr_act()
            .map_err(|_| UiError::DisplayUnavailable)?;
        let screen = screen.raw();
        Self::on_screen(screen, theme, capacity)
    }

    // The caller retains this native screen for the entire mounted lifetime.
    pub(crate) fn on_screen(
        screen: NonNull<lv_obj_t>,
        theme: Theme,
        capacity: usize,
    ) -> Result<Self, UiError> {
        let mut ui = Self {
            screen,
            theme,
            queue: Rc::new(EventQueue::new(capacity)),
            root: None,
            mounted: None,
            overlay_root: None,
            overlay: None,
            busy: Cell::new(false),
            counters: Counters::default(),
            messages_processed: 0,
            reconciled_updates: 0,
            dirty: false,
            #[cfg(feature = "input-keypad")]
            focus: None,
        };
        ui.ensure_root()?;
        Ok(ui)
    }

    /// Reconcile the application's first tree.
    ///
    /// Calling `mount` again reconciles against the existing mounted tree.
    pub fn mount<A: App<Action = M>>(&mut self, app: &A) -> Result<(), UiError> {
        let _guard = enter(&self.busy)?;
        let tree = app.view(&self.theme);
        let overlay = app.overlay(&self.theme);
        self.render(tree, overlay)
    }

    /// Process at most `max_messages` queued messages, then reconcile once.
    ///
    /// When no message was processed and nothing marked the UI dirty, the
    /// drain returns without calling `App::view`, so an idle main loop does
    /// not rebuild and diff the whole tree. State that changes outside
    /// messages (clocks, sensors read in `view`) must call
    /// [`Ui::mark_dirty`] or [`Ui::update`].
    pub fn drain<A: App<Action = M>>(
        &mut self,
        app: &mut A,
        max_messages: usize,
    ) -> Result<DrainStats, UiError> {
        let _guard = enter(&self.busy)?;
        let dropped_before = self.queue.dropped();
        let mut processed = 0;
        let mut requested = false;
        while processed < max_messages {
            let Some(message) = self.queue.pop() else {
                break;
            };
            requested |= app.update_with_render(message) == RenderDecision::Reconcile;
            processed += 1;
        }
        self.messages_processed += processed as u64;
        let reconciled = requested || self.dirty || self.mounted.is_none();
        if reconciled {
            let tree = app.view(&self.theme);
            let overlay = app.overlay(&self.theme);
            self.render(tree, overlay)?;
        }
        Ok(DrainStats {
            processed,
            dropped: self.queue.dropped() - dropped_before,
            reconciled,
        })
    }

    /// Reconcile without processing queued messages.
    pub fn update<A: App<Action = M>>(&mut self, app: &A) -> Result<(), UiError> {
        let _guard = enter(&self.busy)?;
        let tree = app.view(&self.theme);
        let overlay = app.overlay(&self.theme);
        self.render(tree, overlay)
    }

    /// Request a reconciliation on the next [`Ui::drain`] even if no message
    /// is processed.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Enqueue a message from the platform or an external event source.
    pub fn post(&mut self, message: M) -> Result<(), UiError> {
        let _guard = enter(&self.busy)?;
        self.queue.post(message);
        Ok(())
    }

    /// Enqueue with an explicit policy and return any rejected/evicted value.
    /// The legacy `post` method always retains its drop-oldest behavior.
    pub fn post_with(
        &mut self,
        message: M,
        policy: crate::QueuePolicy<M>,
    ) -> Result<crate::PostOutcome<M>, UiError> {
        let _guard = enter(&self.busy)?;
        Ok(self.queue.post_with(message, policy))
    }

    /// Select congestion behavior for messages produced by native handlers.
    /// Existing queued messages retain their order. Defaults to drop-oldest.
    pub fn set_event_queue_policy(&mut self, policy: crate::QueuePolicy<M>) {
        self.queue.set_event_policy(policy);
    }

    /// Observe native-handler outcomes, including rejected message values.
    ///
    /// This runs synchronously inside the native callback on the UI thread.
    /// It must not unwind, block, or call Ui methods. Save work for the next
    /// platform-loop iteration. `None` releases the observer.
    pub fn set_event_queue_observer(
        &mut self,
        observer: Option<Rc<dyn Fn(crate::PostOutcome<M>)>>,
    ) {
        self.queue.set_event_observer(observer);
    }

    pub fn queue_stats(&self) -> crate::QueueStats {
        self.queue.stats()
    }

    /// Install an optional structured path sink. The sink runs during UI
    /// operations; it must not unwind or reenter Ui. No property values appear.
    /// A sink panic aborts at an internal non-unwinding callback boundary.
    #[cfg(feature = "diagnostics")]
    pub fn set_diagnostic_sink(&mut self, sink: Option<crate::DiagnosticSink>) {
        self.queue.diagnostics.set(sink);
    }

    /// Advance every mounted custom widget by `elapsed_ms`.
    ///
    /// Walks the mounted view, then the overlay, once each, pre-order, and calls
    /// [`crate::CustomWidget::update`] for each lifecycle custom widget. It does
    /// not call `App::view`, reconcile descriptors, process queued messages or
    /// change the mounted tree.
    ///
    /// `tick` is mutually exclusive with [`Ui::mount`], [`Ui::drain`],
    /// [`Ui::update`], [`Ui::post`] and [`Ui::timer_handler`]: a reentrant call
    /// from a widget hook returns [`UiError::Reentrant`]. The first update error
    /// stops the walk and is returned; the tree is left mounted. Subtrees
    /// without lifecycle widgets are skipped.
    pub fn tick(&mut self, elapsed_ms: u32) -> Result<(), UiError> {
        let _guard = enter(&self.busy)?;
        if let Some(mounted) = self.mounted.as_mut() {
            mounted.tick(elapsed_ms)?;
        }
        if let Some(overlay) = self.overlay.as_mut() {
            overlay.tick(elapsed_ms)?;
        }
        Ok(())
    }

    /// Thin wrapper over `lv_timer_handler`.
    ///
    /// Callbacks dispatched here only enqueue messages; call [`Ui::drain`]
    /// afterwards to update application state. The busy guard rejects a nested
    /// call (for example a [`Ui::tick`] from inside a timer callback) and
    /// returns `0` in that case.
    pub fn timer_handler(&mut self) -> u32 {
        let Ok(_guard) = enter(&self.busy) else {
            return 0;
        };
        // SAFETY: the platform calls this on the single UI thread.
        unsafe { rs_lvgl_sys::lv_timer_handler() }
    }

    /// Change the theme used by subsequent `view` calls.
    ///
    /// The next `update`, `mount` or `drain` re-renders with the new tokens.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.dirty = true;
    }

    /// Capture the mounted view (excluding top-layer overlays) as owned RGBA
    /// pixels. The snapshot includes native extra draw bounds such as shadows.
    pub fn snapshot(&mut self) -> Result<crate::Snapshot, UiError> {
        let _guard = enter(&self.busy)?;
        let root = self.mounted.as_ref().ok_or(UiError::InvalidProperty)?.obj;
        crate::Snapshot::take(root)
    }

    pub fn stats(&self) -> UiStats {
        UiStats {
            mounted_nodes: self.mounted.as_ref().map(|m| m.count).unwrap_or(0)
                + self.overlay.as_ref().map(|m| m.count).unwrap_or(0),
            messages_processed: self.messages_processed,
            messages_dropped: self.queue.dropped(),
            reconciled_updates: self.reconciled_updates,
            created_nodes: self.counters.created,
            deleted_nodes: self.counters.deleted,
            moved_nodes: self.counters.moved,
        }
    }
}
