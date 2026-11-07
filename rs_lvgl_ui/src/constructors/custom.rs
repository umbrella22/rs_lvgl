//! Constructors for custom views.

use crate::custom::{CustomWidget, CustomWidgetKind};
use crate::node::{Node, WidgetKind};
use crate::View;
use alloc::boxed::Box;
use core::ptr::NonNull;
use rs_lvgl_sys::lv_obj_t;

/// Create an opaque native subtree.
///
/// The reconciler calls `create` with a valid LVGL parent and never
/// traverses the returned object's children. Deletion happens when the
/// mounted node is removed or the UI is unmounted.
///
/// # Safety
///
/// The returned object must be a live direct child of `parent`, must remain
/// valid until LVGL deletes the mounted subtree, and must not be deleted by
/// the caller. The factory must not call back into [`crate::Ui`]. Any native
/// callbacks it registers are its own responsibility.
pub unsafe fn raw<M, F>(create: F) -> View<M>
where
    F: Fn(NonNull<lv_obj_t>) -> Option<NonNull<lv_obj_t>> + 'static,
{
    Node::new(WidgetKind::Custom(CustomWidgetKind::raw(Box::new(create))))
}

/// Create a property-aware native subtree. The mounted instance receives every
/// current property value, including equal values. Key or concrete implementation
/// changes replace it; unused incoming factories are dropped. Tick is separate.
/// Props and the widget outlive native deletion callbacks, including failed
/// property updates. See [`CustomWidget`] for hook and retention obligations.
pub fn custom<M, W: CustomWidget>(widget: W, props: W::Props) -> View<M> {
    Node::new(WidgetKind::Custom(CustomWidgetKind::lifecycle(
        widget, props,
    )))
}
