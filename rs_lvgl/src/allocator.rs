use core::alloc::{GlobalAlloc, Layout};

#[global_allocator]
static ALLOCATOR: LvglAlloc = LvglAlloc;

pub struct LvglAlloc;

unsafe impl GlobalAlloc for LvglAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        crate::init();
        rs_lvgl_sys::lv_malloc(layout.size() as cty::size_t) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        crate::init();
        rs_lvgl_sys::lv_free(ptr as *mut cty::c_void)
    }
}
