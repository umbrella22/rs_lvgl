//! Paired full-sync/history strategies with identical canonical samples and pixels.
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
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    num::NonZeroU32,
    rc::Rc,
    time::Instant,
};
const WARMUP: usize = 4;
const BATCH: usize = 16;
const WIDTH: u32 = 240;
const HEIGHT: u32 = 160;
struct Page {
    history: Vec<ChartHistory>,
    reference: Vec<VecDeque<(u64, i32)>>,
    capacity: usize,
    circular: bool,
    full: bool,
    clocks: bool,
    construction: Cell<u128>,
}
impl Page {
    fn advance(&mut self, count: usize) {
        for (id, (history, reference)) in
            self.history.iter_mut().zip(&mut self.reference).enumerate()
        {
            for _ in 0..count {
                let sequence = history.next_sequence();
                let value = ((sequence % 97) as i32 + id as i32 * 7) % 100;
                assert_eq!(history.push_y(value).unwrap(), sequence);
                if reference.len() == self.capacity {
                    reference.pop_front();
                }
                reference.push_back((sequence, value));
            }
        }
    }
    fn displayed(&self, id: usize) -> Vec<i32> {
        if !self.circular {
            return self.reference[id].iter().map(|(_, v)| *v).collect();
        }
        let mut values = vec![0; self.capacity];
        for (sequence, value) in &self.reference[id] {
            values[(*sequence % self.capacity as u64) as usize] = *value;
        }
        values
    }
    fn verify(&self, corrupt: bool) {
        // SAFETY: the currently mounted root is a native chart with owned arrays.
        unsafe {
            let chart = common::app_root().as_ptr();
            let mut series = sys::lv_chart_get_series_next(chart, std::ptr::null());
            for id in 0..self.history.len() {
                assert!(!series.is_null(), "HISTORY_ORACLE_SAMPLES");
                let points = std::slice::from_raw_parts(
                    sys::lv_chart_get_series_y_array(chart, series),
                    self.capacity,
                );
                let start = sys::lv_chart_get_x_start_point(chart, series) as usize;
                let actual: Vec<_> = (0..self.capacity)
                    .map(|i| {
                        points[if self.circular {
                            i
                        } else {
                            (start + i) % self.capacity
                        }]
                    })
                    .collect();
                let mut expected = self.displayed(id);
                if corrupt {
                    expected[0] ^= 1;
                }
                assert_eq!(actual, expected, "HISTORY_ORACLE_SAMPLES");
                series = sys::lv_chart_get_series_next(chart, series);
            }
            assert!(series.is_null());
        }
    }
}
impl App for Page {
    type Action = ();
    fn update(&mut self, _: ()) {}
    fn view(&self, _: &Theme) -> View<()> {
        let start = self.clocks.then(Instant::now);
        let mut spec = ChartSpec::new(
            ChartType::Line,
            ChartAxes::new(ChartRange::new(0, 100), ChartRange::new(0, 100)),
            self.history
                .iter()
                .enumerate()
                .map(|(id, history)| {
                    let data = if self.full {
                        ChartData::values(self.displayed(id))
                    } else {
                        ChartData::History(history.snapshot())
                    };
                    ChartSeries::new(id as u64, Color::rgb(40 + id as u8 * 40, 150, 200), data)
                })
                .collect(),
        );
        spec.update_mode = if self.circular {
            ChartUpdateMode::Circular
        } else {
            ChartUpdateMode::Shift
        };
        let view = chart(spec)
            .width(px(WIDTH as i16))
            .height(px(HEIGHT as i16));
        if let Some(start) = start {
            self.construction
                .set(self.construction.get() + start.elapsed().as_nanos());
        }
        view
    }
}
fn refresh() {
    unsafe {
        sys::lv_refr_now(sys::lv_obj_get_display(common::screen().as_ptr()));
    }
}
fn memory() -> sys::lv_mem_monitor_t {
    let mut m = unsafe { std::mem::zeroed() };
    unsafe { sys::lv_mem_monitor(&mut m) };
    m
}
fn run_case(
    series: usize,
    capacity: usize,
    batch: usize,
    circular: bool,
    full: bool,
    clocks: bool,
    corrupt: &str,
) {
    // SAFETY: the test serializes LVGL and deletes every display before this reset.
    unsafe {
        rs_lvgl::deinit();
    }
    rs_lvgl::init();
    let frame = Rc::new(RefCell::new(vec![
        0u8;
        WIDTH as usize * HEIGHT as usize * 3
    ]));
    let output = frame.clone();
    let display = rs_lvgl::Display::register::<_, { 240 * 8 }>(WIDTH, HEIGHT, move |refresh| {
        let mut pixels = output.borrow_mut();
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
        history: (0..series)
            .map(|id| {
                ChartHistory::new(
                    id as u64 + 1,
                    NonZeroU32::new(capacity as u32).unwrap(),
                    ChartSampleKind::Y,
                )
            })
            .collect(),
        reference: (0..series).map(|_| VecDeque::new()).collect(),
        capacity,
        circular,
        full,
        clocks,
        construction: Cell::new(0),
    };
    page.advance(capacity);
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    ui.mount(&page).unwrap();
    refresh();
    for _ in 0..WARMUP {
        page.advance(batch);
        ui.update(&page).unwrap();
        page.verify(false);
        refresh();
    }
    let native_before = memory();
    let counters = rs_lvgl_ui::update_metrics();
    let stats = ui.stats();
    let mut total = 0;
    let mut construction = 0;
    let mut draw = 0;
    let mut allocations = 0;
    let mut bytes = 0;
    let mut peak = 0;
    let mut live = 0;
    for _ in 0..BATCH {
        page.advance(batch);
        let prior = page.construction.get();
        let meter_start = meter::begin();
        let start = Instant::now();
        ui.update(&page).unwrap();
        total += start.elapsed().as_nanos();
        let sample = meter::end(meter_start);
        allocations += sample.0;
        bytes += sample.1;
        peak = peak.max(sample.2);
        live += sample.3;
        construction += page.construction.get() - prior;
        page.verify(corrupt == "samples");
        let start = Instant::now();
        refresh();
        draw += start.elapsed().as_nanos();
    }
    let native_after = memory();
    let after = rs_lvgl_ui::update_metrics();
    let mut appends = after.native_chart_sample_appends - counters.native_chart_sample_appends;
    if corrupt == "counts" {
        appends += 1;
    }
    let writes = after.native_chart_full_sample_writes - counters.native_chart_full_sample_writes;
    let syncs = after.native_chart_full_syncs - counters.native_chart_full_syncs;
    assert_eq!(
        appends,
        if full { 0 } else { series * batch * BATCH },
        "HISTORY_ORACLE_COUNTS"
    );
    assert_eq!(
        writes,
        if full { series * capacity * BATCH } else { 0 },
        "HISTORY_ORACLE_COUNTS"
    );
    assert_eq!(
        syncs,
        if full { series * BATCH } else { 0 },
        "HISTORY_ORACLE_COUNTS"
    );
    assert_eq!(
        ui.stats().created_nodes,
        stats.created_nodes,
        "HISTORY_ORACLE_IDENTITY"
    );
    assert_eq!(
        ui.stats().deleted_nodes,
        stats.deleted_nodes,
        "HISTORY_ORACLE_IDENTITY"
    );
    let pixels = frame.borrow().clone();
    assert!(pixels.windows(3).any(|w| w[0] != w[1]));
    let hash = pixels.iter().fold(0xcbf29ce484222325u64, |hash, b| {
        (hash ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    });
    // A fresh full-vector chart supplies an independent native pixel oracle.
    ui.unmount();
    page.full = true;
    ui.mount(&page).unwrap();
    refresh();
    let mut expected = frame.borrow().clone();
    if corrupt == "pixels" {
        expected[0] ^= 1;
    }
    assert_eq!(pixels, expected, "HISTORY_ORACLE_PIXELS");
    let mode = if circular { "circular" } else { "shift" };
    let strategy = if full { "full" } else { "append" };
    println!("HISTORY_JSON:{{\"family\":\"history\",\"scenario\":\"{mode}\",\"size\":{capacity},\"series\":{series},\"append_batch\":{batch},\"strategy\":\"{strategy}\",\"updates\":{BATCH},\"update_total_ns\":{total},\"construction_ns\":{construction},\"update_excluding_construction_ns\":{},\"memory_render_flush_ns\":{draw},\"rust_alloc_calls\":{allocations},\"rust_requested_bytes\":{bytes},\"rust_peak_extra_bytes\":{peak},\"rust_live_delta\":{live},\"native_appends\":{appends},\"native_full_writes\":{writes},\"native_full_syncs\":{syncs},\"native_used_before\":{},\"native_used_after\":{},\"native_high_water_since_init\":{},\"pixel_hash\":\"{hash:016x}\"}}",total.checked_sub(construction).unwrap(),native_before.total_size-native_before.free_size,native_after.total_size-native_after.free_size,native_after.max_used);
    ui.unmount();
    drop(ui);
    unsafe {
        display.delete();
    }
}
#[test]
fn bounded_history_workloads() {
    let _lock = common::lock();
    assert_eq!(
        sys::LV_USE_STDLIB_MALLOC,
        sys::LV_STDLIB_BUILTIN,
        "HISTORY_NATIVE_ALLOCATOR"
    );
    let strategy = std::env::var("UI_HISTORY_STRATEGY").unwrap_or_else(|_| "append".into());
    assert!(strategy == "append" || strategy == "full");
    let clocks = std::env::var("UI_HISTORY_CLOCKS").as_deref() != Ok("off");
    let corrupt = std::env::var("UI_HISTORY_CORRUPT").unwrap_or_default();
    let m = meter::begin();
    let probe = std::hint::black_box(vec![1u8; 257]);
    let sample = meter::end(m);
    if cfg!(debug_assertions) {
        assert!(sample.0 > 0 && sample.1 >= 257);
    } else {
        assert_eq!(sample, (0, 0, 0, 0));
    }
    drop(probe);
    println!("HISTORY_META:{{\"allocation_instrumentation\":{},\"warmup\":{WARMUP},\"batch\":{BATCH},\"phase_clocks\":{clocks},\"width\":{WIDTH},\"height\":{HEIGHT}}}",cfg!(debug_assertions));
    for series in [1, 4] {
        for capacity in [128, 1024] {
            for batch in [1, 16] {
                for circular in [false, true] {
                    run_case(
                        series,
                        capacity,
                        batch,
                        circular,
                        strategy == "full",
                        clocks,
                        &corrupt,
                    );
                }
            }
        }
    }
    println!("HISTORY_CORRECTNESS:PASS");
    unsafe {
        rs_lvgl::deinit();
    }
}
