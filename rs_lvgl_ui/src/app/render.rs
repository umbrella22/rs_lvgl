//! Descriptor validation and reconciliation of view/overlay trees.

use super::Ui;
use crate::events::EventQueue;
use crate::node;
use crate::reconciler::{self, Counters, MountedNode};
use crate::widgets::{self, KindTag};
use crate::{Size, Style, UiError, View};
use alloc::rc::Rc;
use core::ptr::NonNull;
use rs_lvgl_sys::lv_obj_t;

impl<M: Clone + 'static> Ui<M> {
    pub(super) fn ensure_root(&mut self) -> Result<(), UiError> {
        if self.root.is_none() {
            // SAFETY: `screen` belongs to the live display supplied by the
            // platform and remains valid under the lifecycle contract.
            let root = unsafe { rs_lvgl_sys::lv_obj_create(self.screen.as_ptr()) };
            let Some(root) = NonNull::new(root) else {
                return Err(UiError::Allocation);
            };
            // The theme's card style would otherwise give the root a
            // background, border and text color that `Style::plain` does
            // not reset.
            widgets::remove_main_theme(root);
            let style = Style::plain()
                .width(Size::Percent(100))
                .height(Size::Percent(100));
            widgets::apply_style(root, &style, KindTag::Container);
            self.root = Some(root);
        }
        Ok(())
    }

    pub(super) fn render(
        &mut self,
        tree: View<M>,
        overlay: Option<View<M>>,
    ) -> Result<(), UiError> {
        use crate::diagnostics::{DiagnosticEvent, DiagnosticTree};
        self.queue
            .diagnostics
            .emit(DiagnosticEvent::Tree(DiagnosticTree::View));
        node::validate(&tree, &self.queue.diagnostics)?;
        if let Some(overlay) = &overlay {
            self.queue
                .diagnostics
                .emit(DiagnosticEvent::Tree(DiagnosticTree::Overlay));
            node::validate(overlay, &self.queue.diagnostics)?;
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
        queue
            .diagnostics
            .emit(DiagnosticEvent::Tree(DiagnosticTree::View));
        reconcile_tree(root, &mut self.mounted, tree, &queue, &mut self.counters)?;

        match overlay {
            Some(overlay) => {
                queue
                    .diagnostics
                    .emit(DiagnosticEvent::Tree(DiagnosticTree::Overlay));
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

    pub(super) fn ensure_overlay_root(&mut self) -> Result<NonNull<lv_obj_t>, UiError> {
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
        let style = Style::plain()
            .width(Size::Percent(100))
            .height(Size::Percent(100));
        widgets::apply_style(root, &style, KindTag::Container);
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
}
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
            *slot = Some(reconciler::create(root, tree, queue, counters, 0)?);
            Ok(())
        }
        Some(mut mounted) => match reconciler::update(&mut mounted, tree, queue, counters, 0) {
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
