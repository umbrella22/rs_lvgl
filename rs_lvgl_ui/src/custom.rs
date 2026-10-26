//! Lifecycle contract for custom widgets.
//!
//! A custom widget owns one native LVGL subtree. The reconciler calls
//! [`CustomWidget::create`] once when the node is mounted, [`CustomWidget::update`]
//! once per [`crate::Ui::tick`], and [`CustomWidget::delete`] exactly once before
//! the native object is deleted.
//!
//! All three hooks run on the UI thread, inside the `Ui` method that owns the
//! LVGL access. They receive only their own [`CustomObject`] (or the parent
//! object during creation). A widget must not reconcile `View` children, mutate
//! sibling objects, or call back into [`crate::Ui`]; the `Ui` reentrancy guard
//! rejects such calls.
//!
//! The trait is deliberately not generic over the message type. `WidgetKind` is
//! not generic, so storing a `Box<dyn CustomWidget<M>>` inside it would require
//! making the whole widget descriptor generic. Application messages enter
//! through the node's explicit handler (`on_event`, `on_click`, and so on),
//! which the reconciler registers on the custom object like any typed widget.

use alloc::boxed::Box;
use core::cell::Cell;
use core::fmt;
use core::ptr::NonNull;

use rs_lvgl::{NativeObject, Obj, Widget};
use rs_lvgl_sys::lv_obj_t;

use crate::UiError;

/// A widget with its own native lifecycle.
///
/// Implementations keep their engine state in `self`; the mounted tree retains
/// the boxed instance until the node is removed. A later `App::view` call may
/// build another instance, but the reconciler keeps the mounted one unless the
/// node key or the custom variant changes.
pub trait CustomWidget: 'static {
    /// Create the native object as a direct child of `parent` and return an
    /// opaque handle to it.
    ///
    /// The object must stay alive until the mounted node is deleted. Return an
    /// error to abort the mount; any native objects created before the error are
    /// the implementation's responsibility.
    fn create(&mut self, parent: &mut Obj<'_>) -> Result<CustomObject, UiError>;

    /// Advance one tick and optionally invalidate the widget's native subtree.
    ///
    /// `elapsed_ms` is the exact value passed to [`crate::Ui::tick`]. Returning
    /// without calling [`CustomObject::invalidate`] keeps the previous frame.
    fn update(&mut self, object: &mut CustomObject, elapsed_ms: u32) -> Result<(), UiError>;

    /// Release widget state before the native object is deleted.
    ///
    /// The native object is still alive during this call. It is called exactly
    /// once per successful `create`.
    fn delete(&mut self, object: &mut CustomObject);
}

/// A borrowed view of the widget's native object.
///
/// The handle is created and owned by `rs_lvgl_ui`. It is only passed to the
/// widget's hooks as `&mut CustomObject`, so no lifetime tied to the mounted
/// native tree can escape a callback.
pub struct CustomObject {
    raw: NonNull<lv_obj_t>,
    invalidations: Cell<u64>,
}

impl CustomObject {
    /// Wrap a native object created as a direct child of `parent`.
    ///
    /// Returns [`UiError::InvalidProperty`] when `child` is not a direct child
    /// of `parent`.
    ///
    /// # Safety
    ///
    /// - `child` must point to a live LVGL object.
    /// - The returned handle must only be returned from
    ///   [`CustomWidget::create`]; the mounted tree then owns it and lends it
    ///   to the widget's hooks.
    /// - The caller must not create a second handle for the same object or
    ///   keep `child` and use it after the widget's
    ///   [`CustomWidget::delete`] hook returns. `rs_lvgl_ui` deletes the object
    ///   right after that hook, so any retained handle or pointer would
    ///   dangle.
    pub unsafe fn new(parent: &Obj<'_>, child: NonNull<lv_obj_t>) -> Result<Self, UiError> {
        // SAFETY: both pointers are live LVGL objects for the duration of the
        // call; `lv_obj_get_parent` only reads the parent link.
        let actual = unsafe { rs_lvgl_sys::lv_obj_get_parent(child.as_ptr()) };
        if actual != parent.raw().as_ptr() {
            return Err(UiError::InvalidProperty);
        }
        Ok(Self::from_raw_checked(child))
    }

    /// Construct a handle for an object whose parent linkage was already
    /// validated by `crate::raw`.
    pub(crate) fn from_raw_checked(raw: NonNull<lv_obj_t>) -> Self {
        Self {
            raw,
            invalidations: Cell::new(0),
        }
    }

    /// The native object as a borrowed LVGL handle.
    ///
    /// The returned `Obj` borrows this handle; it cannot be retained beyond the
    /// widget hook that received the `CustomObject`.
    pub fn obj(&self) -> Obj<'_> {
        // SAFETY: the mounted tree keeps the object alive for the whole hook
        // call that borrowed this `CustomObject`; `from_raw` does not take
        // ownership and `Obj` has no destructor.
        unsafe { Obj::from_raw(self.raw) }.expect("custom object is non-null")
    }

    /// The raw native pointer.
    ///
    /// Callers must not retain the pointer after the hook returns or delete the
    /// object themselves.
    pub fn raw(&self) -> NonNull<lv_obj_t> {
        self.raw
    }

    /// Mark the widget's native subtree as dirty and schedule a redraw.
    ///
    /// A no-op frame should skip this call; LVGL then keeps the previous frame.
    pub fn invalidate(&self) {
        self.invalidations.set(self.invalidations.get() + 1);
        // SAFETY: the object is live for the duration of the callback.
        unsafe { rs_lvgl_sys::lv_obj_invalidate(self.raw.as_ptr()) };
    }

    /// Number of [`CustomObject::invalidate`] calls made through this handle.
    pub fn invalidations(&self) -> u64 {
        self.invalidations.get()
    }
}

impl fmt::Debug for CustomObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomObject")
            .field("invalidations", &self.invalidations.get())
            .finish_non_exhaustive()
    }
}

/// The factory behind [`crate::raw`].
///
/// This is the static escape hatch: it has no update or delete hook and the
/// native subtree and its callbacks belong to the caller.
pub struct RawWidget {
    create: Box<dyn Fn(NonNull<lv_obj_t>) -> Option<NonNull<lv_obj_t>>>,
}

impl RawWidget {
    pub(crate) fn new(create: Box<dyn Fn(NonNull<lv_obj_t>) -> Option<NonNull<lv_obj_t>>>) -> Self {
        Self { create }
    }
}

/// Implementation behind [`crate::WidgetKind::Custom`].
///
/// The variant decides whether the reconciler treats the node as a static raw
/// subtree or as a lifecycle widget that [`crate::Ui::tick`] visits.
pub enum CustomWidgetKind {
    /// A one-shot opaque subtree created by `crate::raw`.
    Raw(RawWidget),
    /// A lifecycle widget created by `crate::custom`.
    Lifecycle(Box<dyn CustomWidget>),
}

impl CustomWidgetKind {
    pub(crate) fn raw(widget: RawWidget) -> Self {
        Self::Raw(widget)
    }

    pub(crate) fn lifecycle(widget: impl CustomWidget) -> Self {
        Self::Lifecycle(Box::new(widget))
    }

    pub(crate) fn is_lifecycle(&self) -> bool {
        matches!(self, Self::Lifecycle(_))
    }

    pub(crate) fn same_variant(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Raw(_), Self::Raw(_)) | (Self::Lifecycle(_), Self::Lifecycle(_))
        )
    }

    pub(crate) fn create(&mut self, parent: &mut Obj<'_>) -> Result<CustomObject, UiError> {
        match self {
            Self::Raw(raw) => {
                let object = (raw.create)(parent.raw()).ok_or(UiError::Allocation)?;
                Ok(CustomObject::from_raw_checked(object))
            }
            Self::Lifecycle(widget) => {
                let object = widget.create(parent)?;
                // SAFETY: the handle was just produced by the widget, which
                // promised in `CustomObject::new` that the object is live.
                let actual = unsafe { rs_lvgl_sys::lv_obj_get_parent(object.raw.as_ptr()) };
                if actual != parent.raw().as_ptr() {
                    return Err(UiError::InvalidProperty);
                }
                Ok(object)
            }
        }
    }

    pub(crate) fn update(
        &mut self,
        object: &mut CustomObject,
        elapsed_ms: u32,
    ) -> Result<(), UiError> {
        match self {
            Self::Raw(_) => Ok(()),
            Self::Lifecycle(widget) => widget.update(object, elapsed_ms),
        }
    }

    pub(crate) fn delete(&mut self, object: &mut CustomObject) {
        if let Self::Lifecycle(widget) = self {
            widget.delete(object);
        }
    }
}

impl<T: CustomWidget + ?Sized> CustomWidget for Box<T> {
    fn create(&mut self, parent: &mut Obj<'_>) -> Result<CustomObject, UiError> {
        (**self).create(parent)
    }

    fn update(&mut self, object: &mut CustomObject, elapsed_ms: u32) -> Result<(), UiError> {
        (**self).update(object, elapsed_ms)
    }

    fn delete(&mut self, object: &mut CustomObject) {
        (**self).delete(object)
    }
}
