//! Native fixtures shared by tests of the actual example views.
#![allow(dead_code)]

use core::ptr::NonNull;
use rs_lvgl::Display;
use rs_lvgl_ui::{Snapshot, Ui};
use std::{ffi::CStr, sync::Mutex, time::Duration};

pub static LOCK: Mutex<()> = Mutex::new(());
pub type Obj = *mut rs_lvgl_sys::lv_obj_t;

// Bindgen intentionally omits class variables; these const declarations match
// the vendored native headers and are used only to identify live test objects.
unsafe extern "C" {
    pub static lv_label_class: rs_lvgl_sys::lv_obj_class_t;
    pub static lv_tabview_class: rs_lvgl_sys::lv_obj_class_t;
    pub static lv_calendar_class: rs_lvgl_sys::lv_obj_class_t;
    pub static lv_keyboard_class: rs_lvgl_sys::lv_obj_class_t;
    pub static lv_textarea_class: rs_lvgl_sys::lv_obj_class_t;
    pub static lv_checkbox_class: rs_lvgl_sys::lv_obj_class_t;
}

pub fn init(width: u32, height: u32) -> Display {
    // SAFETY: each test holds LOCK and drops its UI/devices before returning.
    unsafe { rs_lvgl::deinit() };
    rs_lvgl::init();
    Display::register::<_, { 960 * 16 }>(width, height, |_| {}).unwrap()
}

pub fn screen() -> Obj {
    unsafe { rs_lvgl_sys::lv_screen_active() }
}

pub fn top() -> Obj {
    unsafe { rs_lvgl_sys::lv_display_get_layer_top(rs_lvgl_sys::lv_display_get_default()) }
}

pub fn child(parent: Obj, index: i32) -> Obj {
    let obj = unsafe { rs_lvgl_sys::lv_obj_get_child(parent, index) };
    assert!(!obj.is_null());
    obj
}

pub fn root() -> Obj {
    child(child(screen(), 0), 0)
}

pub fn find(root: Obj, predicate: &impl Fn(Obj) -> bool) -> Option<Obj> {
    if predicate(root) {
        return Some(root);
    }
    for i in 0..unsafe { rs_lvgl_sys::lv_obj_get_child_count(root) } {
        if let Some(found) = find(child(root, i as i32), predicate) {
            return Some(found);
        }
    }
    None
}

pub fn class(root: Obj, class: *const rs_lvgl_sys::lv_obj_class_t) -> Obj {
    find(root, &|obj| unsafe {
        rs_lvgl_sys::lv_obj_check_type(obj, class)
    })
    .unwrap()
}

pub fn label(root: Obj, text: &str) -> Obj {
    find(root, &|obj| unsafe {
        rs_lvgl_sys::lv_obj_check_type(obj, &raw const lv_label_class)
            && CStr::from_ptr(rs_lvgl_sys::lv_label_get_text(obj)).to_bytes() == text.as_bytes()
    })
    .unwrap_or_else(|| panic!("missing label {text:?}"))
}

pub fn parent(obj: Obj) -> Obj {
    unsafe { rs_lvgl_sys::lv_obj_get_parent(obj) }
}

pub fn rect(obj: Obj) -> [i32; 4] {
    unsafe {
        rs_lvgl_sys::lv_obj_update_layout(obj);
        let mut area = core::mem::zeroed();
        rs_lvgl_sys::lv_obj_get_coords(obj, &mut area);
        [area.x1, area.y1, area.x2, area.y2]
    }
}

pub fn settle<M: Clone + 'static>(ui: &mut Ui<M>) {
    for _ in 0..30 {
        rs_lvgl::tick_inc(Duration::from_millis(10));
        ui.timer_handler();
    }
}

pub fn capture(obj: Obj, name: &str) -> Snapshot {
    // SAFETY: tests supply live objects on the locked native UI thread. The
    // snapshot completes drawing; copy each aligned row before freeing it.
    let image = unsafe {
        rs_lvgl_sys::lv_obj_update_layout(obj);
        let buf = NonNull::new(rs_lvgl_sys::lv_snapshot_take(
            obj,
            rs_lvgl_sys::lv_color_format_t_LV_COLOR_FORMAT_ARGB8888,
        ))
        .expect("native snapshot");
        let raw = buf.as_ref();
        let (width, height, stride) = (raw.header.w(), raw.header.h(), raw.header.stride());
        assert!(width * 4 <= stride);
        assert!(stride * height <= raw.data_size);
        let bytes = core::slice::from_raw_parts(raw.data, raw.data_size as usize);
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height as usize {
            for pixel in
                bytes[y * stride as usize..y * stride as usize + width as usize * 4].chunks_exact(4)
            {
                pixels.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
            }
        }
        rs_lvgl_sys::lv_draw_buf_destroy(buf.as_ptr());
        Snapshot {
            width,
            height,
            pixels,
        }
    };
    if let Some(dir) = std::env::var_os("LVGL_AUDIT_DIR") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut ppm = format!("P6\n{} {}\n255\n", image.width, image.height).into_bytes();
        for pixel in image.pixels.chunks_exact(4) {
            ppm.extend_from_slice(&pixel[..3]);
        }
        std::fs::write(dir.join(format!("{name}.ppm")), ppm).unwrap();
    }
    image
}

pub fn pixel(image: &Snapshot, x: i32, y: i32) -> &[u8] {
    assert!(x >= 0 && y >= 0 && x < image.width as i32 && y < image.height as i32);
    let offset = (y as usize * image.width as usize + x as usize) * 4;
    &image.pixels[offset..offset + 4]
}

pub fn outline_changed(before: &Snapshot, after: &Snapshot, obj: Obj) {
    let [x1, y1, x2, y2] = rect(obj);
    let (width, pad) = unsafe {
        (
            rs_lvgl_sys::lv_obj_get_style_outline_width(obj, 0),
            rs_lvgl_sys::lv_obj_get_style_outline_pad(obj, 0),
        )
    };
    assert!(width > 0);
    let distance = pad + (width + 1) / 2;
    for (side, x, y) in [
        ("left", x1 - distance, (y1 + y2) / 2),
        ("right", x2 + distance, (y1 + y2) / 2),
        ("top", (x1 + x2) / 2, y1 - distance),
        ("bottom", (x1 + x2) / 2, y2 + distance),
    ] {
        let a = pixel(before, x, y);
        let b = pixel(after, x, y);
        assert!(
            a[..3].iter().zip(&b[..3]).any(|(a, b)| a.abs_diff(*b) > 20),
            "{side} focus outline clipped at ({x}, {y}): before={a:?}, after={b:?}"
        );
    }
}

#[derive(Default)]
struct Sample {
    x: i32,
    y: i32,
    down: bool,
}

pub struct Pointer {
    raw: NonNull<rs_lvgl_sys::lv_indev_t>,
    sample: Box<Sample>,
}
impl Pointer {
    pub fn new() -> Self {
        unsafe extern "C" fn read(
            indev: *mut rs_lvgl_sys::lv_indev_t,
            data: *mut rs_lvgl_sys::lv_indev_data_t,
        ) {
            // SAFETY: the owned Box stays allocated until the native indev is deleted.
            unsafe {
                let sample = &*rs_lvgl_sys::lv_indev_get_user_data(indev).cast::<Sample>();
                (*data).point.x = sample.x;
                (*data).point.y = sample.y;
                (*data).state = if sample.down {
                    rs_lvgl_sys::lv_indev_state_t_LV_INDEV_STATE_PRESSED
                } else {
                    rs_lvgl_sys::lv_indev_state_t_LV_INDEV_STATE_RELEASED
                };
            }
        }
        let mut sample = Box::<Sample>::default();
        let raw = NonNull::new(unsafe { rs_lvgl_sys::lv_indev_create() }).unwrap();
        unsafe {
            rs_lvgl_sys::lv_indev_set_type(
                raw.as_ptr(),
                rs_lvgl_sys::lv_indev_type_t_LV_INDEV_TYPE_POINTER,
            );
            rs_lvgl_sys::lv_indev_set_user_data(raw.as_ptr(), (&mut *sample as *mut Sample).cast());
            rs_lvgl_sys::lv_indev_set_read_cb(raw.as_ptr(), Some(read));
        }
        Self { raw, sample }
    }
    pub fn read(&mut self, x: i32, y: i32, down: bool) {
        *self.sample = Sample { x, y, down };
        unsafe { rs_lvgl_sys::lv_indev_read(self.raw.as_ptr()) };
    }
    pub fn click(&mut self, obj: Obj) {
        unsafe { rs_lvgl_sys::lv_obj_scroll_to_view_recursive(obj, false) };
        let [x1, y1, x2, y2] = rect(obj);
        self.read((x1 + x2) / 2, (y1 + y2) / 2, true);
        self.read((x1 + x2) / 2, (y1 + y2) / 2, false);
    }
}
impl Drop for Pointer {
    fn drop(&mut self) {
        unsafe { rs_lvgl_sys::lv_indev_delete(self.raw.as_ptr()) };
    }
}

#[cfg(feature = "input-keypad")]
pub mod keys {
    use super::*;
    use rs_lvgl::input_device::{Keypad, KeypadInputData};
    use std::{cell::Cell, rc::Rc};
    pub struct Keyboard {
        pub device: Keypad,
        sample: Rc<Cell<KeypadInputData>>,
    }
    impl Keyboard {
        pub fn new(display: &Display) -> Self {
            let sample = Rc::new(Cell::new(KeypadInputData::released(0)));
            let input = sample.clone();
            Self {
                device: Keypad::register(move || input.get(), display).unwrap(),
                sample,
            }
        }
        pub fn send(&self, key: u32) {
            for data in [
                KeypadInputData::pressed(key),
                KeypadInputData::released(key),
            ] {
                self.sample.set(data);
                unsafe { rs_lvgl_sys::lv_indev_read(self.device.raw().unwrap().as_ptr()) };
            }
        }
    }
}
