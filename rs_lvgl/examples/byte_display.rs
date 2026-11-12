//! Host memory-display example; no panel, window or hardware DMA is used.
use rs_lvgl::{ColorFormat, DisplayBuffers, DisplayBuilder, PixelBuffer, RenderMode};
use std::{cell::Cell, rc::Rc, time::Duration};

fn main() {
    rs_lvgl::init();
    let pixels = Rc::new(Cell::new(0usize));
    let count = pixels.clone();
    let mut display = DisplayBuilder::new(7, 5)
        .color_format(ColorFormat::Rgb888)
        .render_mode(RenderMode::Direct)
        .stride(32)
        .buffers(DisplayBuffers::Double {
            front: PixelBuffer::Owned(vec![0; 160].into_boxed_slice()),
            back: PixelBuffer::Owned(vec![0; 160].into_boxed_slice()),
        })
        .build(move |flush| {
            // SAFETY: this synchronous sink finishes every row read here and
            // never reenters LVGL or starts a transfer that outlives the callback.
            for row in unsafe { flush.rows() } {
                assert!(row.chunks_exact(3).all(|bgr| bgr == [0, 0, 255]));
                count.set(count.get() + row.len() / 3);
            }
        })
        .unwrap();
    // Choose the software period for the actual device; it does not set panel Hz.
    display
        .set_refresh_period(Duration::from_millis(33))
        .unwrap();
    unsafe {
        let screen = rs_lvgl::sys::lv_display_get_screen_active(display.raw().as_ptr());
        rs_lvgl::sys::lv_obj_set_style_bg_color(screen, rs_lvgl::sys::lv_color_make(255, 0, 0), 0);
        rs_lvgl::sys::lv_obj_set_style_bg_opa(screen, 255, 0);
        rs_lvgl::sys::lv_refr_now(display.raw().as_ptr());
    }
    assert_eq!(pixels.get(), 35);
    println!(
        "RGB888 memory display: {} red pixels, 32-byte stride",
        pixels.get()
    );
    // SAFETY: no widget/input aliases or native/DMA/render work remain.
    unsafe {
        display.delete();
    }
}
