//! Optional counters for update-cost experiments, independent of heap accounting.
use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};
static LABEL_TEXT_SETS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UpdateMetrics {
    pub native_label_text_sets: usize,
    pub native_chart_sample_appends: usize,
    pub native_chart_full_sample_writes: usize,
    pub native_chart_full_syncs: usize,
}

pub fn update_metrics() -> UpdateMetrics {
    UpdateMetrics {
        native_label_text_sets: LABEL_TEXT_SETS.load(Relaxed),
        native_chart_sample_appends: CHART_APPENDS.load(Relaxed),
        native_chart_full_sample_writes: CHART_FULL_WRITES.load(Relaxed),
        native_chart_full_syncs: CHART_FULL_SYNCS.load(Relaxed),
    }
}
pub(crate) fn label_text_set() {
    LABEL_TEXT_SETS.fetch_add(1, Relaxed);
}

static CHART_APPENDS: AtomicUsize = AtomicUsize::new(0);
static CHART_FULL_WRITES: AtomicUsize = AtomicUsize::new(0);
static CHART_FULL_SYNCS: AtomicUsize = AtomicUsize::new(0);

// One XY pair counts as one sample; these exclude render and allocator calls.
#[cfg(feature = "widget-chart")]
pub(crate) fn chart_append(count: usize) {
    CHART_APPENDS.fetch_add(count, Relaxed);
}
#[cfg(feature = "widget-chart")]
pub(crate) fn chart_full(count: usize) {
    CHART_FULL_WRITES.fetch_add(count, Relaxed);
    CHART_FULL_SYNCS.fetch_add(1, Relaxed);
}
