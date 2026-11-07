//! Independent native identity, property correction and retained-backing oracles.
mod common;
use core::{
    cell::{Cell, RefCell},
    ptr::NonNull,
};
use rs_lvgl_sys as sys;
use rs_lvgl_ui::prelude::*;
use std::{
    ffi::{CStr, CString},
    rc::Rc,
};

#[derive(Default)]
struct Probe {
    trace: RefCell<Vec<String>>,
    factories: Cell<u32>,
    drops: Cell<u32>,
    deletes: Cell<u32>,
    native_deletes: Cell<u32>,
    backing_valid: Cell<bool>,
}
struct Props {
    text: CString,
    alive: Rc<Cell<bool>>,
    invalid: bool,
    fail_apply: bool,
    fail_create: bool,
}
impl Drop for Props {
    fn drop(&mut self) {
        self.alive.set(false);
    }
}
struct Deletion {
    alive: [RefCell<Rc<Cell<bool>>>; 2],
    widget_alive: Rc<Cell<bool>>,
    probe: Rc<Probe>,
}
unsafe extern "C" fn deleted(event: *mut sys::lv_event_t) {
    // SAFETY: the test retains this stable allocation independently of the widget.
    // Alive flags guard the read, so an early widget/props drop is observed without dereferencing
    // a freed CString or unwinding across the C callback.
    unsafe {
        let context = &*sys::lv_event_get_user_data(event).cast::<Deletion>();
        let root = sys::lv_event_get_current_target(event).cast::<sys::lv_obj_t>();
        let alive = context.widget_alive.get() && context.alive.iter().all(|v| v.borrow().get());
        let valid = alive
            && (0..2).all(|i| {
                let label = sys::lv_obj_get_child(root, i);
                !label.is_null()
                    && !sys::lv_label_get_text(label).is_null()
                    && !CStr::from_ptr(sys::lv_label_get_text(label).cast())
                        .to_bytes()
                        .is_empty()
            });
        context
            .probe
            .backing_valid
            .set(context.probe.backing_valid.get() && valid);
        context
            .probe
            .native_deletes
            .set(context.probe.native_deletes.get() + 1);
    }
}
struct Gauge<const TYPE: u8> {
    probe: Rc<Probe>,
    deletion: Option<Rc<Deletion>>,
    contexts: Rc<RefCell<Vec<Rc<Deletion>>>>,
    alive: Rc<Cell<bool>>,
}
impl<const TYPE: u8> Drop for Gauge<TYPE> {
    fn drop(&mut self) {
        self.alive.set(false);
        self.probe.drops.set(self.probe.drops.get() + 1);
    }
}
impl<const TYPE: u8> CustomWidget for Gauge<TYPE> {
    type Props = Props;
    fn validate(props: &Props) -> Result<(), UiError> {
        if props.invalid {
            Err(UiError::InvalidProperty)
        } else {
            Ok(())
        }
    }
    fn create(&mut self, parent: &mut Obj<'_>, props: &Props) -> Result<CustomObject, UiError> {
        self.probe.trace.borrow_mut().push(format!("create-{TYPE}"));
        // SAFETY: serialized native test; each allocation is parented immediately.
        // The hook owns partial cleanup on Err before returning to the reconciler.
        unsafe {
            let root = NonNull::new(sys::lv_obj_create(parent.raw().as_ptr()))
                .ok_or(UiError::Allocation)?;
            for _ in 0..2 {
                let label = sys::lv_label_create(root.as_ptr());
                if label.is_null() {
                    sys::lv_obj_delete(root.as_ptr());
                    return Err(UiError::Allocation);
                }
                sys::lv_label_set_text_static(label, props.text.as_ptr().cast());
            }
            if props.fail_create {
                sys::lv_obj_delete(root.as_ptr());
                return Err(UiError::Allocation);
            }
            let context = Rc::new(Deletion {
                alive: [
                    RefCell::new(props.alive.clone()),
                    RefCell::new(props.alive.clone()),
                ],
                widget_alive: self.alive.clone(),
                probe: self.probe.clone(),
            });
            let callback = sys::lv_obj_add_event_cb(
                root.as_ptr(),
                Some(deleted),
                sys::lv_event_code_t_LV_EVENT_DELETE,
                (Rc::as_ptr(&context) as *mut Deletion).cast(),
            );
            if callback.is_null() {
                sys::lv_obj_delete(root.as_ptr());
                return Err(UiError::Allocation);
            }
            self.contexts.borrow_mut().push(context.clone());
            self.deletion = Some(context);
            CustomObject::new(parent, root)
        }
    }
    fn apply_props(&mut self, object: &mut CustomObject, props: &Props) -> Result<(), UiError> {
        self.probe
            .trace
            .borrow_mut()
            .push(format!("props-{}", props.text.to_str().unwrap()));
        let context = self.deletion.as_mut().unwrap();
        // SAFETY: live borrowed root has the two labels created by this widget.
        // On failure label 0 retains incoming backing while label 1 retains old backing.
        unsafe {
            sys::lv_label_set_text_static(
                sys::lv_obj_get_child(object.raw().as_ptr(), 0),
                props.text.as_ptr().cast(),
            );
        }
        context.alive[0].replace(props.alive.clone());
        if props.fail_apply {
            return Err(UiError::InvalidProperty);
        }
        unsafe {
            sys::lv_label_set_text_static(
                sys::lv_obj_get_child(object.raw().as_ptr(), 1),
                props.text.as_ptr().cast(),
            );
        }
        context.alive[1].replace(props.alive.clone());
        Ok(())
    }
    fn tick(&mut self, _: &mut CustomObject, elapsed: u32) -> Result<(), UiError> {
        self.probe
            .trace
            .borrow_mut()
            .push(format!("tick-{elapsed}"));
        Ok(())
    }
    fn delete(&mut self, object: &mut CustomObject) {
        // SAFETY: hook runs before the root's native deletion.
        assert!(!unsafe { sys::lv_obj_get_parent(object.raw().as_ptr()) }.is_null());
        self.probe.deletes.set(self.probe.deletes.get() + 1);
        self.probe.trace.borrow_mut().push(format!("delete-{TYPE}"));
    }
}
struct Page {
    probe: Rc<Probe>,
    text: &'static str,
    kind: bool,
    key: u64,
    invalid: bool,
    fail_apply: bool,
    fail_create: bool,
    fail_sibling: bool,
    overlay: bool,
    backing: RefCell<Vec<Rc<Cell<bool>>>>,
    contexts: Rc<RefCell<Vec<Rc<Deletion>>>>,
}
impl Page {
    fn new() -> Self {
        let probe = Rc::new(Probe::default());
        probe.backing_valid.set(true);
        Self {
            probe,
            text: "first",
            kind: false,
            key: 1,
            invalid: false,
            fail_apply: false,
            fail_create: false,
            fail_sibling: false,
            overlay: true,
            backing: RefCell::new(Vec::new()),
            contexts: Rc::default(),
        }
    }
    fn node<const T: u8>(&self) -> View<()> {
        let alive = Rc::new(Cell::new(true));
        self.backing.borrow_mut().push(alive.clone());
        self.probe.factories.set(self.probe.factories.get() + 1);
        custom(
            Gauge::<T> {
                probe: self.probe.clone(),
                deletion: None,
                contexts: self.contexts.clone(),
                alive: Rc::new(Cell::new(true)),
            },
            Props {
                text: CString::new(self.text).unwrap(),
                alive,
                invalid: self.invalid,
                fail_apply: self.fail_apply,
                fail_create: self.fail_create,
            },
        )
        .key(Key::Id(self.key))
    }
}
impl App for Page {
    type Action = ();
    fn update(&mut self, _: ()) {}
    fn view(&self, _: &Theme) -> View<()> {
        let sibling = if self.fail_sibling {
            // SAFETY: an explicitly failed raw allocation creates no native object.
            unsafe { raw(|_| None) }
        } else {
            label("sibling")
        };
        column([
            if self.kind {
                self.node::<2>()
            } else {
                self.node::<1>()
            },
            sibling.key(Key::Id(9)),
        ])
    }
    fn overlay(&self, _: &Theme) -> Option<View<()>> {
        self.overlay.then(|| label("overlay"))
    }
}
fn mounted() -> NonNull<sys::lv_obj_t> {
    common::child(common::app_root(), 0)
}
fn text(root: NonNull<sys::lv_obj_t>, index: i32) -> String {
    // SAFETY: tests only read currently mounted, copied-or-retained label data.
    unsafe {
        CStr::from_ptr(sys::lv_label_get_text(common::child(root, index).as_ptr()).cast())
            .to_str()
            .unwrap()
            .to_owned()
    }
}
fn assert_backing(app: &Page) {
    assert!(
        app.probe.backing_valid.get(),
        "widget and both props must outlive native deletion"
    );
    assert!(
        app.backing.borrow().iter().all(|v| !v.get()),
        "all consumed and unused properties released"
    );
    assert_eq!(app.probe.factories.get(), app.probe.drops.get());
    assert_eq!(app.probe.deletes.get(), app.probe.native_deletes.get());
}
#[test]
fn same_instance_receives_changed_and_equal_props_before_separate_tick() {
    let _lock = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Page::new();
    ui.mount(&app).unwrap();
    let root = mounted();
    assert_eq!(&*app.probe.trace.borrow(), &["create-1", "props-first"]);
    app.text = "second";
    ui.update(&app).unwrap();
    assert_eq!(mounted(), root);
    assert_eq!(text(root, 0), "second");
    // SAFETY: simulate native divergence while the mounted label is live.
    unsafe {
        sys::lv_label_set_text(common::child(root, 0).as_ptr(), c"drift".as_ptr().cast());
    }
    ui.update(&app).unwrap();
    assert_eq!(text(root, 0), "second");
    assert_eq!(
        app.probe.drops.get(),
        2,
        "unused incoming factories drop after property transfer"
    );
    assert!(!app
        .probe
        .trace
        .borrow()
        .iter()
        .any(|s| s.starts_with("tick")));
    ui.tick(17).unwrap();
    assert_eq!(app.probe.trace.borrow().last().unwrap(), "tick-17");
    ui.unmount();
    ui.unmount();
    assert_eq!(app.probe.deletes.get(), 1);
    assert_backing(&app);
}
#[test]
fn pure_rejection_preserves_main_and_overlay_trees() {
    let _lock = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Page::new();
    ui.mount(&app).unwrap();
    let root = mounted();
    let before = ui.stats();
    let trace = app.probe.trace.borrow().clone();
    // SAFETY: the display and its top-layer overlay are live under the test lock.
    let overlay = unsafe {
        let layer =
            sys::lv_display_get_layer_top(sys::lv_obj_get_display(common::screen().as_ptr()));
        sys::lv_obj_get_child(sys::lv_obj_get_child(layer, 0), 0)
    };
    assert!(!overlay.is_null());
    app.invalid = true;
    app.text = "rejected";
    app.overlay = false;
    assert_eq!(ui.update(&app), Err(UiError::InvalidProperty));
    assert_eq!(ui.stats(), before);
    assert_eq!(mounted(), root);
    assert_eq!(text(root, 0), "first");
    assert_eq!(*app.probe.trace.borrow(), trace);
    assert_eq!(
        unsafe {
            let layer =
                sys::lv_display_get_layer_top(sys::lv_obj_get_display(common::screen().as_ptr()));
            sys::lv_obj_get_child(sys::lv_obj_get_child(layer, 0), 0)
        },
        overlay,
        "preflight preserves overlay identity"
    );
    app.invalid = false;
    ui.update(&app).unwrap();
    assert_eq!(text(root, 0), "rejected");
    ui.unmount();
    assert_backing(&app);
}
#[test]
fn concrete_type_and_key_changes_replace_even_with_same_property_type() {
    let _lock = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Page::new();
    ui.mount(&app).unwrap();
    app.kind = true;
    ui.update(&app).unwrap();
    assert_eq!(app.probe.deletes.get(), 1);
    app.key = 2;
    ui.update(&app).unwrap();
    assert_eq!(app.probe.deletes.get(), 2);
    assert_eq!(
        app.probe
            .trace
            .borrow()
            .iter()
            .filter(|s| s.starts_with("create"))
            .count(),
        3
    );
    ui.unmount();
    assert_backing(&app);
}
#[test]
fn partial_property_error_retains_old_and_incoming_backing_then_retry_remounts() {
    let _lock = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Page::new();
    ui.mount(&app).unwrap();
    app.text = "incoming";
    app.fail_apply = true;
    assert_eq!(ui.update(&app), Err(UiError::InvalidProperty));
    assert_eq!(
        ui.stats().mounted_nodes,
        1,
        "unaffected overlay remains mounted"
    );
    assert_backing(&app);
    app.fail_apply = false;
    ui.update(&app).unwrap();
    assert_eq!(text(mounted(), 0), "incoming");
    ui.unmount();
    assert_backing(&app);
}
#[test]
fn initial_apply_failure_still_deletes_live_object_once() {
    let _lock = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Page::new();
    app.fail_apply = true;
    assert_eq!(ui.mount(&app), Err(UiError::InvalidProperty));
    assert_eq!(app.probe.deletes.get(), 1);
    assert_backing(&app);
    app.fail_apply = false;
    ui.mount(&app).unwrap();
    ui.unmount();
    assert_backing(&app);
}
#[test]
fn later_sibling_failure_keeps_custom_backing_through_native_delete() {
    let _lock = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Page::new();
    ui.mount(&app).unwrap();
    app.text = "updated-before-sibling";
    app.fail_sibling = true;
    assert_eq!(ui.update(&app), Err(UiError::Allocation));
    assert_backing(&app);
    app.fail_sibling = false;
    ui.update(&app).unwrap();
    ui.unmount();
    assert_backing(&app);
}
#[test]
fn failed_create_cleans_its_partial_subtree_without_delete_hook() {
    let _lock = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Page::new();
    app.fail_create = true;
    assert_eq!(ui.mount(&app), Err(UiError::Allocation));
    assert_eq!(app.probe.deletes.get(), 0);
    assert_backing(&app);
    app.fail_create = false;
    ui.mount(&app).unwrap();
    ui.unmount();
    assert_backing(&app);
}

#[test]
fn property_and_delete_hooks_reject_reentry_and_remount_cleanly() {
    struct Reentry {
        ui: Cell<*mut Ui<()>>,
        errors: RefCell<Vec<UiError>>,
    }
    impl Reentry {
        fn attempt(&self) {
            // SAFETY: the serialized test keeps the Ui live. Its busy guard
            // rejects reentry before any tree or native object is accessed.
            self.errors
                .borrow_mut()
                .push(unsafe { (*self.ui.get()).tick(1) }.unwrap_err());
        }
    }
    struct ReentrantWidget(Rc<Reentry>);
    impl CustomWidget for ReentrantWidget {
        type Props = Rc<Reentry>;
        fn create(
            &mut self,
            parent: &mut Obj<'_>,
            _: &Self::Props,
        ) -> Result<CustomObject, UiError> {
            // SAFETY: return the newly created direct child to the mounted owner.
            unsafe {
                CustomObject::new(
                    parent,
                    NonNull::new(sys::lv_obj_create(parent.raw().as_ptr()))
                        .ok_or(UiError::Allocation)?,
                )
            }
        }
        fn apply_props(
            &mut self,
            _: &mut CustomObject,
            props: &Self::Props,
        ) -> Result<(), UiError> {
            props.attempt();
            Ok(())
        }
        fn delete(&mut self, _: &mut CustomObject) {
            self.0.attempt();
        }
    }
    struct ReentrantPage(Rc<Reentry>);
    impl App for ReentrantPage {
        type Action = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            custom(ReentrantWidget(self.0.clone()), self.0.clone())
        }
    }
    let _lock = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let probe = Rc::new(Reentry {
        ui: Cell::new(&mut ui),
        errors: RefCell::new(Vec::new()),
    });
    let app = ReentrantPage(probe.clone());
    for _ in 0..2 {
        ui.mount(&app).unwrap();
        ui.update(&app).unwrap();
        ui.unmount();
    }
    assert_eq!(&*probe.errors.borrow(), &[UiError::Reentrant; 6]);
}
