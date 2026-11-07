//! Frozen original-API workloads. Run via scripts/check-ui-workloads.py.
//! Correctness checks and model preparation are outside the measured phases.
mod common;
#[cfg(debug_assertions)]
#[path = "workloads/meter.rs"]
mod meter;
#[cfg(not(debug_assertions))]
mod meter {
    pub fn begin() -> usize {
        0
    }
    pub fn end(_: usize) -> (usize, usize, usize, isize) {
        (0, 0, 0, 0)
    }
}

use rs_lvgl_sys as sys;
use rs_lvgl_ui::prelude::*;
use std::rc::Rc;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    ffi::CStr,
    ptr::NonNull,
    time::Instant,
};

const WARMUP: usize = 4;
const BATCH: usize = 16;
const WIDTH: u32 = 240;
const HEIGHT: u32 = 160;

#[derive(Clone)]
enum Action {
    Numeric(i32),
    Toggle(bool),
    Text(String),
}
enum Model {
    Settings {
        value: i32,
        enabled: bool,
        text: String,
        overlay: bool,
    },
    List {
        items: Vec<(u64, i32)>,
    },
    Chart {
        series: usize,
        capacity: usize,
        batch: usize,
        sequence: i32,
        history: Vec<i32>,
    },
}
struct Page {
    model: Model,
    construction_ns: Cell<u128>,
    view_calls: Cell<usize>,
    actions: usize,
    clocks: bool,
}
impl Page {
    fn measure<T>(&self, f: impl FnOnce() -> T) -> T {
        if self.clocks {
            let start = Instant::now();
            let value = f();
            self.construction_ns
                .set(self.construction_ns.get() + start.elapsed().as_nanos());
            value
        } else {
            f()
        }
    }
}
impl App for Page {
    type Action = Action;
    fn update(&mut self, action: Action) {
        self.actions += 1;
        // The numeric field deliberately rejects native input. Other typed
        // bindings remain exercised without borrowing application state.
        match (&mut self.model, action) {
            (Model::Settings { enabled, .. }, Action::Toggle(value)) => *enabled = value,
            (Model::Settings { text, .. }, Action::Text(value)) => *text = value,
            (_, Action::Numeric(value)) => {
                std::hint::black_box(value);
            }
            _ => (),
        }
    }
    fn view(&self, theme: &Theme) -> View<Action> {
        self.view_calls.set(self.view_calls.get() + 1);
        self.measure(|| match &self.model {
            Model::Settings {
                value,
                enabled,
                text,
                ..
            } => typed::column([])
                .child(
                    typed::slider(0, 100, binding(*value, Action::Numeric))
                        .width(px(180))
                        .height(px(12))
                        .key(Key::Id(1)),
                )
                .child(typed::switch(binding(*enabled, Action::Toggle)).key(Key::Id(2)))
                .child(
                    typed::textarea(binding(text.as_str(), |v: &str| Action::Text(v.into())))
                        .width(px(180))
                        .height(px(36))
                        .key(Key::Id(3)),
                )
                .style(Style::screen(theme))
                .padding(12)
                .gap(8)
                .width(px(WIDTH as i16))
                .height(px(HEIGHT as i16))
                .into_view(),
            Model::List { items } => column(items.iter().map(|(key, value)| {
                label(format!("Item {key}: {value}"))
                    .key(Key::Id(*key))
                    .height(px(20))
                    .focusable(true)
            }))
            .width(px(WIDTH as i16))
            .height(px(HEIGHT as i16))
            .padding(4)
            .gap(0)
            .scrollable(true)
            // Automatic scrollbar thumb rounding depends on native scroll
            // history; this workload fixes visibility while testing viewport
            // and focused-object preservation separately.
            .scrollbar_mode(ScrollbarMode::Off)
            .scroll_momentum(false)
            .scroll_elastic(false),
            Model::Chart {
                series, history, ..
            } => chart(ChartSpec::new(ChartType::Line, ChartAxes::new(ChartRange::new(0,100),ChartRange::new(0,100)), (0..*series)
                    .map(|index| {
                        ChartSeries::new(index as u64, Color::rgb(40 + index as u8 * 40, 150, 200), ChartData::values(history
                                .iter()
                                .map(|value| (value + index as i32 * 7) % 100)
                                .collect::<Vec<_>>()))
                    })
                    .collect()))
            .width(px(WIDTH as i16))
            .height(px(HEIGHT as i16))
            .key(Key::Id(1)),
        })
    }
    fn overlay(&self, theme: &Theme) -> Option<View<Action>> {
        self.measure(|| match &self.model {
            Model::Settings { overlay: true, .. } => Some(
                container()
                    .child(label("Overlay"))
                    .width(px(90))
                    .height(px(50))
                    .background(theme.surface),
            ),
            _ => None,
        })
    }
}

fn native_memory() -> sys::lv_mem_monitor_t {
    let mut monitor = sys::lv_mem_monitor_t::default();
    // SAFETY: the single test thread owns initialized LVGL and its BUILTIN pool.
    unsafe { sys::lv_mem_monitor(&mut monitor) };
    monitor
}
fn native_text(obj: NonNull<sys::lv_obj_t>) -> String {
    // SAFETY: all callers pass a live label and copy its text synchronously.
    unsafe { CStr::from_ptr(sys::lv_label_get_text(obj.as_ptr())) }
        .to_str()
        .unwrap()
        .into()
}
fn identities(page: &Page) -> BTreeMap<u64, NonNull<sys::lv_obj_t>> {
    match &page.model {
        Model::List { items } => items
            .iter()
            .enumerate()
            .map(|(i, (key, _))| (*key, common::child(common::app_root(), i as i32)))
            .collect(),
        _ => BTreeMap::new(),
    }
}
fn verify(page: &Page, old: &BTreeMap<u64, NonNull<sys::lv_obj_t>>, corruption: bool) {
    // SAFETY: Ui owns the live tree throughout synchronous oracle checks. No
    // native array/reference escapes this function or survives reconciliation.
    unsafe {
        match &page.model {
            Model::Settings {
                value,
                text,
                overlay,
                ..
            } => {
                let root = common::app_root();
                let expected = *value + i32::from(corruption);
                assert_eq!(
                    sys::lv_slider_get_value(common::child(root, 0).as_ptr()),
                    expected,
                    "WORKLOAD_ORACLE_SETTINGS"
                );
                assert_eq!(
                    CStr::from_ptr(sys::lv_textarea_get_text(common::child(root, 2).as_ptr()))
                        .to_str()
                        .unwrap(),
                    text,
                    "WORKLOAD_ORACLE_SETTINGS_TEXT"
                );
                assert_eq!(
                    sys::lv_obj_get_child_count(sys::lv_layer_top()) > 0,
                    *overlay,
                    "WORKLOAD_ORACLE_OVERLAY"
                );
            }
            Model::List { items } => {
                let root = common::app_root();
                assert_eq!(
                    sys::lv_obj_get_child_count(root.as_ptr()) as usize,
                    items.len()
                );
                for (i, (id, value)) in items.iter().enumerate() {
                    let obj = common::child(root, i as i32);
                    assert_eq!(
                        native_text(obj),
                        format!("Item {id}: {}", value + i32::from(corruption)),
                        "WORKLOAD_ORACLE_LIST"
                    );
                    if let Some(previous) = old.get(id) {
                        assert_eq!(obj, *previous, "WORKLOAD_ORACLE_IDENTITY");
                    }
                    if *id == 1 {
                        assert!(
                            sys::lv_obj_has_state(obj.as_ptr(), sys::lv_state_t_LV_STATE_FOCUSED),
                            "WORKLOAD_ORACLE_FOCUS"
                        );
                    }
                }
            }
            Model::Chart {
                series, history, ..
            } => {
                let chart = common::app_root().as_ptr();
                assert_eq!(sys::lv_chart_get_point_count(chart) as usize, history.len());
                let mut pointer = sys::lv_chart_get_series_next(chart, std::ptr::null());
                for index in 0..*series {
                    assert!(!pointer.is_null(), "WORKLOAD_ORACLE_CHART_SERIES");
                    let observed = std::slice::from_raw_parts(
                        sys::lv_chart_get_series_y_array(chart, pointer),
                        history.len(),
                    );
                    // Compute from canonical scalar history independently of
                    // descriptor construction and native write order.
                    for (slot, value) in observed.iter().zip(history) {
                        assert_eq!(
                            *slot,
                            (value + index as i32 * 7) % 100 + i32::from(corruption),
                            "WORKLOAD_ORACLE_CHART"
                        );
                    }
                    pointer = sys::lv_chart_get_series_next(chart, pointer);
                }
                assert!(pointer.is_null());
            }
        }
    }
}

fn prepare(page: &mut Page, scenario: &str, iteration: usize, ui: &mut Ui<Action>) {
    match &mut page.model {
        Model::Settings { value, overlay, .. } => match scenario {
            "numeric" => *value = if *value == 30 { 70 } else { 30 },
            "theme" => ui.set_theme(if iteration % 2 == 0 {
                Theme::light()
            } else {
                Theme::dark()
            }),
            "overlay" => *overlay = !*overlay,
            _ => (),
        },
        Model::List { items } => match scenario {
            "label" => items.iter_mut().find(|(id, _)| *id == 1).unwrap().1 ^= 1,
            "prepend-remove" => {
                if items.first().unwrap().0 == 9999 {
                    items.remove(0);
                } else {
                    items.insert(0, (9999, 0));
                }
            }
            "rotate" => items.rotate_left(1),
            "reverse" => items.reverse(),
            "scroll-focus" => (),
            _ => unreachable!(),
        },
        Model::Chart {
            capacity,
            batch,
            sequence,
            history,
            ..
        } => {
            let steps = if scenario == "skipped" {
                *batch * 3
            } else {
                *batch
            };
            for _ in 0..steps {
                *sequence += 1;
                history.push(*sequence % 100);
                if history.len() > *capacity {
                    history.remove(0);
                }
            }
            if scenario == "replacement" {
                history.iter_mut().for_each(|v| *v = (*v + 31) % 100);
            }
        }
    }
    if scenario == "rejected" {
        let slider = common::child(common::app_root(), 0).as_ptr();
        // SAFETY: known live slider. This emulates an LVGL input event, not OS input.
        unsafe {
            sys::lv_slider_set_value(slider, 99, false);
            sys::lv_obj_send_event(
                slider,
                sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
                std::ptr::null_mut(),
            );
        }
    }
}
fn operate(page: &mut Page, scenario: &str, ui: &mut Ui<Action>) {
    match scenario {
        "idle" | "rejected" | "theme" => {
            ui.drain(page, 8).unwrap();
        }
        "remount" => {
            ui.unmount();
            ui.mount(page).unwrap();
        }
        _ => ui.update(page).unwrap(),
    }
}
fn refresh() {
    // SAFETY: synchronous memory rendering; no DMA/GPU worker or pending flush.
    unsafe { sys::lv_refr_now(sys::lv_obj_get_display(common::screen().as_ptr())) };
}
fn pixel_hash(pixels: &[u8]) -> u64 {
    assert!(
        pixels.windows(4).any(|w| w[0] != w[1]),
        "WORKLOAD_ORACLE_PIXELS"
    );
    pixels.iter().fold(0xcbf29ce484222325u64, |hash, b| {
        (hash ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}

fn run_case(model: Model, scenarios: &[&str], clocks: bool, corrupt: &str) {
    let frame = Rc::new(RefCell::new(vec![
        0u8;
        WIDTH as usize * HEIGHT as usize * 3
    ]));
    let flush_frame = frame.clone();
    let display = rs_lvgl::Display::register::<_, { 240 * 8 }>(WIDTH, HEIGHT, move |refresh| {
        // RGB888 rows are copied synchronously; native refresh areas lie inside
        // this fixed display. No application/UI mutation occurs in the callback.
        let mut pixels = flush_frame.borrow_mut();
        let mut index = 0;
        for y in refresh.area.y1..=refresh.area.y2 {
            for x in refresh.area.x1..=refresh.area.x2 {
                let offset = (y as usize * WIDTH as usize + x as usize) * 3;
                let color = refresh.colors[index];
                pixels[offset..offset + 3].copy_from_slice(&[color.r(), color.g(), color.b()]);
                index += 1;
            }
        }
    })
    .unwrap();
    let mut page = Page {
        model,
        construction_ns: Cell::new(0),
        view_calls: Cell::new(0),
        actions: 0,
        clocks,
    };
    let (family, size, series_count, append_batch) = match &page.model {
        Model::Settings { .. } => ("settings", 0, 0, 0),
        Model::List { items } => ("list", items.len(), 0, 0),
        Model::Chart {
            capacity,
            series,
            batch,
            ..
        } => ("chart", *capacity, *series, *batch),
    };
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    ui.mount(&page).unwrap();
    if family == "list" {
        ui.enable_focus().unwrap();
        assert!(ui.focus(Key::Id(1)).unwrap());
    }
    refresh();
    for scenario in scenarios {
        for index in 0..WARMUP {
            let ids = identities(&page);
            prepare(&mut page, scenario, index, &mut ui);
            operate(&mut page, scenario, &mut ui);
            verify(&page, &ids, false);
            refresh();
        }
        let native_before = native_memory();
        let counters = ui.stats();
        let labels = rs_lvgl_ui::update_metrics().native_label_text_sets;
        let calls_before = page.view_calls.get();
        let actions_before = page.actions;
        let mut total_ns = 0;
        let mut construction_ns = 0;
        let mut draw_ns = 0;
        let mut alloc_calls = 0;
        let mut alloc_bytes = 0;
        let mut peak = 0;
        let mut live_delta = 0;
        for index in 0..BATCH {
            let ids = identities(&page);
            prepare(&mut page, scenario, index, &mut ui);
            if *scenario == "scroll-focus" {
                // SAFETY: the live list root has overflowing content; explicit
                // no-animation scrolling is model preparation, outside timing.
                unsafe {
                    sys::lv_obj_scroll_to_y(
                        common::app_root().as_ptr(),
                        20 + index as i32 % 2 * 20,
                        false,
                    )
                };
            }
            let scroll_before = if family == "list" {
                // SAFETY: live list root, synchronous scalar getter.
                unsafe { sys::lv_obj_get_scroll_y(common::app_root().as_ptr()) }
            } else {
                0
            };
            let view_before = page.construction_ns.get();
            let before = meter::begin();
            let start = Instant::now();
            operate(&mut page, scenario, &mut ui);
            total_ns += start.elapsed().as_nanos();
            let sample = meter::end(before);
            alloc_calls += sample.0;
            alloc_bytes += sample.1;
            peak = peak.max(sample.2);
            live_delta += sample.3;
            construction_ns += page.construction_ns.get() - view_before;
            verify(&page, &ids, corrupt == family);
            if family == "list" && matches!(*scenario, "label" | "scroll-focus") {
                // SAFETY: compatible list root survives update.
                assert_eq!(
                    unsafe { sys::lv_obj_get_scroll_y(common::app_root().as_ptr()) },
                    scroll_before,
                    "WORKLOAD_ORACLE_SCROLL"
                );
            }
            if *scenario == "idle" {
                assert_eq!(page.view_calls.get(), calls_before, "WORKLOAD_IDLE_VIEW");
            }
            let start = Instant::now();
            refresh();
            draw_ns += start.elapsed().as_nanos();
        }
        let native_after = native_memory();
        let stats = ui.stats();
        let label_sets = rs_lvgl_ui::update_metrics().native_label_text_sets - labels;
        let view_calls = page.view_calls.get() - calls_before;
        let actions = page.actions - actions_before;
        let pixels = pixel_hash(&frame.borrow());
        // Compare the actual flushed screen (including overlays) with a fresh
        // canonical mount. This oracle does not reuse reconciliation results.
        let mut expected_pixels = frame.borrow().clone();
        if corrupt == "pixels" {
            expected_pixels[0] ^= 1;
        }
        let scroll_y = if family == "list" {
            // SAFETY: live list root; save its viewport for the fresh mount.
            unsafe { sys::lv_obj_get_scroll_y(common::app_root().as_ptr()) }
        } else {
            0
        };
        ui.unmount();
        ui.mount(&page).unwrap();
        if family == "list" {
            ui.enable_focus().unwrap();
            assert!(ui.focus(Key::Id(1)).unwrap());
            // SAFETY: restore the same viewport on the fresh list root.
            unsafe { sys::lv_obj_scroll_to_y(common::app_root().as_ptr(), scroll_y, false) };
        }
        refresh();
        if frame.borrow().as_slice() != expected_pixels.as_slice() {
            if let Ok(folder) = std::env::var("UI_WORKLOAD_ARTIFACTS") {
                for (name, data) in [
                    ("updated", expected_pixels.as_slice()),
                    ("fresh", frame.borrow().as_slice()),
                ] {
                    let mut ppm = format!("P6\n{WIDTH} {HEIGHT}\n255\n").into_bytes();
                    ppm.extend_from_slice(data);
                    std::fs::write(
                        std::path::Path::new(&folder)
                            .join(format!("{family}-{scenario}-{name}.ppm")),
                        ppm,
                    )
                    .unwrap();
                }
            }
        }
        assert!(
            frame.borrow().as_slice() == expected_pixels.as_slice(),
            "WORKLOAD_ORACLE_PIXELS: {family}/{scenario}, first difference {:?}",
            frame
                .borrow()
                .iter()
                .zip(&expected_pixels)
                .position(|(a, b)| a != b)
        );
        println!("WORKLOAD_JSON:{{\"family\":\"{family}\",\"scenario\":\"{scenario}\",\"size\":{size},\"series\":{series_count},\"append_batch\":{append_batch},\"updates\":{BATCH},\"update_total_ns\":{total_ns},\"construction_ns\":{construction_ns},\"update_excluding_construction_ns\":{},\"memory_render_flush_ns\":{draw_ns},\"view_calls\":{},\"actions\":{},\"creates\":{},\"deletes\":{},\"moves\":{},\"label_text_sets\":{},\"rust_alloc_calls\":{alloc_calls},\"rust_requested_bytes\":{alloc_bytes},\"rust_peak_extra_bytes\":{peak},\"rust_live_delta\":{live_delta},\"native_used_before\":{},\"native_used_after\":{},\"native_blocks_after\":{},\"native_high_water_since_init\":{},\"pixel_hash\":\"{pixels:016x}\"}}",
            total_ns.checked_sub(construction_ns).expect("phase clocks nested"),
            view_calls, actions,
            stats.created_nodes - counters.created_nodes, stats.deleted_nodes - counters.deleted_nodes,
            stats.moved_nodes - counters.moved_nodes,
            label_sets,
            native_before.total_size - native_before.free_size,
            native_after.total_size - native_after.free_size,
            native_after.used_cnt, native_after.max_used);
    }
    ui.unmount();
    drop(ui);
    // SAFETY: all trees/focus groups released, synchronous memory rendering idle.
    unsafe { display.delete() };
}

#[test]
fn representative_workloads() {
    let _guard = common::lock();
    assert_eq!(
        sys::LV_USE_STDLIB_MALLOC,
        sys::LV_STDLIB_BUILTIN,
        "WORKLOAD_NATIVE_ALLOCATOR"
    );
    rs_lvgl::init();
    let clocks = std::env::var("UI_WORKLOAD_CLOCKS").as_deref() != Ok("off");
    let corrupt = std::env::var("UI_WORKLOAD_CORRUPT").unwrap_or_default();
    // Positive control proves that allocation metering is active only in debug.
    let before = meter::begin();
    let probe = std::hint::black_box(vec![1u8; 257]);
    let sample = meter::end(before);
    if cfg!(debug_assertions) {
        assert!(sample.0 > 0 && sample.1 >= 257);
    } else {
        assert_eq!(sample, (0, 0, 0, 0));
    }
    drop(probe);
    println!("WORKLOAD_META:{{\"schema\":1,\"warmup\":{WARMUP},\"batch\":{BATCH},\"phase_clocks\":{clocks},\"allocation_instrumentation\":{},\"view_bytes\":{},\"style_bytes\":{},\"width\":{WIDTH},\"height\":{HEIGHT}}}",
        cfg!(debug_assertions), std::mem::size_of::<View<Action>>(), std::mem::size_of::<Style>());
    run_case(
        Model::Settings {
            value: 30,
            enabled: false,
            text: "Canonical".into(),
            overlay: false,
        },
        &[
            "idle",
            "unchanged",
            "numeric",
            "rejected",
            "theme",
            "overlay",
        ],
        clocks,
        &corrupt,
    );
    for size in [32, 128, 512] {
        run_case(
            Model::List {
                items: (0..size).map(|i| (i as u64, i)).collect(),
            },
            &[
                "label",
                "prepend-remove",
                "rotate",
                "reverse",
                "scroll-focus",
            ],
            clocks,
            &corrupt,
        );
    }
    for series in [1, 4] {
        for capacity in [128, 1024] {
            for batch in [1, 16] {
                run_case(
                    Model::Chart {
                        series,
                        capacity,
                        batch,
                        sequence: capacity as i32,
                        history: (0..capacity).map(|v| v as i32 % 100).collect(),
                    },
                    &["replacement", "wrap", "skipped", "remount"],
                    clocks,
                    &corrupt,
                );
            }
        }
    }
    println!("WORKLOAD_CORRECTNESS:PASS");
    // SAFETY: all run_case displays and every dependent object were deleted.
    unsafe { rs_lvgl::deinit() };
}
