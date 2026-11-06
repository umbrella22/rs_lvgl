//! Isolated update accounting; run through scripts/check-ui-costs.py.
use rs_lvgl_ui::prelude::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
use std::time::Instant;

struct Meter;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static CALLS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
static TRACK: AtomicBool = AtomicBool::new(false);

fn allocated(size: usize) {
    let live = LIVE.fetch_add(size, SeqCst) + size;
    if TRACK.load(SeqCst) {
        CALLS.fetch_add(1, SeqCst);
        BYTES.fetch_add(size, SeqCst);
        PEAK.fetch_max(live, SeqCst);
    }
}

// SAFETY: System receives unchanged pointer/layout arguments. Counters allocate
// nothing, and only successful allocation operations change live accounting.
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            allocated(layout.size());
        }
        ptr
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            allocated(layout.size());
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), SeqCst);
        unsafe { System.dealloc(ptr, layout) };
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let next = unsafe { System.realloc(ptr, layout, size) };
        if !next.is_null() {
            LIVE.fetch_sub(layout.size(), SeqCst);
            allocated(size);
        }
        next
    }
}

#[global_allocator]
static ALLOCATOR: Meter = Meter;

fn begin() -> usize {
    let before = LIVE.load(SeqCst);
    PEAK.store(before, SeqCst);
    CALLS.store(0, SeqCst);
    BYTES.store(0, SeqCst);
    TRACK.store(true, SeqCst);
    before
}

fn end(before: usize) -> (usize, usize, usize, isize) {
    TRACK.store(false, SeqCst);
    (
        CALLS.load(SeqCst),
        BYTES.load(SeqCst),
        PEAK.load(SeqCst).saturating_sub(before),
        LIVE.load(SeqCst) as isize - before as isize,
    )
}

struct Page(u32);
impl App for Page {
    type Action = ();
    fn update(&mut self, _: ()) {
        self.0 += 1;
    }
    fn view(&self, _: &Theme) -> View<()> {
        column((0..64).map(|index| {
            let node = if index == 0 {
                label(format!("value {}", self.0))
            } else {
                #[cfg(feature = "cost-static-text")]
                {
                    label_static("Fixed display text")
                }
                #[cfg(not(feature = "cost-static-text"))]
                {
                    label("Fixed display text")
                }
            };
            node.key(Key::Id(index))
        }))
    }
}

fn native_usage() -> rs_lvgl_sys::lv_mem_monitor_t {
    let mut monitor = rs_lvgl_sys::lv_mem_monitor_t::default();
    // SAFETY: only the serial test thread accesses the initialized native pool.
    unsafe { rs_lvgl_sys::lv_mem_monitor(&mut monitor) };
    monitor
}

fn explicit_form(capture: i32) -> View<i32> {
    column([])
        .child(slider(0, 100, 50).on_value_changed(move |v| v + capture))
        .child(switch(false).on_toggle(move |v| i32::from(v) + capture))
        .child(textarea("model").on_text_changed(move |v| v.len() as i32 + capture))
}

fn typed_form(capture: i32) -> View<i32> {
    typed::column([])
        .child(typed::slider(0, 100, 50).on_value_changed(move |v| v + capture))
        .child(typed::switch(false).on_toggle(move |v| i32::from(v) + capture))
        .child(typed::textarea("model").on_text_changed(move |v| v.len() as i32 + capture))
        .into_view()
}

fn bound_form(capture: i32) -> View<i32> {
    typed::column([])
        .child(typed::slider(0, 100, binding(50, move |v| v + capture)))
        .child(typed::switch(binding(false, move |v| {
            i32::from(v) + capture
        })))
        .child(typed::textarea(binding("model", move |v: &str| {
            v.len() as i32 + capture
        })))
        .into_view()
}

#[test]
fn form_construction_measurements() {
    let mut reference = None;
    for (scenario, build) in [
        ("erased-explicit", explicit_form as fn(i32) -> View<i32>),
        ("typed-explicit", typed_form),
        ("typed-binding", bound_form),
    ] {
        let mut totals = (0, 0, 0, 0);
        for index in 0..32 {
            let before = begin();
            let form = std::hint::black_box(build(std::hint::black_box(index)));
            let sample = end(before);
            totals.0 += sample.0;
            totals.1 += sample.1;
            totals.2 = totals.2.max(sample.2);
            totals.3 += sample.3;
            drop(form);
            assert_eq!(LIVE.load(SeqCst), before, "FORM_CAPTURE_LEAK: {scenario}");
        }
        if let Some(expected) = reference {
            assert_eq!(totals, expected, "FORM_ADAPTER_COST: {scenario}");
        } else {
            reference = Some(totals);
        }
        println!("FORM_COST_JSON:{{\"scenario\":\"{scenario}\",\"constructions\":32,\"rust_alloc_calls\":{},\"rust_requested_bytes\":{},\"rust_peak_extra_bytes\":{},\"rust_retained_bytes_sum\":{}}}", totals.0, totals.1, totals.2, totals.3);
    }
}

#[test]
fn measurements() {
    assert_eq!(
        rs_lvgl_sys::LV_USE_STDLIB_MALLOC,
        rs_lvgl_sys::LV_STDLIB_BUILTIN,
        "COST_UNOBSERVABLE_NATIVE: use check-ui-costs.py"
    );
    let before = begin();
    let probe = std::hint::black_box(vec![23u8; 257]);
    let control = end(before);
    assert!(
        control.0 >= 1 && control.1 >= 257 && control.2 >= 257,
        "COST_CONTROL_MISSING: {control:?}"
    );
    drop(probe);
    println!("COST_CONTROL:PASS");
    let display = rs_lvgl::Display::register::<_, { 64 * 64 }>(64, 64, |_| {}).unwrap();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = Page(0);
    ui.mount(&app).unwrap();
    ui.update(&app).unwrap(); // One declared priming update.
    for scenario in ["idle", "unchanged", "one-value"] {
        let mut total_calls = 0;
        let mut total_bytes = 0;
        let mut peak = 0;
        let mut live_delta = 0;
        let native_before = native_usage();
        let writes_before = rs_lvgl_ui::update_metrics().native_label_text_sets;
        let start = Instant::now();
        for _ in 0..16 {
            let before = begin();
            match scenario {
                "idle" => {
                    ui.drain(&mut app, 8).unwrap();
                }
                "unchanged" => ui.update(&app).unwrap(),
                _ => {
                    app.0 += 1;
                    ui.update(&app).unwrap();
                }
            }
            let sample = end(before);
            total_calls += sample.0;
            total_bytes += sample.1;
            peak = peak.max(sample.2);
            live_delta += sample.3;
        }
        let nanos = start.elapsed().as_nanos();
        let native_after = native_usage();
        let writes = rs_lvgl_ui::update_metrics().native_label_text_sets - writes_before;
        let mode = if cfg!(feature = "cost-static-text") {
            "static"
        } else {
            "owned"
        };
        println!("COST_JSON:{{\"scenario\":\"{scenario}\",\"text\":\"{mode}\",\"updates\":16,\"rust_alloc_calls\":{total_calls},\"rust_requested_bytes\":{total_bytes},\"rust_peak_extra_bytes\":{peak},\"rust_live_delta\":{live_delta},\"native_used_before\":{},\"native_used_after\":{},\"native_high_water_since_init\":{},\"native_label_text_sets\":{writes},\"elapsed_ns\":{nanos}}}",
                 native_before.total_size-native_before.free_size,
                 native_after.total_size-native_after.free_size, native_after.max_used);
    }
    ui.unmount();
    drop(ui);
    // SAFETY: UI is gone; this memory display's flush completes synchronously.
    unsafe { display.delete() };
}

struct StyledPage {
    changed: bool,
    reversed: bool,
}

impl App for StyledPage {
    type Action = ();
    fn update(&mut self, _: ()) {}
    fn view(&self, theme: &Theme) -> View<()> {
        column((0..24).map(|position| {
            let id = if self.reversed {
                23 - position
            } else {
                position
            };
            row([
                label_static("Status")
                    .font(theme.body_font)
                    .text_color(theme.text),
                button_static("Apply")
                    .style(Style::accent(theme))
                    .state_style(State::PRESSED, style().opacity(200)),
            ])
            .key(Key::Id(id))
            .width(percent(100))
            .gap(if self.changed { 10 } else { 6 })
            .padding(4)
        }))
        .style(Style::screen(theme))
        .gap(6)
        .padding(8)
    }
}

#[test]
fn styled_measurements() {
    assert_eq!(
        rs_lvgl_sys::LV_USE_STDLIB_MALLOC,
        rs_lvgl_sys::LV_STDLIB_BUILTIN,
        "COST_UNOBSERVABLE_NATIVE: use check-ui-costs.py"
    );
    let before = begin();
    std::hint::black_box(container::<()>().gap(12).padding(16));
    let construction = end(before);
    assert_eq!(construction, (0, 0, 0, 0), "STYLE_CONSTRUCTION_ALLOCATED");
    println!("STYLE_CONTROL:PASS");
    println!(
        "STYLE_SIZE_JSON:{{\"style\":{},\"view\":{},\"native_style\":{}}}",
        core::mem::size_of::<Style>(),
        core::mem::size_of::<View<()>>(),
        core::mem::size_of::<rs_lvgl_sys::lv_style_t>()
    );

    let display = rs_lvgl::Display::register::<_, { 64 * 64 }>(64, 64, |_| {}).unwrap();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let mut app = StyledPage {
        changed: false,
        reversed: false,
    };
    ui.mount(&app).unwrap();
    ui.update(&app).unwrap();
    // Prime both key orders and themes before measuring steady-state updates.
    // The first reorder releases excess child-vector capacity from mounting.
    app.reversed = true;
    ui.update(&app).unwrap();
    app.reversed = false;
    ui.update(&app).unwrap();
    ui.set_theme(Theme::light());
    ui.update(&app).unwrap();
    ui.set_theme(Theme::dark());
    ui.update(&app).unwrap();
    for scenario in [
        "idle",
        "unchanged",
        "style-change",
        "reorder",
        "theme-change",
    ] {
        let native_before = native_usage();
        let mut calls = 0;
        let mut bytes = 0;
        let mut peak = 0;
        let mut live_delta = 0;
        for index in 0..16 {
            let before = begin();
            match scenario {
                "idle" => {
                    assert!(!ui.drain(&mut app, 8).unwrap().reconciled);
                }
                "unchanged" => ui.update(&app).unwrap(),
                "style-change" => {
                    app.changed = !app.changed;
                    ui.update(&app).unwrap();
                }
                "reorder" => {
                    app.reversed = !app.reversed;
                    ui.update(&app).unwrap();
                }
                _ => {
                    ui.set_theme(if index % 2 == 0 {
                        Theme::light()
                    } else {
                        Theme::dark()
                    });
                    assert!(ui.drain(&mut app, 8).unwrap().reconciled);
                }
            }
            let sample = end(before);
            calls += sample.0;
            bytes += sample.1;
            peak = peak.max(sample.2);
            live_delta += sample.3;
        }
        let native_after = native_usage();
        assert_eq!(live_delta, 0, "STYLE_RUST_GROWTH: {scenario}");
        assert_eq!(native_after.used_cnt, native_before.used_cnt);
        assert_eq!(native_after.free_size, native_before.free_size);
        println!("STYLE_COST_JSON:{{\"scenario\":\"{scenario}\",\"updates\":16,\"rust_alloc_calls\":{calls},\"rust_requested_bytes\":{bytes},\"rust_peak_extra_bytes\":{peak},\"rust_live_delta\":{live_delta},\"native_used_before\":{},\"native_used_after\":{},\"native_live_blocks\":{},\"native_high_water_since_init\":{}}}",
                 native_before.total_size - native_before.free_size,
                 native_after.total_size - native_after.free_size,
                 native_after.used_cnt, native_after.max_used);
    }
    ui.unmount();
    drop(ui);
    // SAFETY: no UI or pending asynchronous rendering remains on this display.
    unsafe { display.delete() };
}
