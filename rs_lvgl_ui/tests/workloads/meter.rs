//! Test-only allocator accounting, never linked into release timing.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};

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

pub fn begin() -> usize {
    let before = LIVE.load(SeqCst);
    PEAK.store(before, SeqCst);
    CALLS.store(0, SeqCst);
    BYTES.store(0, SeqCst);
    TRACK.store(true, SeqCst);
    before
}

pub fn end(before: usize) -> (usize, usize, usize, isize) {
    TRACK.store(false, SeqCst);
    (
        CALLS.load(SeqCst),
        BYTES.load(SeqCst),
        PEAK.load(SeqCst).saturating_sub(before),
        LIVE.load(SeqCst) as isize - before as isize,
    )
}
