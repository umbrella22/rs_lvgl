mod common;
use rs_lvgl_ui::{App, Canvas, Color, DrawCommand, Font, Node, Theme, Ui, UiError};
const RED: Color = Color::rgb(255, 0, 0);
const BLUE: Color = Color::rgb(0, 0, 255);
struct Drawing(Canvas);
impl App for Drawing {
    type Msg = ();
    fn update(&mut self, _: ()) {}
    fn view(&self, _: &Theme) -> Node<()> {
        Node::canvas(self.0.clone())
    }
}
#[test]
fn redraw_and_resize_preserve_native_object_and_snapshot_survives_teardown() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Drawing(
        Canvas::new(17, 13)
            .background(BLUE, 255)
            .draw(DrawCommand::Rect {
                x: 2,
                y: 3,
                width: 4,
                height: 5,
                radius: 0,
                color: RED,
                opacity: 255,
            }),
    );
    ui.mount(&app).unwrap();
    let obj = common::app_root();
    unsafe {
        let pixel = rs_lvgl_sys::lv_canvas_get_px(obj.as_ptr(), 3, 4);
        assert_eq!(
            (pixel.red, pixel.green, pixel.blue, pixel.alpha),
            (255, 0, 0, 255)
        );
        let buf = rs_lvgl_sys::lv_canvas_get_draw_buf(obj.as_ptr());
        assert!((*buf).header.stride() >= 17 * 4);
    }
    ui.update(&app).unwrap();
    app.0 = Canvas::new(19, 7)
        .background(RED, 255)
        .draw(DrawCommand::Pixel {
            x: 18,
            y: 6,
            color: BLUE,
            opacity: 255,
        });
    ui.update(&app).unwrap();
    assert_eq!(common::app_root(), obj);
    let snapshot = ui.snapshot().unwrap();
    assert_eq!((snapshot.width, snapshot.height), (19, 7));
    assert_eq!(&snapshot.pixels[..4], &[255, 0, 0, 255]);
    assert_eq!(
        &snapshot.pixels[snapshot.pixels.len() - 4..],
        &[0, 0, 255, 255]
    );
    ui.unmount();
    unsafe {
        rs_lvgl::deinit();
    }
    drop(ui);
    assert_eq!(snapshot.pixels.len(), 19 * 7 * 4);
}
#[test]
fn text_and_line_finish_drawing_before_local_storage_is_released() {
    let _guard = common::lock();
    for _ in 0..3 {
        let display = common::init();
        let mut ui = Ui::new(&display, common::theme()).unwrap();
        let app = Drawing(
            Canvas::new(50, 40)
                .background(BLUE, 255)
                .draw(DrawCommand::Text {
                    x: 0,
                    y: 0,
                    width: 50,
                    height: 20,
                    text: "Canvas".into(),
                    font: Font::default(),
                    color: RED,
                    opacity: 255,
                })
                .draw(DrawCommand::Line {
                    from: (0, 30),
                    to: (40, 30),
                    width: 3,
                    color: RED,
                    opacity: 255,
                }),
        );
        ui.mount(&app).unwrap();
        let snapshot = ui.snapshot().unwrap();
        assert!(snapshot.pixels[..50 * 20 * 4]
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[0] > 0));
        assert_eq!(
            &snapshot.pixels[(30 * 50 + 10) * 4..(30 * 50 + 10) * 4 + 4],
            &[255, 0, 0, 255]
        );
        ui.unmount();
    }
}
#[test]
fn invalid_dimensions_stride_and_commands_reject_before_mutation() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    ui.mount(&Drawing(Canvas::new(8, 8).background(RED, 255)))
        .unwrap();
    let obj = common::app_root();
    for frame in [
        Canvas::new(0, 5),
        Canvas::new(u16::MAX, 1),
        Canvas::new(5, 5).draw(DrawCommand::Pixel {
            x: 5,
            y: 0,
            color: RED,
            opacity: 255,
        }),
    ] {
        assert_eq!(ui.update(&Drawing(frame)), Err(UiError::InvalidProperty));
        assert_eq!(common::app_root(), obj);
    }
}
