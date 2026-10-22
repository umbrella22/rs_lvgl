mod common;

use core::ptr;
use std::cell::RefCell;
use std::rc::Rc;

use common::lock;
use rs_lvgl::Display;
use rs_lvgl_ui::{App, Color, Key, Node, Size, Style, Theme, Ui};

const WIDTH: usize = 64;
const HEIGHT: usize = 64;
const EXPECTED_HASH: u64 = 0x6fa9_6bde_8d31_e465;

type Buffer = Rc<RefCell<Vec<u8>>>;

fn init_capture() -> (Display, Buffer) {
    let buffer: Buffer = Rc::new(RefCell::new(vec![0u8; WIDTH * HEIGHT * 3]));
    let capture = Rc::clone(&buffer);
    // SAFETY: the test lock serializes LVGL access and the previous test's
    // display has been dropped.
    unsafe { rs_lvgl::deinit() };
    rs_lvgl::init();
    let display =
        Display::register::<_, { WIDTH * HEIGHT }>(WIDTH as u32, HEIGHT as u32, move |refresh| {
            let width = (refresh.area.x2 - refresh.area.x1 + 1) as usize;
            let height = (refresh.area.y2 - refresh.area.y1 + 1) as usize;
            let mut pixels = capture.borrow_mut();
            for y in 0..height {
                for x in 0..width {
                    let color = refresh.colors[y * width + x];
                    let index =
                        ((refresh.area.y1 as usize + y) * WIDTH + refresh.area.x1 as usize + x) * 3;
                    pixels[index] = color.r();
                    pixels[index + 1] = color.g();
                    pixels[index + 2] = color.b();
                }
            }
        })
        .expect("capturing display registration");
    (display, buffer)
}

struct SnapshotApp;

impl App for SnapshotApp {
    type Msg = ();

    fn update(&mut self, _: ()) {}

    fn view(&self, _: &Theme) -> Node<()> {
        let block = |id: u64, color: Color| {
            Node::obj().key(Key::Id(id)).style(
                Style::default()
                    .bg(color)
                    .width(Size::Px(20))
                    .height(Size::Px(20)),
            )
        };
        Node::column()
            .child(block(0, Color::rgb(255, 0, 0)))
            .child(block(1, Color::rgb(0, 255, 0)))
            .child(block(2, Color::rgb(0, 0, 255)))
    }
}

fn refresh() {
    // SAFETY: the display and its buffer are alive for the whole test.
    unsafe {
        rs_lvgl_sys::lv_obj_invalidate(rs_lvgl_sys::lv_screen_active());
        rs_lvgl_sys::lv_refr_now(ptr::null_mut());
    }
}

fn hash(pixels: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in pixels {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[test]
fn static_region_has_stable_pixels() {
    let _guard = lock();
    let (display, buffer) = init_capture();
    let mut ui = Ui::new(&display, Theme::dark()).unwrap();
    let app = SnapshotApp;

    ui.mount(&app).unwrap();
    refresh();

    {
        let pixels = buffer.borrow();
        let pixel = |x: usize, y: usize| {
            let index = (y * WIDTH + x) * 3;
            (pixels[index], pixels[index + 1], pixels[index + 2])
        };
        assert_eq!(pixel(0, 0), (255, 0, 0));
        assert_eq!(pixel(0, 20), (0, 255, 0));
        assert_eq!(pixel(0, 40), (0, 0, 255));
        assert_eq!(pixel(0, 60), (245, 245, 245));
    }

    let first = hash(&buffer.borrow());
    ui.update(&app).unwrap();
    refresh();
    let second = hash(&buffer.borrow());
    assert_eq!(first, second, "re-rendering the same tree must be stable");
    println!("snapshot hash: 0x{first:016x}");
    assert_eq!(
        first, EXPECTED_HASH,
        "snapshot hash changed; verify the rendering change and update EXPECTED_HASH"
    );

    ui.unmount();
    assert_eq!(ui.stats().mounted_nodes, 0);
    assert_eq!(
        unsafe { rs_lvgl_sys::lv_obj_get_child_count(rs_lvgl_sys::lv_screen_active()) },
        0
    );
}
