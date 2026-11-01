//! Optional counters for update-cost experiments, independent of heap accounting.
use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};
static LABEL_TEXT_SETS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UpdateMetrics {
    pub native_label_text_sets: usize,
}

pub fn update_metrics() -> UpdateMetrics {
    UpdateMetrics {
        native_label_text_sets: LABEL_TEXT_SETS.load(Relaxed),
    }
}
pub(crate) fn label_text_set() {
    LABEL_TEXT_SETS.fetch_add(1, Relaxed);
}
