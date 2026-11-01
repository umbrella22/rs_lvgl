//! Root teardown and native-before-Rust resource release.

use super::guard::enter;
use super::Ui;
use crate::reconciler;

impl<M: Clone + 'static> Ui<M> {
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

    pub(super) fn delete_overlay(&mut self) {
        if let Some(overlay) = self.overlay.take() {
            reconciler::delete_subtree(overlay, &mut self.counters);
        }
        if let Some(root) = self.overlay_root.take() {
            // SAFETY: the overlay root is live and its mounted tree was
            // deleted above, so no Rust handle refers to its children.
            unsafe { rs_lvgl_sys::lv_obj_delete(root.as_ptr()) };
        }
    }

    pub(super) fn delete_root(&mut self) {
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
