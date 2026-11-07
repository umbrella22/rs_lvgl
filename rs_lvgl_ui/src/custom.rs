//! Property-aware custom widgets and type-safe mounted dispatch.
//!
//! Validation is pure. Native creation and every property application run on
//! the UI thread, independently of tick. The mounted entry owns both its widget
//! and boxed properties until native deletion; failed updates retain old and
//! incoming properties through deletion callbacks.

use crate::UiError;
use alloc::boxed::Box;
use core::{
    any::{Any, TypeId},
    cell::Cell,
    fmt,
    ptr::NonNull,
};
use rs_lvgl::{NativeObject, Obj, Widget};
use rs_lvgl_sys::lv_obj_t;

/// One native subtree controlled by application-owned properties.
///
/// Hooks may mutate only their own subtree, must not delete its root, and must
/// not reenter Ui. Native callbacks enqueue actions and must not unwind across C.
/// The Rust widget and its current properties outlive native deletion callbacks.
pub trait CustomWidget: 'static {
    type Props: 'static;

    /// Pure preflight, before either mounted tree changes. Do not read or
    /// mutate LVGL, external state or a mounted instance here.
    fn validate(_props: &Self::Props) -> Result<(), UiError>
    where
        Self: Sized,
    {
        Ok(())
    }

    /// Create a live direct child. Any partial construction on Err must be
    /// deleted by this hook before returning; no delete hook follows Err.
    /// Initial props are boxed and remain stable through native deletion.
    fn create(
        &mut self,
        parent: &mut Obj<'_>,
        props: &Self::Props,
    ) -> Result<CustomObject, UiError>;

    /// Apply current properties on mount and on every compatible reconciliation,
    /// including equal values, to correct native divergence. No tick is implied.
    ///
    /// On success, native references to previous properties must be retired.
    /// On Err, old and incoming properties remain alive until native deletion;
    /// the affected tree is deleted and a later render can retry canonical state.
    fn apply_props(
        &mut self,
        object: &mut CustomObject,
        props: &Self::Props,
    ) -> Result<(), UiError>;

    /// Advance time without rebuilding or applying properties.
    fn tick(&mut self, _object: &mut CustomObject, _elapsed_ms: u32) -> Result<(), UiError> {
        Ok(())
    }

    /// Called exactly once for successful creation while the root is live.
    /// Retire external registrations here; do not delete the root. Rust backing
    /// remains owned until LVGL finishes deleting the subtree and callbacks.
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

struct RawWidget {
    create: Box<dyn Fn(NonNull<lv_obj_t>) -> Option<NonNull<lv_obj_t>>>,
}
impl RawWidget {
    pub(crate) fn new(create: Box<dyn Fn(NonNull<lv_obj_t>) -> Option<NonNull<lv_obj_t>>>) -> Self {
        Self { create }
    }
}

trait ErasedWidget {
    fn implementation_type(&self) -> TypeId;
    fn validate(&self) -> Result<(), UiError>;
    fn create(&mut self, parent: &mut Obj<'_>) -> Result<CustomObject, UiError>;
    fn apply_initial(&mut self, object: &mut CustomObject) -> Result<(), UiError>;
    fn into_props(self: Box<Self>) -> Box<dyn Any>;
    fn apply_from(&mut self, object: &mut CustomObject, props: Box<dyn Any>)
        -> Result<(), UiError>;
    fn tick(&mut self, object: &mut CustomObject, elapsed_ms: u32) -> Result<(), UiError>;
    fn delete(&mut self, object: &mut CustomObject);
}
struct Entry<W: CustomWidget> {
    widget: W,
    props: Box<W::Props>,
    incoming: Option<Box<W::Props>>,
}
impl<W: CustomWidget> ErasedWidget for Entry<W> {
    fn implementation_type(&self) -> TypeId {
        TypeId::of::<W>()
    }
    fn validate(&self) -> Result<(), UiError> {
        W::validate(&self.props)
    }
    fn create(&mut self, parent: &mut Obj<'_>) -> Result<CustomObject, UiError> {
        self.widget.create(parent, &self.props)
    }
    fn apply_initial(&mut self, object: &mut CustomObject) -> Result<(), UiError> {
        self.widget.apply_props(object, &self.props)
    }
    fn into_props(self: Box<Self>) -> Box<dyn Any> {
        self.props
    }
    fn apply_from(
        &mut self,
        object: &mut CustomObject,
        props: Box<dyn Any>,
    ) -> Result<(), UiError> {
        let props = props
            .downcast::<W::Props>()
            .map_err(|_| UiError::InvalidProperty)?;
        self.incoming = Some(props);
        let result = self.widget.apply_props(
            object,
            self.incoming.as_deref().ok_or(UiError::InvalidProperty)?,
        );
        if result.is_ok() {
            self.props = self.incoming.take().ok_or(UiError::InvalidProperty)?;
        }
        result
    }
    fn tick(&mut self, object: &mut CustomObject, elapsed_ms: u32) -> Result<(), UiError> {
        self.widget.tick(object, elapsed_ms)
    }
    fn delete(&mut self, object: &mut CustomObject) {
        self.widget.delete(object)
    }
}

enum CustomEntry {
    Raw(RawWidget),
    Lifecycle(Box<dyn ErasedWidget>),
}

/// Opaque descriptor payload created by `custom` or unsafe `raw`.
/// Concrete implementation identity is checked privately using TypeId/Any.
pub struct CustomWidgetKind {
    entry: CustomEntry,
}
impl CustomWidgetKind {
    pub(crate) fn raw(create: Box<dyn Fn(NonNull<lv_obj_t>) -> Option<NonNull<lv_obj_t>>>) -> Self {
        Self {
            entry: CustomEntry::Raw(RawWidget::new(create)),
        }
    }
    pub(crate) fn lifecycle<W: CustomWidget>(widget: W, props: W::Props) -> Self {
        Self {
            entry: CustomEntry::Lifecycle(Box::new(Entry {
                widget,
                props: Box::new(props),
                incoming: None,
            })),
        }
    }
    pub(crate) fn is_lifecycle(&self) -> bool {
        matches!(self.entry, CustomEntry::Lifecycle(_))
    }
    pub(crate) fn validate(&self) -> Result<(), UiError> {
        match &self.entry {
            CustomEntry::Raw(_) => Ok(()),
            CustomEntry::Lifecycle(w) => w.validate(),
        }
    }
    pub(crate) fn same_implementation(&self, other: &Self) -> bool {
        match (&self.entry, &other.entry) {
            (CustomEntry::Raw(_), CustomEntry::Raw(_)) => true,
            (CustomEntry::Lifecycle(a), CustomEntry::Lifecycle(b)) => {
                a.implementation_type() == b.implementation_type()
            }
            _ => false,
        }
    }
    pub(crate) fn create(&mut self, parent: &mut Obj<'_>) -> Result<CustomObject, UiError> {
        match &mut self.entry {
            CustomEntry::Raw(raw) => (raw.create)(parent.raw())
                .map(CustomObject::from_raw_checked)
                .ok_or(UiError::Allocation),
            CustomEntry::Lifecycle(widget) => {
                let object = widget.create(parent)?;
                // SAFETY: CustomObject::new establishes the live direct-child
                // obligation for successful hooks; this getter checks linkage.
                if unsafe { rs_lvgl_sys::lv_obj_get_parent(object.raw.as_ptr()) }
                    != parent.raw().as_ptr()
                {
                    return Err(UiError::InvalidProperty);
                }
                Ok(object)
            }
        }
    }
    pub(crate) fn apply_initial(&mut self, object: &mut CustomObject) -> Result<(), UiError> {
        match &mut self.entry {
            CustomEntry::Raw(_) => Ok(()),
            CustomEntry::Lifecycle(w) => w.apply_initial(object),
        }
    }
    pub(crate) fn apply_from(
        &mut self,
        object: &mut CustomObject,
        incoming: Self,
    ) -> Result<(), UiError> {
        if !self.same_implementation(&incoming) {
            return Err(UiError::InvalidProperty);
        }
        match (&mut self.entry, incoming.entry) {
            (CustomEntry::Raw(_), CustomEntry::Raw(_)) => Ok(()),
            (CustomEntry::Lifecycle(old), CustomEntry::Lifecycle(new)) => {
                old.apply_from(object, new.into_props())
            }
            _ => Err(UiError::InvalidProperty),
        }
    }
    pub(crate) fn tick(
        &mut self,
        object: &mut CustomObject,
        elapsed_ms: u32,
    ) -> Result<(), UiError> {
        match &mut self.entry {
            CustomEntry::Raw(_) => Ok(()),
            CustomEntry::Lifecycle(w) => w.tick(object, elapsed_ms),
        }
    }
    pub(crate) fn delete(&mut self, object: &mut CustomObject) {
        if let CustomEntry::Lifecycle(w) = &mut self.entry {
            w.delete(object)
        }
    }
}

impl<T: CustomWidget> CustomWidget for Box<T> {
    type Props = T::Props;
    fn validate(props: &Self::Props) -> Result<(), UiError> {
        T::validate(props)
    }
    fn create(
        &mut self,
        parent: &mut Obj<'_>,
        props: &Self::Props,
    ) -> Result<CustomObject, UiError> {
        (**self).create(parent, props)
    }
    fn apply_props(
        &mut self,
        object: &mut CustomObject,
        props: &Self::Props,
    ) -> Result<(), UiError> {
        (**self).apply_props(object, props)
    }
    fn tick(&mut self, object: &mut CustomObject, elapsed_ms: u32) -> Result<(), UiError> {
        (**self).tick(object, elapsed_ms)
    }
    fn delete(&mut self, object: &mut CustomObject) {
        (**self).delete(object)
    }
}
