//! Constructors for custom views.

use crate::custom::{CustomWidget, CustomWidgetKind, RawWidget};
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
    Node::new(WidgetKind::Custom(CustomWidgetKind::raw(RawWidget::new(
        Box::new(create),
    ))))
}

/// Create a custom widget with a native lifecycle.
///
/// The widget's [`CustomWidget::create`] runs when the node is mounted;
/// [`CustomWidget::update`] runs once per [`crate::Ui::tick`];
/// [`CustomWidget::delete`] runs exactly once before the native object is
/// deleted by a key change, a kind change, subtree removal or
/// [`crate::Ui::unmount`].
///
/// The mounted tree keeps the widget instance. A widget supplied by a later
/// `App::view` call replaces it only when the node key or the custom
/// variant changes; otherwise the new descriptor instance is dropped. Use a
/// key change to swap implementations, or keep shared application state in
/// an `Rc`/`RefCell` visible to both the widget and the application.
///
/// Messages from native callbacks are delivered through the node's explicit
/// handler (`on_event`, `on_click`, ...), which is registered on the custom
/// object.
pub fn custom<M>(widget: impl CustomWidget) -> View<M> {
    Node::new(WidgetKind::Custom(CustomWidgetKind::lifecycle(widget)))
}
