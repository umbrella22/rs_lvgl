mod common;
use common::*;
use rs_lvgl_ui::prelude::*;
use rs_lvgl_ui::{PostOutcome, QueueOverflow, QueuePolicy};
use std::{cell::RefCell, rc::Rc};

struct Messages(Vec<i32>);
impl App for Messages {
    type Msg = i32;
    fn update(&mut self, msg: i32) {
        self.0.push(msg);
    }
    fn view(&self, _: &Theme) -> View<i32> {
        button("Post").on_click(7)
    }
}

#[test]
fn explicit_queue_outcomes_and_native_policy() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::with_capacity(&display, theme(), 2).unwrap();
    let mut app = Messages(Vec::new());
    ui.mount(&app).unwrap();
    let reject = QueuePolicy::new(QueueOverflow::RejectNew);
    assert_eq!(ui.post_with(1, reject).unwrap(), PostOutcome::Enqueued);
    assert_eq!(ui.post_with(2, reject).unwrap(), PostOutcome::Enqueued);
    assert_eq!(ui.post_with(3, reject).unwrap(), PostOutcome::Rejected(3));
    assert_eq!(
        ui.post_with(4, QueuePolicy::default()).unwrap(),
        PostOutcome::Evicted(1)
    );
    let merge = QueuePolicy::new(QueueOverflow::RejectNew)
        .coalesce(|previous, next| (*previous == 4).then_some(*previous + *next));
    assert_eq!(ui.post_with(5, merge).unwrap(), PostOutcome::Merged);
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.0, [2, 9]);
    let observed = Rc::new(RefCell::new(Vec::new()));
    let sink = observed.clone();
    ui.set_event_queue_policy(reject);
    ui.set_event_queue_observer(Some(Rc::new(move |outcome| {
        sink.borrow_mut().push(outcome)
    })));
    for _ in 0..3 {
        // SAFETY: app_root is a live mounted button, and its handlers do not unwind.
        unsafe {
            rs_lvgl_sys::lv_obj_send_event(
                app_root().as_ptr(),
                rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED,
                core::ptr::null_mut(),
            )
        };
    }
    assert_eq!(
        &*observed.borrow(),
        &[
            PostOutcome::Enqueued,
            PostOutcome::Enqueued,
            PostOutcome::Rejected(7)
        ]
    );
    ui.drain(&mut app, 8).unwrap();
    assert_eq!(app.0, [2, 9, 7, 7]);
    let stats = ui.queue_stats();
    assert_eq!((stats.evicted, stats.rejected, stats.merged), (1, 2, 1));
    ui.unmount();
}

#[cfg(feature = "widget-textarea")]
#[test]
fn wrong_kind_modifier_survives_map_and_has_explicit_mode() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, theme()).unwrap();
    struct Invalid;
    impl App for Invalid {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            column([label("public").password(true).map(|_: ()| ())])
        }
    }
    let result = ui.mount(&Invalid);
    #[cfg(feature = "strict-properties")]
    assert_eq!(result, Err(UiError::InvalidProperty));
    #[cfg(not(feature = "strict-properties"))]
    assert_eq!(result, Ok(()));
    assert_eq!(
        ui.stats().mounted_nodes,
        if cfg!(feature = "strict-properties") {
            0
        } else {
            2
        }
    );
    ui.unmount();
}

#[cfg(all(feature = "diagnostics", feature = "widget-textarea"))]
#[test]
fn diagnostics_identify_nested_paths_and_balance_failures() {
    use rs_lvgl_ui::{DiagnosticEvent as E, DiagnosticStage as S, Property};
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, theme()).unwrap();
    let events = Rc::new(RefCell::new(Vec::new()));
    let captured = events.clone();
    ui.set_diagnostic_sink(Some(Rc::new(move |event| {
        captured.borrow_mut().push(event)
    })));
    struct Invalid;
    impl App for Invalid {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            column([
                column([label("one").key(Key::Named("same"))]).key(Key::Id(1)),
                column([label("never log this value")
                    .password(true)
                    .key(Key::Named("same"))])
                .key(Key::Id(2)),
            ])
        }
    }
    let _ = ui.mount(&Invalid);
    let mut stack = Vec::new();
    let mut found = 0;
    for event in events.borrow().iter() {
        match *event {
            E::Enter {
                key, child_index, ..
            } => stack.push((key, child_index)),
            E::Exit => {
                assert!(stack.pop().is_some());
            }
            E::Error {
                property: Property::Password,
                stage: S::Validation,
                ..
            }
            | E::IgnoredProperty(Property::Password) => {
                assert_eq!(
                    stack,
                    [(Key::Auto, 0), (Key::Id(2), 1), (Key::Named("same"), 0)]
                );
                found += 1;
            }
            _ => {}
        }
    }
    assert!(stack.is_empty());
    assert_eq!(found, 1);
    assert!(!format!("{:?}", events.borrow()).contains("never log"));
    events.borrow_mut().clear();
    struct Fails;
    impl CustomWidget for Fails {
        fn create(&mut self, _: &mut Obj<'_>) -> Result<rs_lvgl_ui::CustomObject, UiError> {
            Err(UiError::Allocation)
        }
        fn update(&mut self, _: &mut rs_lvgl_ui::CustomObject, _: u32) -> Result<(), UiError> {
            Ok(())
        }
        fn delete(&mut self, _: &mut rs_lvgl_ui::CustomObject) {
            panic!("failed create must not get delete")
        }
    }
    struct Failure;
    impl App for Failure {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            column([label("valid"), custom(Fails).key(Key::Named("failure"))])
        }
    }
    assert_eq!(ui.mount(&Failure), Err(UiError::Allocation));
    let mut depth = 0;
    let mut errors = 0;
    for event in events.borrow().iter() {
        match event {
            E::Enter { .. } => depth += 1,
            E::Exit => depth -= 1,
            E::Error {
                error: UiError::Allocation,
                stage: S::Create,
                ..
            } => {
                assert_eq!(depth, 2);
                errors += 1;
            }
            _ => {}
        }
    }
    assert_eq!((depth, errors), (0, 1));
    assert_eq!(ui.stats().mounted_nodes, 0);
    struct Recovered;
    impl App for Recovered {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            label("recovered")
        }
    }
    ui.mount(&Recovered).unwrap();
    ui.unmount();
}

#[test]
fn render_decisions_merge_and_preserve_dirty_overlay_and_first_mount() {
    use rs_lvgl_ui::RenderDecision;
    use std::cell::Cell;
    let _guard = lock();
    let display = init();
    #[derive(Clone)]
    enum Msg {
        Quiet,
        Change,
        Overlay,
    }
    struct Page {
        value: u32,
        overlay: bool,
        views: Cell<usize>,
    }
    impl App for Page {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) {
            match msg {
                Msg::Quiet => {}
                Msg::Change => self.value += 1,
                Msg::Overlay => self.overlay = !self.overlay,
            }
        }
        fn update_with_render(&mut self, msg: Msg) -> RenderDecision {
            let quiet = matches!(msg, Msg::Quiet);
            self.update(msg);
            if quiet {
                RenderDecision::Skip
            } else {
                RenderDecision::Reconcile
            }
        }
        fn view(&self, _: &Theme) -> View<Msg> {
            self.views.set(self.views.get() + 1);
            label(format!("{}", self.value))
        }
        fn overlay(&self, _: &Theme) -> Option<View<Msg>> {
            self.overlay.then(|| label("overlay"))
        }
    }
    let mut app = Page {
        value: 0,
        overlay: false,
        views: Cell::new(0),
    };
    let mut ui = Ui::new(&display, theme()).unwrap();
    ui.post(Msg::Quiet).unwrap();
    assert!(
        ui.drain(&mut app, 8).unwrap().reconciled,
        "first mount overrides skip"
    );
    let initial = app.views.get();
    ui.post(Msg::Quiet).unwrap();
    assert!(!ui.drain(&mut app, 8).unwrap().reconciled);
    assert_eq!(app.views.get(), initial);
    for msg in [Msg::Quiet, Msg::Change, Msg::Quiet] {
        ui.post(msg).unwrap();
    }
    assert!(ui.drain(&mut app, 8).unwrap().reconciled);
    assert_eq!(app.views.get(), initial + 1);
    assert_eq!(app.value, 1);
    ui.mark_dirty();
    assert!(ui.drain(&mut app, 8).unwrap().reconciled);
    ui.set_theme(Theme::light());
    assert!(ui.drain(&mut app, 8).unwrap().reconciled);
    ui.post(Msg::Overlay).unwrap();
    assert!(ui.drain(&mut app, 8).unwrap().reconciled);
    assert_eq!(ui.stats().mounted_nodes, 2);
    ui.unmount();
    ui.post(Msg::Quiet).unwrap();
    assert!(ui.drain(&mut app, 8).unwrap().reconciled);
    ui.unmount();
}

#[test]
fn stable_children_restore_externally_changed_native_order() {
    let _guard = lock();
    let display = init();
    struct Page;
    impl App for Page {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            column((0..3).map(|id| label_static("fixed").key(Key::Id(id))))
        }
    }
    let mut ui = Ui::new(&display, theme()).unwrap();
    ui.mount(&Page).unwrap();
    let nodes = [
        child(app_root(), 0),
        child(app_root(), 1),
        child(app_root(), 2),
    ];
    ui.update(&Page).unwrap();
    assert_eq!(ui.stats().moved_nodes, 0);
    // SAFETY: these live siblings belong to the mounted column.
    unsafe {
        rs_lvgl_sys::lv_obj_move_to_index(nodes[2].as_ptr(), 0);
    }
    ui.update(&Page).unwrap();
    assert_eq!(
        nodes,
        [
            child(app_root(), 0),
            child(app_root(), 1),
            child(app_root(), 2)
        ]
    );
    assert!(ui.stats().moved_nodes > 0);
    ui.unmount();
}

#[cfg(feature = "widget-textarea")]
#[test]
fn dedicated_modifiers_reject_wrong_kinds_and_accept_textarea() {
    let _guard = lock();
    let display = init();
    struct Page(u8);
    impl App for Page {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            match self.0 {
                0 => label("x").placeholder("hint"),
                1 => label("x").multiline(),
                _ => textarea("").placeholder("hint").password(true).multiline(),
            }
        }
    }
    let mut ui = Ui::new(&display, theme()).unwrap();
    for case in 0..2 {
        let result = ui.mount(&Page(case));
        assert_eq!(result.is_err(), cfg!(feature = "strict-properties"));
    }
    ui.mount(&Page(2)).unwrap();
    ui.unmount();
}

#[cfg(feature = "diagnostics")]
#[test]
fn reordered_diagnostic_path_uses_destination_child_index() {
    use rs_lvgl_ui::{DiagnosticEvent, DiagnosticStage};
    let _guard = lock();
    let display = init();
    struct Page(bool);
    impl App for Page {
        type Msg = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            let ids = if self.0 { [2, 1] } else { [1, 2] };
            column(ids.map(|id| label_static("fixed").key(Key::Id(id))))
        }
    }
    let paths = Rc::new(RefCell::new(Vec::new()));
    let captured = paths.clone();
    let mut ui = Ui::new(&display, theme()).unwrap();
    ui.mount(&Page(false)).unwrap();
    ui.set_diagnostic_sink(Some(Rc::new(move |event| {
        if let DiagnosticEvent::Enter {
            key,
            child_index,
            stage: DiagnosticStage::Update,
            ..
        } = event
        {
            captured.borrow_mut().push((key, child_index));
        }
    })));
    ui.update(&Page(true)).unwrap();
    assert_eq!(
        &*paths.borrow(),
        &[(Key::Auto, 0), (Key::Id(2), 0), (Key::Id(1), 1)]
    );
    ui.unmount();
}

#[cfg(all(feature = "diagnostics", unix))]
#[test]
fn diagnostic_sink_panic_cannot_unwind_through_reconciliation() {
    use std::os::unix::process::ExitStatusExt;
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "diagnostic_sink_panic_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .output()
        .unwrap();
    assert_eq!(
        output.status.signal(),
        Some(6),
        "sink panic must abort, not unwind"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("DIAGNOSTIC_SINK_PANIC_CONTROL"));
}

#[cfg(feature = "diagnostics")]
#[test]
#[ignore = "abort control invoked in a fresh subprocess"]
fn diagnostic_sink_panic_child() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, theme()).unwrap();
    ui.set_diagnostic_sink(Some(Rc::new(|_| panic!("DIAGNOSTIC_SINK_PANIC_CONTROL"))));
    ui.mount(&Messages(Vec::new())).unwrap();
}
