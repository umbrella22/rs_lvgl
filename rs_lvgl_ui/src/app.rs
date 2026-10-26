use crate::View;
use alloc::rc::Rc;
use core::cell::Cell;
use core::ptr::NonNull;

use rs_lvgl::Display;
use rs_lvgl::NativeObject;
use rs_lvgl_sys::lv_obj_t;

use crate::events::EventQueue;
use crate::node::{self, Size};
use crate::reconciler::{self, Counters, MountedNode};
use crate::theme::{Style, Theme};
use crate::widgets::{self, KindTag};
use crate::UiError;

/// Default capacity of the message queue.
pub const DEFAULT_QUEUE_CAPACITY: usize = 64;

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
    pub fn mount<A: App<Msg = M>>(&mut self, app: &A) -> Result<(), UiError> {
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
    pub fn drain<A: App<Msg = M>>(
        &mut self,
        app: &mut A,
        max_messages: usize,
    ) -> Result<DrainStats, UiError> {
        let _guard = enter(&self.busy)?;
        let dropped_before = self.queue.dropped();
        let mut processed = 0;
        while processed < max_messages {
            let Some(message) = self.queue.pop() else {
                break;
            };
            app.update(message);
            processed += 1;
        }
        self.messages_processed += processed as u64;
        let reconciled = processed > 0 || self.dirty || self.mounted.is_none();
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
    pub fn update<A: App<Msg = M>>(&mut self, app: &A) -> Result<(), UiError> {
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

    /// Focus a unique non-Auto key across the view and overlay. Returns false
    /// for missing or ineligible nodes; ambiguous keys and Auto are invalid.
    /// This does not change sibling reconciliation identity rules.
    /// Enable focus with `enable_focus` or bind an input device first.
    #[cfg(feature = "input-keypad")]
    pub fn focus(&mut self, key: crate::Key) -> Result<bool, UiError> {
        let _guard = enter(&self.busy)?;
        if key == crate::Key::Auto {
            return Err(UiError::InvalidProperty);
        }
        self.focus
            .as_ref()
            .map(|group| group.focus(&[self.mounted.as_ref(), self.overlay.as_ref()], key))
            .unwrap_or(Ok(false))
    }

    /// Bind a live typed keypad to this UI's group.
    #[cfg(feature = "input-keypad")]
    pub fn bind_keypad(&mut self, keypad: &rs_lvgl::input_device::Keypad) -> Result<(), UiError> {
        let raw = keypad.raw().ok_or(UiError::InvalidProperty)?;
        // SAFETY: the keypad exposes a live descriptor on the same UI thread.
        unsafe { self.bind_input(raw) }
    }

    /// Activate this UI's focus group, even without a bound device.
    /// Merely enabling the Cargo feature does not change existing screens'
    /// focus or emit new focus messages. Binding an input also activates it.
    #[cfg(feature = "input-keypad")]
    pub fn enable_focus(&mut self) -> Result<(), UiError> {
        let _guard = enter(&self.busy)?;
        if self.focus.is_none() {
            self.focus = Some(crate::focus::FocusGroup::new()?);
        }
        self.sync_focus()
    }

    /// Bind a platform keypad or encoder (including LVGL's SDL devices).
    ///
    /// # Safety
    /// `input` must be a live LVGL input device on this UI thread. Do not
    /// mutate/delete it reentrantly during callbacks. Native group deletion
    /// automatically detaches the device, so either may be deleted first.
    #[cfg(feature = "input-keypad")]
    pub unsafe fn bind_input(
        &mut self,
        input: NonNull<rs_lvgl_sys::lv_indev_t>,
    ) -> Result<(), UiError> {
        let _guard = enter(&self.busy)?;
        // SAFETY: the caller guarantees input validity; screen is live under
        // Ui's lifecycle contract. Reject foreign displays before mutation.
        unsafe {
            let kind = rs_lvgl_sys::lv_indev_get_type(input.as_ptr());
            if !matches!(
                kind,
                rs_lvgl_sys::lv_indev_type_t_LV_INDEV_TYPE_KEYPAD
                    | rs_lvgl_sys::lv_indev_type_t_LV_INDEV_TYPE_ENCODER
            ) || rs_lvgl_sys::lv_indev_get_display(input.as_ptr())
                != rs_lvgl_sys::lv_obj_get_display(self.screen.as_ptr())
            {
                return Err(UiError::InvalidProperty);
            }
            if self.focus.is_none() {
                self.focus = Some(crate::focus::FocusGroup::new()?);
            }
            rs_lvgl_sys::lv_indev_set_group(
                input.as_ptr(),
                self.focus.as_ref().expect("group was just ensured").raw(),
            );
        }
        self.sync_focus()
    }

    /// Select native encoder editing or navigation mode.
    #[cfg(feature = "input-keypad")]
    pub fn set_editing(&mut self, editing: bool) -> Result<(), UiError> {
        let _guard = enter(&self.busy)?;
        let group = self.focus.as_ref().ok_or(UiError::InvalidProperty)?;
        // SAFETY: the per-UI group is live on this thread.
        unsafe { rs_lvgl_sys::lv_group_set_editing(group.raw(), editing) };
        Ok(())
    }

    /// Enqueue a message from the platform or an external event source.
    pub fn post(&mut self, message: M) -> Result<(), UiError> {
        let _guard = enter(&self.busy)?;
        self.queue.post(message);
        Ok(())
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

    /// Delete the root containers and clear the mounted view and overlay.
    ///
    /// Must run before the display is deleted, before `rs_lvgl::deinit`, and
    /// before the `Display` handle used to construct this `Ui` is dropped.
    /// Releases the input group and detaches devices. Bind inputs again after
    /// a subsequent mount. Dropping this unmounted UI needs no native state.
    pub fn unmount(&mut self) {
        let Ok(_guard) = enter(&self.busy) else {
            return;
        };
        self.delete_overlay();
        self.delete_root();
        #[cfg(feature = "input-keypad")]
        {
            self.focus = None;
        }
    }

    pub(crate) fn deactivate(&mut self) {
        #[cfg(feature = "input-keypad")]
        {
            self.focus = None;
        }
        if let Some(root) = self.overlay_root {
            // SAFETY: the outgoing overlay is live until its page is retired.
            unsafe { rs_lvgl_sys::lv_obj_set_hidden(root.as_ptr(), true) };
        }
    }

    fn ensure_root(&mut self) -> Result<(), UiError> {
        if self.root.is_none() {
            // SAFETY: `screen` belongs to the live display supplied by the
            // platform and remains valid under the lifecycle contract.
            let root = unsafe { rs_lvgl_sys::lv_obj_create(self.screen.as_ptr()) };
            let Some(root) = NonNull::new(root) else {
                return Err(UiError::Allocation);
            };
            // The theme's card style would otherwise give the root a
            // background, border and text color that `Style::neutral` does
            // not reset.
            widgets::remove_main_theme(root);
            let style = Style::neutral()
                .width(Size::Percent(100))
                .height(Size::Percent(100));
            widgets::apply_style(root, &style, KindTag::Obj);
            self.root = Some(root);
        }
        Ok(())
    }

    fn render(&mut self, tree: View<M>, overlay: Option<View<M>>) -> Result<(), UiError> {
        node::validate(&tree)?;
        if let Some(overlay) = &overlay {
            node::validate(overlay)?;
        }
        #[cfg(feature = "input-keypad")]
        if let Some(group) = &mut self.focus {
            group.capture(&[self.mounted.as_ref(), self.overlay.as_ref()]);
            #[cfg(feature = "input-gridnav")]
            for tree in [&mut self.mounted, &mut self.overlay].into_iter().flatten() {
                crate::gridnav::capture(tree);
            }
        }
        self.ensure_root()?;
        let root = self.root.expect("root was just ensured");
        let queue = self.queue.clone();
        reconcile_tree(root, &mut self.mounted, tree, &queue, &mut self.counters)?;

        match overlay {
            Some(overlay) => {
                let overlay_root = self.ensure_overlay_root()?;
                reconcile_tree(
                    overlay_root,
                    &mut self.overlay,
                    overlay,
                    &queue,
                    &mut self.counters,
                )?;
            }
            None => self.delete_overlay(),
        }

        let trees = [self.mounted.as_ref(), self.overlay.as_ref()];
        reconciler::bind_keyboards(&trees, &mut self.counters);
        for tree in trees.iter().flatten() {
            crate::scroll::apply_offsets(tree);
        }
        #[cfg(feature = "input-keypad")]
        self.sync_focus()?;
        self.reconciled_updates += 1;
        self.dirty = false;
        Ok(())
    }

    #[cfg(feature = "input-keypad")]
    fn sync_focus(&mut self) -> Result<(), UiError> {
        if let Some(group) = &mut self.focus {
            #[cfg(feature = "input-gridnav")]
            for tree in [&mut self.mounted, &mut self.overlay].into_iter().flatten() {
                crate::gridnav::sync(tree);
            }
            if let Err(error) = group.sync(&[self.mounted.as_ref(), self.overlay.as_ref()]) {
                self.delete_overlay();
                self.delete_root();
                self.focus = None;
                return Err(error);
            }
        }
        Ok(())
    }

    fn ensure_overlay_root(&mut self) -> Result<NonNull<lv_obj_t>, UiError> {
        if let Some(root) = self.overlay_root {
            return Ok(root);
        }
        // SAFETY: `screen` is live under the lifecycle contract, so its
        // display and that display's top layer are live too.
        let root = unsafe {
            let display = rs_lvgl_sys::lv_obj_get_display(self.screen.as_ptr());
            let layer = rs_lvgl_sys::lv_display_get_layer_top(display);
            if layer.is_null() {
                return Err(UiError::DisplayUnavailable);
            }
            rs_lvgl_sys::lv_obj_create(layer)
        };
        let Some(root) = NonNull::new(root) else {
            return Err(UiError::Allocation);
        };
        widgets::remove_main_theme(root);
        let style = Style::neutral()
            .width(Size::Percent(100))
            .height(Size::Percent(100));
        widgets::apply_style(root, &style, KindTag::Obj);
        // SAFETY: `root` was just created. A non-clickable, non-scrollable
        // root lets input fall through to the view where no overlay widget
        // is hit.
        unsafe {
            rs_lvgl_sys::lv_obj_set_clickable(root.as_ptr(), false);
            rs_lvgl_sys::lv_obj_set_scrollable(root.as_ptr(), false);
        }
        self.overlay_root = Some(root);
        Ok(root)
    }

    fn delete_overlay(&mut self) {
        if let Some(overlay) = self.overlay.take() {
            reconciler::delete_subtree(overlay, &mut self.counters);
        }
        if let Some(root) = self.overlay_root.take() {
            // SAFETY: the overlay root is live and its mounted tree was
            // deleted above, so no Rust handle refers to its children.
            unsafe { rs_lvgl_sys::lv_obj_delete(root.as_ptr()) };
        }
    }

    fn delete_root(&mut self) {
        if let Some(root) = self.root.take() {
            let mut mounted = self.mounted.take();
            let count = match mounted.as_mut() {
                Some(mounted) => {
                    // Custom delete hooks must observe live native objects.
                    mounted.run_delete_hooks();
                    mounted.count
                }
                None => 0,
            };
            self.counters.deleted += count as u64;
            // The overlay is deleted first, so every remaining bound keyboard
            // and textarea lives under the root.
            self.counters.keyboards.clear();
            // SAFETY: the root is live and no mounted Rust handle is used after
            // this point. LVGL deletes the root's children and fires their
            // delete callbacks.
            unsafe { rs_lvgl_sys::lv_obj_delete(root.as_ptr()) };
            // Native destructors may read retained descriptors and buffers.
            // Release the Rust tree only after those destructors finish.
            drop(mounted);
        } else {
            self.mounted = None;
        }
    }
}

impl<M: Clone + 'static> Drop for Ui<M> {
    fn drop(&mut self) {
        self.delete_overlay();
        self.delete_root();
    }
}

struct BusyGuard(*const Cell<bool>);

/// Create or update one mounted tree under `root`.
fn reconcile_tree<M: Clone + 'static>(
    root: NonNull<lv_obj_t>,
    slot: &mut Option<MountedNode<M>>,
    tree: View<M>,
    queue: &Rc<EventQueue<M>>,
    counters: &mut Counters,
) -> Result<(), UiError> {
    match slot.take() {
        None => {
            *slot = Some(reconciler::create(root, tree, queue, counters)?);
            Ok(())
        }
        Some(mut mounted) => match reconciler::update(&mut mounted, tree, queue, counters) {
            Ok(()) => {
                *slot = Some(mounted);
                Ok(())
            }
            Err(error) => {
                // The mounted tree may be partially updated. Run the custom
                // delete hooks while the native objects are alive, then delete
                // the tree so no Rust handle outlives a native object; the
                // next render recreates the tree from the application.
                reconciler::delete_subtree(mounted, counters);
                Err(error)
            }
        },
    }
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        // SAFETY: the guard is created and dropped inside one `&mut self`
        // method. `busy` is a field of the same `Ui`, which cannot move while
        // the method holds `&mut self`.
        unsafe { (*self.0).set(false) };
    }
}

fn enter(busy: &Cell<bool>) -> Result<BusyGuard, UiError> {
    if busy.replace(true) {
        return Err(UiError::Reentrant);
    }
    Ok(BusyGuard(busy as *const Cell<bool>))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_mutation_is_rejected() {
        let busy = Cell::new(false);
        let guard = enter(&busy).expect("first entry succeeds");
        assert_eq!(enter(&busy).err(), Some(UiError::Reentrant));
        drop(guard);
        assert!(enter(&busy).is_ok());
    }
}
