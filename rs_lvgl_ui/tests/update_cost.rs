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
    type Msg = ();
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
