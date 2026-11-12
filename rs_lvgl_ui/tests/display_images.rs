#![cfg(all(feature = "widget-image", feature = "widget-canvas"))]
mod common;
use rs_lvgl::{CanvasBuffer, ColorFormat, NativeObject, PixelBuffer};
use rs_lvgl_ui::prelude::*;

#[repr(align(4))]
struct Pixels([u8; 24]);
static BGR: Pixels = Pixels([
    0, 0, 255, 0, 255, 0, 255, 0, 0, 91, 92, 93, 255, 0, 0, 0, 255, 0, 0, 0, 255, 94, 95, 96,
]);

struct Picture(ImageSource);
impl App for Picture {
    type Action = ();
    fn update(&mut self, _: ()) {}
    fn view(&self, _: &Theme) -> View<()> {
        image(self.0)
    }
}

#[test]
fn rgb888_image_uses_bgr_stride_and_replaces_its_descriptor() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Picture(ImageSource::rgb888_with_stride(3, 2, 12, &BGR.0));
    ui.mount(&app).unwrap();
    let object = common::app_root();
    let snapshot = ui.snapshot().unwrap();
    assert_eq!((snapshot.width, snapshot.height), (3, 2));
    assert_eq!(
        snapshot.pixels,
        [
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 255, 0, 255, 255, 0,
            0, 255,
        ]
    );
    app.0 = ImageSource::rgb888(1, 1, &BGR.0[..3]);
    ui.update(&app).unwrap();
    assert_eq!(common::app_root(), object);
    assert_eq!(ui.snapshot().unwrap().pixels, [255, 0, 0, 255]);
    app.0 = ImageSource::rgb565(1, 1, &[31, 0]);
    ui.update(&app).unwrap();
    assert_eq!(ui.snapshot().unwrap().pixels, [0, 0, 255, 255]);
    ui.unmount();
}

#[test]
fn invalid_rgb888_layout_preserves_the_mounted_image() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    ui.mount(&Picture(ImageSource::rgb888(1, 1, &BGR.0[..3])))
        .unwrap();
    let object = common::app_root();
    for source in [
        ImageSource::rgb888_with_stride(3, 2, 8, &BGR.0),
        ImageSource::rgb888_with_stride(3, 2, 65536, &BGR.0),
        ImageSource::rgb888(3, 2, &BGR.0[..17]),
        ImageSource::rgb888(0, 2, &[]),
    ] {
        assert_eq!(ui.update(&Picture(source)), Err(UiError::InvalidProperty));
        assert_eq!(common::app_root(), object);
        assert_eq!(ui.snapshot().unwrap().pixels, [255, 0, 0, 255]);
    }
    ui.unmount();
}

#[test]
fn native_canvas_retains_padded_rgb888_and_replaces_with_rgb565() {
    let _guard = common::lock();
    let _display = common::init();
    let mut canvas = rs_lvgl::widgets::Canvas::new().unwrap();
    let frame = CanvasBuffer::new(
        3,
        2,
        ColorFormat::Rgb888,
        Some(16),
        PixelBuffer::Owned(vec![0xA5; 32].into_boxed_slice()),
    )
    .unwrap();
    canvas.attach_buffer(frame).unwrap();
    unsafe {
        let raw = canvas.raw().as_ptr();
        rs_lvgl_sys::lv_canvas_fill_bg(raw, rs_lvgl_sys::lv_color_make(255, 0, 0), 255);
        rs_lvgl_sys::lv_canvas_set_px(raw, 2, 1, rs_lvgl_sys::lv_color_make(0, 0, 255), 255);
        let descriptor = &*rs_lvgl_sys::lv_canvas_get_draw_buf(raw);
        assert_eq!(descriptor.header.stride(), 16);
        let bytes = core::slice::from_raw_parts(descriptor.data, 32);
        assert_eq!(&bytes[..9], &[0, 0, 255, 0, 0, 255, 0, 0, 255]);
        assert_eq!(&bytes[22..25], &[255, 0, 0]);
        assert_eq!(&bytes[9..16], &[0xA5; 7]);
        assert_eq!(&bytes[25..32], &[0xA5; 7]);
    }
    unsafe {
        let raw = canvas.raw().as_ptr();
        let mut layer: rs_lvgl_sys::lv_layer_t = core::mem::zeroed();
        rs_lvgl_sys::lv_canvas_init_layer(raw, &mut layer);
        let mut rectangle: rs_lvgl_sys::lv_draw_rect_dsc_t = core::mem::zeroed();
        rs_lvgl_sys::lv_draw_rect_dsc_init(&mut rectangle);
        rectangle.bg_color = rs_lvgl_sys::lv_color_make(0, 255, 0);
        rectangle.bg_opa = 255;
        let area = rs_lvgl_sys::lv_area_t {
            x1: 1,
            y1: 0,
            x2: 1,
            y2: 1,
        };
        rs_lvgl_sys::lv_draw_rect(&mut layer, &rectangle, &area);
        rs_lvgl_sys::lv_canvas_finish_layer(raw, &mut layer);
        let descriptor = &*rs_lvgl_sys::lv_canvas_get_draw_buf(raw);
        let bytes = core::slice::from_raw_parts(descriptor.data, 32);
        assert_eq!(&bytes[3..6], &[0, 255, 0]);
        assert_eq!(&bytes[19..22], &[0, 255, 0]);
        assert_eq!(&bytes[9..16], &[0xA5; 7]);
    }
    let previous = unsafe { rs_lvgl_sys::lv_canvas_get_draw_buf(canvas.raw().as_ptr()) };
    assert!(matches!(
        CanvasBuffer::new(
            3,
            2,
            ColorFormat::Rgb888,
            Some(16),
            PixelBuffer::Owned(vec![0; 31].into_boxed_slice())
        ),
        Err(rs_lvgl::DisplayConfigError::BufferTooSmall)
    ));
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_canvas_get_draw_buf(canvas.raw().as_ptr()) },
        previous
    );
    canvas
        .attach_buffer(
            CanvasBuffer::new(
                2,
                2,
                ColorFormat::Rgb565,
                Some(8),
                PixelBuffer::Owned(vec![0; 16].into_boxed_slice()),
            )
            .unwrap(),
        )
        .unwrap();
    unsafe {
        rs_lvgl_sys::lv_canvas_fill_bg(
            canvas.raw().as_ptr(),
            rs_lvgl_sys::lv_color_make(0, 255, 0),
            255,
        );
        let descriptor = &*rs_lvgl_sys::lv_canvas_get_draw_buf(canvas.raw().as_ptr());
        assert_eq!(descriptor.header.cf(), ColorFormat::Rgb565.to_raw());
        assert_eq!(
            core::slice::from_raw_parts(descriptor.data, 4),
            &[224, 7, 224, 7]
        );
    }
    assert!(matches!(
        CanvasBuffer::new(
            2,
            2,
            ColorFormat::Rgb565Swapped,
            None,
            PixelBuffer::Owned(vec![0; 16].into_boxed_slice())
        ),
        Err(rs_lvgl::DisplayConfigError::UnsupportedFormat)
    ));
    // Obj handles do not implicitly delete native objects; deletion invokes the
    // attached buffer cleanup while the canvas and its descriptor are still live.
    unsafe {
        rs_lvgl_sys::lv_obj_delete(canvas.raw().as_ptr());
    }
}

#[test]
fn shared_rgb888_resource_retains_pixels_through_mount_and_release() {
    use std::rc::Rc;
    struct Loaded(rs_lvgl_ui::ImageResource);
    impl App for Loaded {
        type Action = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            image_resource(self.0.clone())
        }
    }
    let _guard = common::lock();
    let display = common::init();
    let pixels: Rc<[u8]> = Rc::from(&BGR.0[..]);
    let weak = Rc::downgrade(&pixels);
    let source = rs_lvgl_ui::ImageResource::rgb888(3, 2, 12, pixels).unwrap();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    ui.mount(&Loaded(source.clone())).unwrap();
    drop(source);
    assert!(weak.upgrade().is_some());
    assert_eq!(&ui.snapshot().unwrap().pixels[..4], &[255, 0, 0, 255]);
    ui.unmount();
    assert!(weak.upgrade().is_none());
}
