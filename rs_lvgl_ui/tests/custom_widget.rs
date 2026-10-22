mod common;

use core::cell::{Cell, RefCell};
use core::ptr::NonNull;
use std::rc::Rc;

use common::{init, lock, theme};
use rs_lvgl_ui::{
    App, CustomObject, CustomWidget, Key, NativeObject, Node, Obj, Theme, Ui, UiError,
};

/// Counters and shared state for one test's custom widgets.
#[derive(Clone, Default)]
struct Counters {
    create: Rc<Cell<u32>>,
    update: Rc<Cell<u32>>,
    delete: Rc<Cell<u32>>,
    last_elapsed: Rc<Cell<u32>>,
    invalidations: Rc<Cell<u64>>,
    created_ids: Rc<RefCell<Vec<u64>>>,
    deleted_ids: Rc<RefCell<Vec<u64>>>,
    reentrant: Option<Rc<ReentrantProbe>>,
}

impl Counters {
    fn creates(&self) -> u32 {
        self.create.get()
    }

    fn updates(&self) -> u32 {
        self.update.get()
    }

    fn deletes(&self) -> u32 {
        self.delete.get()
    }
}

struct TestWidget {
    id: u64,
    counters: Counters,
    invalidate: bool,
}

impl CustomWidget for TestWidget {
    fn create(&mut self, parent: &mut Obj<'_>) -> Result<CustomObject, UiError> {
        self.counters.create.set(self.counters.creates() + 1);
        self.counters.created_ids.borrow_mut().push(self.id);
        // SAFETY: `parent` is a live object for the duration of the call.
        let raw = unsafe { rs_lvgl_sys::lv_obj_create(parent.raw().as_ptr()) };
        let raw = NonNull::new(raw).ok_or(UiError::Allocation)?;
        // SAFETY: `raw` was just created as a live child of `parent`, and the
        // handle is returned from `create` without being retained.
        unsafe { CustomObject::new(parent, raw) }
    }

    fn update(&mut self, object: &mut CustomObject, elapsed_ms: u32) -> Result<(), UiError> {
        self.counters.update.set(self.counters.updates() + 1);
        self.counters.last_elapsed.set(elapsed_ms);
        if self.invalidate {
            object.invalidate();
        }
        self.counters.invalidations.set(object.invalidations());

        if let Some(probe) = &self.counters.reentrant {
            let ui = probe.ui.get();
            let app = probe.app.get();
            assert!(!ui.is_null() && !app.is_null(), "probe pointers not set");
            let mut results = probe.results.borrow_mut();
            // SAFETY: the test installs these pointers while the `Ui` and app
            // are alive. The busy guard rejects each call before it touches the
            // tree, which is exactly what this test observes.
            unsafe {
                results.push((*ui).tick(2).unwrap_err());
                results.push((*ui).drain(&mut *app, 1).unwrap_err());
                results.push((*ui).update(&*app).unwrap_err());
            }
        }
        Ok(())
    }

    fn delete(&mut self, object: &mut CustomObject) {
        // The native object is still alive and parented during the delete hook.
        // SAFETY: the handle is valid for the duration of the hook.
        let parent = unsafe { rs_lvgl_sys::lv_obj_get_parent(object.raw().as_ptr()) };
        assert!(!parent.is_null());
        self.counters.delete.set(self.counters.deletes() + 1);
        self.counters.deleted_ids.borrow_mut().push(self.id);
    }
}

struct ReentrantProbe {
    ui: Cell<*mut Ui<()>>,
    app: Cell<*mut TestApp>,
    results: RefCell<Vec<UiError>>,
}

struct TestApp {
    ids: Vec<u64>,
    counters: Counters,
    invalidate: bool,
}

impl App for TestApp {
    type Msg = ();

    fn update(&mut self, (): ()) {}

    fn view(&self, _: &Theme) -> Node<()> {
        Node::column().children(self.ids.iter().map(|id| {
            Node::custom(TestWidget {
                id: *id,
                counters: self.counters.clone(),
                invalidate: self.invalidate,
            })
            .key(Key::Id(*id))
        }))
    }
}

#[test]
fn mount_tick_update_tick_unmount_lifecycle() {
    let _guard = lock();
    let display = init();
    let counters = Counters::default();
    let mut ui = Ui::new(&display, theme()).unwrap();
    let app = TestApp {
        ids: vec![1],
        counters: counters.clone(),
        invalidate: true,
    };

    ui.mount(&app).unwrap();
    assert_eq!(counters.creates(), 1);
    assert_eq!(counters.updates(), 0);
    assert_eq!(counters.deletes(), 0);
    assert_eq!(ui.stats().mounted_nodes, 2, "column + custom widget");
    assert_eq!(ui.stats().created_nodes, 2);

    ui.tick(16).unwrap();
    assert_eq!(counters.updates(), 1);
    assert_eq!(counters.last_elapsed.get(), 16);
    assert_eq!(counters.invalidations.get(), 1);

    ui.update(&app).unwrap();
    assert_eq!(counters.creates(), 1, "reconcile must not recreate");
    assert_eq!(counters.updates(), 1, "reconcile must not tick");
    assert_eq!(counters.deletes(), 0);

    ui.tick(33).unwrap();
    assert_eq!(counters.updates(), 2);
    assert_eq!(counters.last_elapsed.get(), 33);
    assert_eq!(counters.invalidations.get(), 2);

    ui.unmount();
    assert_eq!(counters.deletes(), 1);
    assert_eq!(*counters.deleted_ids.borrow(), vec![1]);
    assert_eq!(ui.stats().mounted_nodes, 0);

    // Tick after unmount is a successful no-op.
    ui.tick(10).unwrap();
    assert_eq!(counters.updates(), 2);
}

#[test]
fn tick_passes_exact_elapsed_and_does_not_rebuild() {
    let _guard = lock();
    let display = init();
    let counters = Counters::default();
    let mut ui = Ui::new(&display, theme()).unwrap();
    let app = TestApp {
        ids: vec![1],
        counters: counters.clone(),
        invalidate: false,
    };

    ui.mount(&app).unwrap();
    let before = ui.stats();

    for elapsed in [7u32, 0, 1_000] {
        ui.tick(elapsed).unwrap();
        assert_eq!(counters.last_elapsed.get(), elapsed);
    }

    assert_eq!(counters.updates(), 3);
    assert_eq!(ui.stats(), before, "tick must not reconcile the tree");
    assert_eq!(ui.stats().created_nodes, 2);
    assert_eq!(ui.stats().mounted_nodes, 2);
}

#[test]
fn key_replacement_deletes_old_and_creates_new() {
    let _guard = lock();
    let display = init();
    let counters = Counters::default();
    let mut ui = Ui::new(&display, theme()).unwrap();
    let mut app = TestApp {
        ids: vec![1],
        counters: counters.clone(),
        invalidate: true,
    };

    ui.mount(&app).unwrap();
    assert_eq!(counters.creates(), 1);

    app.ids = vec![2];
    ui.update(&app).unwrap();
    assert_eq!(counters.creates(), 2);
    assert_eq!(counters.deletes(), 1);
    assert_eq!(*counters.created_ids.borrow(), vec![1, 2]);
    assert_eq!(*counters.deleted_ids.borrow(), vec![1]);
    assert_eq!(ui.stats().created_nodes, 3, "column + two custom roots");
    assert_eq!(ui.stats().deleted_nodes, 1);

    // Only the replacement widget exists now, so one tick updates once.
    ui.tick(5).unwrap();
    assert_eq!(counters.updates(), 1);
}

#[test]
fn kind_change_deletes_custom_widget_exactly_once() {
    struct SwitchApp {
        custom: bool,
        counters: Counters,
    }

    impl App for SwitchApp {
        type Msg = ();

        fn update(&mut self, (): ()) {}

        fn view(&self, _: &Theme) -> Node<()> {
            let child = if self.custom {
                Node::custom(TestWidget {
                    id: 1,
                    counters: self.counters.clone(),
                    invalidate: false,
                })
            } else {
                Node::label("plain")
            };
            Node::column().child(child.key(Key::Id(1)))
        }
    }

    let _guard = lock();
    let display = init();
    let counters = Counters::default();
    let mut ui = Ui::new(&display, theme()).unwrap();
    let mut app = SwitchApp {
        custom: true,
        counters: counters.clone(),
    };

    ui.mount(&app).unwrap();
    assert_eq!(counters.creates(), 1);

    app.custom = false;
    ui.update(&app).unwrap();
    assert_eq!(counters.deletes(), 1, "kind change must delete the widget");

    app.custom = true;
    ui.update(&app).unwrap();
    assert_eq!(counters.creates(), 2);
    assert_eq!(counters.deletes(), 1);

    ui.unmount();
    assert_eq!(counters.deletes(), 2);
}

#[test]
fn reentrant_calls_from_update_are_rejected() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, theme()).unwrap();

    let counters = Counters::default();
    let mut app = TestApp {
        ids: vec![1],
        counters: counters.clone(),
        invalidate: false,
    };
    let probe = Rc::new(ReentrantProbe {
        ui: Cell::new(core::ptr::null_mut()),
        app: Cell::new(core::ptr::null_mut()),
        results: RefCell::new(Vec::new()),
    });
    app.counters.reentrant = Some(Rc::clone(&probe));
    probe.app.set(&mut app as *mut TestApp);
    probe.ui.set(&mut ui as *mut Ui<()>);

    ui.mount(&app).unwrap();
    let before = ui.stats();

    // The widget's update calls tick, drain and update on the same Ui.
    ui.tick(16).unwrap();

    let results = probe.results.borrow();
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|error| *error == UiError::Reentrant));
    drop(results);

    assert_eq!(counters.updates(), 1, "only the outer tick advanced");
    assert_eq!(
        ui.stats(),
        before,
        "reentrant calls must not mutate the tree"
    );

    // The tree is still usable and the busy guard was released.
    ui.tick(16).unwrap();
    assert_eq!(counters.updates(), 2);

    ui.unmount();
    assert_eq!(counters.deletes(), 1);
}

#[test]
fn unmount_deletes_every_custom_object_exactly_once() {
    let _guard = lock();
    let display = init();
    let counters = Counters::default();
    let mut ui = Ui::new(&display, theme()).unwrap();
    let app = TestApp {
        ids: vec![10, 11, 12],
        counters: counters.clone(),
        invalidate: false,
    };

    ui.mount(&app).unwrap();
    assert_eq!(counters.creates(), 3);

    ui.unmount();
    assert_eq!(counters.deletes(), 3);
    let mut deleted = counters.deleted_ids.borrow().clone();
    deleted.sort_unstable();
    assert_eq!(deleted, vec![10, 11, 12]);

    // A second unmount and dropping the Ui must not run delete hooks again.
    ui.unmount();
    drop(ui);
    assert_eq!(counters.deletes(), 3);
}

#[test]
fn subtree_removal_deletes_only_the_removed_widgets() {
    let _guard = lock();
    let display = init();
    let counters = Counters::default();
    let mut ui = Ui::new(&display, theme()).unwrap();
    let mut app = TestApp {
        ids: vec![1, 2, 3],
        counters: counters.clone(),
        invalidate: false,
    };

    ui.mount(&app).unwrap();
    assert_eq!(counters.creates(), 3);

    app.ids = vec![2];
    ui.update(&app).unwrap();
    assert_eq!(counters.creates(), 3);
    assert_eq!(counters.deletes(), 2);
    let mut deleted = counters.deleted_ids.borrow().clone();
    deleted.sort_unstable();
    assert_eq!(deleted, vec![1, 3]);

    ui.tick(1).unwrap();
    assert_eq!(counters.updates(), 1, "only the surviving widget ticks");
}

#[test]
fn no_op_tick_still_calls_update_without_invalidating() {
    let _guard = lock();
    let display = init();
    let counters = Counters::default();
    let mut ui = Ui::new(&display, theme()).unwrap();
    let app = TestApp {
        ids: vec![1],
        counters: counters.clone(),
        invalidate: false,
    };

    ui.mount(&app).unwrap();
    let before = ui.stats();

    ui.tick(0).unwrap();
    ui.tick(16).unwrap();

    assert_eq!(counters.updates(), 2, "update runs even for a no-op frame");
    assert_eq!(
        counters.invalidations.get(),
        0,
        "the widget chose not to invalidate"
    );
    assert_eq!(ui.stats(), before);
}

#[test]
fn custom_node_explicit_handler_posts_messages() {
    struct ClickApp {
        clicks: u32,
    }

    impl App for ClickApp {
        type Msg = u32;

        fn update(&mut self, clicks: u32) {
            self.clicks = clicks;
        }

        fn view(&self, _: &Theme) -> Node<u32> {
            // Boxed trait-object form, as in the plan sketch.
            let widget: Box<dyn CustomWidget> = Box::new(TestWidget {
                id: 1,
                counters: Counters::default(),
                invalidate: false,
            });
            Node::column().child(Node::custom(widget).key(Key::Id(1)).on_click(9))
        }
    }

    let _guard = lock();
    let display = init();
    let mut ui: Ui<u32> = Ui::new(&display, theme()).unwrap();
    let mut app = ClickApp { clicks: 0 };

    ui.mount(&app).unwrap();
    let custom = common::child(common::app_root(), 0);
    // SAFETY: `custom` is the live mounted custom root and the click handler is
    // registered on it. `param` is unused for a click event.
    unsafe {
        rs_lvgl_sys::lv_obj_send_event(
            custom.as_ptr(),
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED,
            core::ptr::null_mut(),
        );
    }

    let stats = ui.drain(&mut app, 8).unwrap();
    assert_eq!(stats.processed, 1);
    assert_eq!(app.clicks, 9);
    ui.unmount();
}

#[test]
fn raw_subtree_is_not_a_lifecycle_widget() {
    struct RawApp {
        creates: Rc<Cell<u32>>,
    }

    impl App for RawApp {
        type Msg = ();

        fn update(&mut self, (): ()) {}

        fn view(&self, _: &Theme) -> Node<()> {
            let creates = Rc::clone(&self.creates);
            // SAFETY: the factory returns a live direct child of the parent;
            // LVGL deletes it with the mounted subtree.
            let raw: Node<()> = unsafe {
                Node::raw(move |parent: NonNull<rs_lvgl_sys::lv_obj_t>| {
                    creates.set(creates.get() + 1);
                    let obj = rs_lvgl_sys::lv_obj_create(parent.as_ptr());
                    NonNull::new(obj)
                })
            };
            Node::column().child(raw.key(Key::Named("raw")))
        }
    }

    let _guard = lock();
    let display = init();
    let creates = Rc::new(Cell::new(0));
    let mut ui = Ui::new(&display, theme()).unwrap();
    let app = RawApp {
        creates: Rc::clone(&creates),
    };

    ui.mount(&app).unwrap();
    assert_eq!(creates.get(), 1);
    assert_eq!(ui.stats().mounted_nodes, 2);

    let before = ui.stats();
    ui.tick(16).unwrap();
    assert_eq!(creates.get(), 1, "tick must not recreate or update raw");
    assert_eq!(ui.stats(), before);

    ui.unmount();
    assert_eq!(ui.stats().created_nodes, ui.stats().deleted_nodes);
}
