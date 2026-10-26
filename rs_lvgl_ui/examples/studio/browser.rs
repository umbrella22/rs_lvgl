//! Single-threaded Emscripten adapter for the shared component studio.
//! JavaScript owns the frame loop and copies pixels before the next native call.
use super::*;
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
    time::Duration,
};

#[derive(Clone, Copy, Default)]
struct Pointer {
    x: i32,
    y: i32,
    pressed: bool,
}

#[derive(Default)]
struct Input {
    pointer: Cell<Pointer>,
    pointers: RefCell<VecDeque<Pointer>>,
    key: Cell<(u32, bool)>,
    keys: RefCell<VecDeque<(u32, bool)>>,
}

struct BrowserStudio {
    app: Studio,
    ui: Option<Ui<Msg>>,
    display: Option<rs_lvgl::Display>,
    mouse: Option<NonNull<rs_lvgl_sys::lv_indev_t>>,
    keyboard: Option<NonNull<rs_lvgl_sys::lv_indev_t>>,
    input: Box<Input>,
    pixels: Rc<RefCell<Box<[u8]>>>,
    generation: Rc<Cell<u32>>,
}

impl BrowserStudio {
    fn new() -> Result<Self, String> {
        rs_lvgl::init();
        let mut state = Self {
            app: Studio::default(),
            ui: None,
            display: None,
            mouse: None,
            keyboard: None,
            input: Box::default(),
            pixels: Rc::new(RefCell::new(
                vec![0; (WIDTH * HEIGHT * 4) as usize].into_boxed_slice(),
            )),
            generation: Rc::new(Cell::new(0)),
        };
        let pixels = state.pixels.clone();
        let generation = state.generation.clone();
        state.display = Some(
            rs_lvgl::Display::register::<_, 10240>(WIDTH as u32, HEIGHT as u32, move |refresh| {
                let mut pixels = pixels.borrow_mut();
                let area = &refresh.area;
                let row_width = (area.x2 - area.x1 + 1) as usize;
                for y in area.y1..=area.y2 {
                    for x in area.x1..=area.x2 {
                        let color = refresh.colors
                            [(y - area.y1) as usize * row_width + (x - area.x1) as usize];
                        let offset = ((y * WIDTH + x) * 4) as usize;
                        pixels[offset..offset + 4].copy_from_slice(&[
                            color.r(),
                            color.g(),
                            color.b(),
                            255,
                        ]);
                    }
                }
                generation.set(generation.get().wrapping_add(1));
            })
            .map_err(|error| error.to_string())?,
        );
        state.ui = Some(
            Ui::new(state.display.as_ref().unwrap(), palette(false)).map_err(|e| e.to_string())?,
        );
        let ui = state.ui.as_mut().unwrap();
        apply_theme(ui, &state.app);
        ui.mount(&state.app).map_err(|e| e.to_string())?;
        // SAFETY: devices and their stable boxed user data belong to this runtime.
        // Callbacks run synchronously on this thread and are removed before the box.
        unsafe {
            let data = (&mut *state.input as *mut Input).cast();
            let mouse =
                NonNull::new(rs_lvgl_sys::lv_indev_create()).ok_or("pointer allocation failed")?;
            state.mouse = Some(mouse);
            rs_lvgl_sys::lv_indev_set_type(
                mouse.as_ptr(),
                rs_lvgl_sys::lv_indev_type_t_LV_INDEV_TYPE_POINTER,
            );
            rs_lvgl_sys::lv_indev_set_user_data(mouse.as_ptr(), data);
            rs_lvgl_sys::lv_indev_set_read_cb(mouse.as_ptr(), Some(read_pointer));
            let keyboard =
                NonNull::new(rs_lvgl_sys::lv_indev_create()).ok_or("keyboard allocation failed")?;
            state.keyboard = Some(keyboard);
            rs_lvgl_sys::lv_indev_set_type(
                keyboard.as_ptr(),
                rs_lvgl_sys::lv_indev_type_t_LV_INDEV_TYPE_KEYPAD,
            );
            rs_lvgl_sys::lv_indev_set_user_data(keyboard.as_ptr(), data);
            rs_lvgl_sys::lv_indev_set_read_cb(keyboard.as_ptr(), Some(read_key));
            ui.bind_input(keyboard).map_err(|e| e.to_string())?;
        }
        Ok(state)
    }

    fn frame(&mut self, elapsed: u32) -> Result<(), UiError> {
        rs_lvgl::tick_inc(Duration::from_millis(elapsed.into()));
        let ui = self.ui.as_mut().unwrap();
        let dark = self.app.dark;
        ui.timer_handler();
        ui.drain(&mut self.app, 128)?;
        if dark != self.app.dark {
            apply_theme(ui, &self.app);
            ui.update(&self.app)?;
        }
        ui.tick(elapsed)?;
        if self.app.toasts.advance(elapsed) {
            ui.update(&self.app)?;
        }
        Ok(())
    }
}

impl Drop for BrowserStudio {
    fn drop(&mut self) {
        if let Some(mut ui) = self.ui.take() {
            ui.unmount();
            drop(ui);
        }
        // SAFETY: no frame/callback is active. Ui is gone, inputs are deleted
        // before their boxed data/display, and all CPU flushes finish synchronously.
        unsafe {
            for device in [self.mouse.take(), self.keyboard.take()]
                .into_iter()
                .flatten()
            {
                rs_lvgl_sys::lv_indev_delete(device.as_ptr());
            }
            if let Some(display) = self.display.take() {
                display.delete();
            }
            rs_lvgl::deinit();
        }
    }
}

unsafe extern "C" fn read_pointer(
    device: *mut rs_lvgl_sys::lv_indev_t,
    data: *mut rs_lvgl_sys::lv_indev_data_t,
) {
    // SAFETY: LVGL invokes this only for a registered live device; user_data
    // points to its owner's Input box and data is writable for this callback.
    unsafe {
        let input = &*rs_lvgl_sys::lv_indev_get_user_data(device).cast::<Input>();
        let mut queue = input.pointers.borrow_mut();
        if let Some(pointer) = queue.pop_front() {
            input.pointer.set(pointer);
        }
        let pointer = input.pointer.get();
        (*data).point.x = pointer.x;
        (*data).point.y = pointer.y;
        (*data).state = u32::from(pointer.pressed);
        (*data).continue_reading = !queue.is_empty();
    }
}

unsafe extern "C" fn read_key(
    device: *mut rs_lvgl_sys::lv_indev_t,
    data: *mut rs_lvgl_sys::lv_indev_data_t,
) {
    // SAFETY: identical ownership and callback lifetime to read_pointer above.
    unsafe {
        let input = &*rs_lvgl_sys::lv_indev_get_user_data(device).cast::<Input>();
        let mut queue = input.keys.borrow_mut();
        if let Some(key) = queue.pop_front() {
            input.key.set(key);
        }
        let (key, pressed) = input.key.get();
        (*data).key = key;
        (*data).state = u32::from(pressed);
        (*data).continue_reading = !queue.is_empty();
    }
}

thread_local! {
    static RUNTIME: RefCell<Option<BrowserStudio>> = const { RefCell::new(None) };
}

#[no_mangle]
pub extern "C" fn studio_init() -> i32 {
    RUNTIME.with_borrow_mut(|slot| {
        if slot.is_some() {
            return -1;
        }
        match BrowserStudio::new() {
            Ok(runtime) => {
                *slot = Some(runtime);
                0
            }
            Err(error) => {
                eprintln!("Studio initialization: {error}");
                -1
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn studio_destroy() {
    RUNTIME.with_borrow_mut(|slot| {
        slot.take();
    });
}

#[no_mangle]
pub extern "C" fn studio_frame(elapsed: u32) -> i32 {
    if elapsed > 1000 {
        return -1;
    }
    RUNTIME.with_borrow_mut(|slot| match slot {
        Some(state) => match state.frame(elapsed) {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("Studio frame: {error}");
                -1
            }
        },
        None => -1,
    })
}

/// Read-only RGBA storage; valid until studio_destroy, never retained across calls.
#[no_mangle]
pub extern "C" fn studio_pixels() -> *const u8 {
    RUNTIME.with_borrow(|slot| {
        slot.as_ref()
            .map_or(core::ptr::null(), |state| state.pixels.borrow().as_ptr())
    })
}

#[no_mangle]
pub extern "C" fn studio_pointer(x: i32, y: i32, pressed: u32) -> i32 {
    if !(0..WIDTH).contains(&x) || !(0..HEIGHT).contains(&y) || pressed > 1 {
        return -1;
    }
    RUNTIME.with_borrow(|slot| match slot {
        Some(state) => {
            state.input.pointers.borrow_mut().push_back(Pointer {
                x,
                y,
                pressed: pressed == 1,
            });
            0
        }
        None => -1,
    })
}

#[no_mangle]
pub extern "C" fn studio_key(key: u32, pressed: u32) -> i32 {
    if char::from_u32(key).is_none() || pressed > 1 {
        return -1;
    }
    RUNTIME.with_borrow(|slot| match slot {
        Some(state) => {
            state.input.keys.borrow_mut().push_back((key, pressed == 1));
            0
        }
        None => -1,
    })
}

/// Browser toolbar commands are queued through the same application message path.
#[no_mangle]
pub extern "C" fn studio_command(command: u32, value: u32) -> i32 {
    let message = match command {
        0 if (value as usize) < PAGES.len() => Msg::Page(value as usize),
        1 => Msg::Theme,
        2 => Msg::Reset,
        _ => return -1,
    };
    RUNTIME.with_borrow_mut(|slot| match slot {
        Some(state) => state.ui.as_mut().unwrap().post(message).map_or(-1, |_| 0),
        None => -1,
    })
}

/// Small observable state surface shared by accessibility text and acceptance.
#[no_mangle]
pub extern "C" fn studio_state(field: u32) -> i32 {
    RUNTIME.with_borrow(|slot| match slot {
        Some(state) => match field {
            0 => state.app.page as i32,
            1 => state.app.value,
            2 => state.app.clicks as i32,
            3 => i32::from(state.app.enabled),
            4 => i32::from(state.app.dark),
            5 => i32::from(state.app.dialog),
            6 => state.app.text.len() as i32,
            7 => state.generation.get() as i32,
            8 => i32::from(state.app.quit),
            _ => -1,
        },
        None => -1,
    })
}
