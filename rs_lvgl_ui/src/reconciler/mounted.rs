//! Custom widget lifecycle, subtree counts and native deletion.

use super::{release_keyboards, Counters, MountedCustom, MountedNode};
use crate::UiError;
#[cfg(feature = "input-keypad")]
use core::ptr::NonNull;
#[cfg(feature = "input-keypad")]
use rs_lvgl_sys::lv_obj_t;

impl MountedCustom {
    pub(super) fn update(&mut self, elapsed_ms: u32) -> Result<(), UiError> {
        self.entry.update(&mut self.object, elapsed_ms)
    }

    pub(super) fn delete(mut self) {
        self.entry.delete(&mut self.object);
    }
}
impl<M: Clone + 'static> MountedNode<M> {
    #[cfg(feature = "input-keypad")]
    pub(crate) fn focus_object(&self) -> NonNull<lv_obj_t> {
        #[cfg(feature = "widget-ime-pinyin")]
        if let Some(input) = self.parts.input {
            return input;
        }
        self.obj
    }
    pub(super) fn recount(&mut self) {
        self.count = 1 + self.children.iter().map(|child| child.count).sum::<usize>();
        let own = self
            .custom
            .as_ref()
            .map(|custom| usize::from(custom.entry.is_lifecycle()))
            .unwrap_or(0);
        self.customs = own
            + self
                .children
                .iter()
                .map(|child| child.customs)
                .sum::<usize>();
    }

    /// Call every custom widget's delete hook exactly once, while the native
    /// objects are still alive.
    ///
    /// This must run before `lv_obj_delete` on any object that owns this
    /// subtree: LVGL's deletion would otherwise invalidate the handles passed
    /// to the hooks. Taking the `MountedCustom` out makes a second call a no-op.
    pub(crate) fn run_delete_hooks(&mut self) {
        if let Some(custom) = self.custom.take() {
            custom.delete();
        }
        for child in &mut self.children {
            child.run_delete_hooks();
        }
    }

    /// Advance every custom widget in this subtree once, pre-order.
    pub(crate) fn tick(&mut self, elapsed_ms: u32) -> Result<(), UiError> {
        if self.customs == 0 {
            return Ok(());
        }
        if let Some(custom) = &mut self.custom {
            custom.update(elapsed_ms)?;
        }
        for child in &mut self.children {
            child.tick(elapsed_ms)?;
        }
        Ok(())
    }
}

impl<M: Clone + 'static> Drop for MountedNode<M> {
    fn drop(&mut self) {
        // Safety net for early-return paths that drop a partially reconciled
        // subtree (for example an error in a later sibling). Native objects are
        // still alive whenever a `MountedCustom` reaches this point; every
        // explicit deletion path takes the hook first.
        if let Some(custom) = self.custom.take() {
            custom.delete();
        }
        // Children drop recursively through this same implementation.
    }
}
pub(crate) fn delete_subtree<M: Clone + 'static>(
    mut mounted: MountedNode<M>,
    counters: &mut Counters,
) {
    release_keyboards(&mounted, counters);
    // Custom delete hooks run before LVGL invalidates the native objects.
    mounted.run_delete_hooks();
    let count = mounted.count;
    // SAFETY: the object is live and owned by the mounted tree. Deleting it
    // also deletes its children and fires their delete callbacks exactly once.
    unsafe { rs_lvgl_sys::lv_obj_delete(mounted.obj.as_ptr()) };
    counters.deleted += count as u64;
}
