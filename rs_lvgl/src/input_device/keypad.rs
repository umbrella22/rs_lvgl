//! Typed keypad input. Native deletion releases the reader and invalidates
//! the Rust handle; ordinary handle drop deletes the native input device.

use alloc::{boxed::Box, rc::Rc};
use core::{
    cell::{Cell, RefCell},
    ptr::NonNull,
};

use crate::{Display, LvError, LvResult};

/// LVGL navigation keys. Other `u32` values pass through unchanged (for text
/// input, LVGL accepts Unicode code points). NEXT/PREV move group focus;
/// arrow keys operate the focused widget, and ENTER activates it.
pub mod key {
    pub const UP: u32 = rs_lvgl_sys::lv_key_t_LV_KEY_UP;
    pub const DOWN: u32 = rs_lvgl_sys::lv_key_t_LV_KEY_DOWN;
    pub const RIGHT: u32 = rs_lvgl_sys::lv_key_t_LV_KEY_RIGHT;
    pub const LEFT: u32 = rs_lvgl_sys::lv_key_t_LV_KEY_LEFT;
    pub const ESC: u32 = rs_lvgl_sys::lv_key_t_LV_KEY_ESC;
    pub const DEL: u32 = rs_lvgl_sys::lv_key_t_LV_KEY_DEL;
    pub const BACKSPACE: u32 = rs_lvgl_sys::lv_key_t_LV_KEY_BACKSPACE;
    pub const ENTER: u32 = rs_lvgl_sys::lv_key_t_LV_KEY_ENTER;
    pub const NEXT: u32 = rs_lvgl_sys::lv_key_t_LV_KEY_NEXT;
    pub const PREV: u32 = rs_lvgl_sys::lv_key_t_LV_KEY_PREV;
    pub const HOME: u32 = rs_lvgl_sys::lv_key_t_LV_KEY_HOME;
    pub const END: u32 = rs_lvgl_sys::lv_key_t_LV_KEY_END;
}

/// One keypad sample. A release carries the same key as its preceding press.
/// Set `continue_reading` only when another buffered sample is ready.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeypadInputData {
    pub key: u32,
    pub pressed: bool,
    pub continue_reading: bool,
}

impl KeypadInputData {
    pub const fn pressed(key: u32) -> Self {
        Self {
            key,
            pressed: true,
            continue_reading: false,
        }
    }

    pub const fn released(key: u32) -> Self {
        Self {
            key,
            pressed: false,
            continue_reading: false,
        }
    }

    pub const fn and_continued(mut self) -> Self {
        self.continue_reading = true;
        self
    }
}

type Descriptor = Rc<Cell<Option<NonNull<rs_lvgl_sys::lv_indev_t>>>>;

struct Reader<F> {
    handler: RefCell<F>,
    descriptor: Descriptor,
}

/// A keypad registered on one display, confined to the LVGL UI thread.
///
/// Drop it before LVGL deinitialization. Native input deletion also releases
/// its reader exactly once; afterwards [`Keypad::raw`] returns `None`.
/// This uses a typed reader rather than the legacy `InputDriver` callback,
/// which can return samples for a different device type.
pub struct Keypad {
    descriptor: Descriptor,
}

impl Keypad {
    /// Register a nonblocking reader on `display`.
    ///
    /// The reader must not call back into LVGL, delete devices, or panic.
    /// Panics in an `extern "C"` callback abort, never unwind through C.
    /// The closure must own its captures because LVGL retains it.
    ///
    /// ```compile_fail,E0373
    /// use rs_lvgl::input_device::{Keypad, KeypadInputData};
    /// let display = rs_lvgl::Display::default();
    /// let key = 123;
    /// let device = Keypad::register(|| KeypadInputData::released(key), &display);
    /// ```
    pub fn register<F>(handler: F, display: &Display) -> LvResult<Self>
    where
        F: FnMut() -> KeypadInputData + 'static,
    {
        let descriptor = Rc::new(Cell::new(None));
        // Allocate the Rust reader before the native device so an allocator
        // failure cannot strand a native device with no cleanup hook.
        let reader = Box::new(Reader {
            handler: RefCell::new(handler),
            descriptor: descriptor.clone(),
        });
        // SAFETY: the platform initializes LVGL and calls on its UI thread.
        unsafe {
            let indev = NonNull::new(rs_lvgl_sys::lv_indev_create()).ok_or(LvError::LvOOMemory)?;
            let reader = Box::into_raw(reader);
            let delete = rs_lvgl_sys::lv_indev_add_event_cb(
                indev.as_ptr(),
                Some(delete_reader::<F>),
                rs_lvgl_sys::lv_event_code_t_LV_EVENT_DELETE,
                reader.cast(),
            );
            if delete.is_null() {
                rs_lvgl_sys::lv_indev_delete(indev.as_ptr());
                drop(Box::from_raw(reader));
                return Err(LvError::LvOOMemory);
            }
            descriptor.set(Some(indev));
            rs_lvgl_sys::lv_indev_set_display(indev.as_ptr(), display.disp.as_ptr());
            rs_lvgl_sys::lv_indev_set_type(
                indev.as_ptr(),
                rs_lvgl_sys::lv_indev_type_t_LV_INDEV_TYPE_KEYPAD,
            );
            rs_lvgl_sys::lv_indev_set_user_data(indev.as_ptr(), reader.cast());
            rs_lvgl_sys::lv_indev_set_read_cb(indev.as_ptr(), Some(read_keypad::<F>));
        }
        Ok(Self { descriptor })
    }

    /// The live native input device, or `None` after native deletion.
    /// Using the pointer requires preserving the device's reader/user data
    /// and deletion hook. Never delete it during one of its callbacks.
    pub fn raw(&self) -> Option<NonNull<rs_lvgl_sys::lv_indev_t>> {
        self.descriptor.get()
    }
}

impl Drop for Keypad {
    fn drop(&mut self) {
        if let Some(indev) = self.descriptor.take() {
            // SAFETY: the deletion hook clears this cell on external deletion.
            // The UI thread contract excludes concurrent reads or deletion.
            unsafe { rs_lvgl_sys::lv_indev_delete(indev.as_ptr()) };
        }
    }
}

/// # Safety
/// LVGL supplies a live device, writable sample and its registered Reader<F>.
unsafe extern "C" fn read_keypad<F: FnMut() -> KeypadInputData>(
    indev: *mut rs_lvgl_sys::lv_indev_t,
    data: *mut rs_lvgl_sys::lv_indev_data_t,
) {
    // SAFETY: the native delete hook retains the reader throughout this call.
    let reader = unsafe { &*rs_lvgl_sys::lv_indev_get_user_data(indev).cast::<Reader<F>>() };
    let Ok(mut handler) = reader.handler.try_borrow_mut() else {
        return;
    };
    let sample = handler();
    // SAFETY: `data` is the writable output allocated by the native caller.
    unsafe {
        (*data).key = sample.key;
        (*data).state = if sample.pressed {
            rs_lvgl_sys::lv_indev_state_t_LV_INDEV_STATE_PRESSED
        } else {
            rs_lvgl_sys::lv_indev_state_t_LV_INDEV_STATE_RELEASED
        };
        (*data).continue_reading = sample.continue_reading;
    }
}

/// # Safety
/// The device invokes DELETE once with the Box<Reader<F>> registered above.
unsafe extern "C" fn delete_reader<F>(event: *mut rs_lvgl_sys::lv_event_t) {
    // SAFETY: this callback uniquely releases the native-owned box. Invalidate
    // the handle before dropping captures, so it cannot double-delete later.
    let reader =
        unsafe { Box::from_raw(rs_lvgl_sys::lv_event_get_user_data(event).cast::<Reader<F>>()) };
    reader.descriptor.set(None);
    drop(reader);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keypad_preserves_samples_display_and_native_cleanup() {
        crate::tests::initialize_test(true);
        let display = Display::default();
        let other = Display::register::<_, 64>(8, 8, |_| {}).unwrap();
        let drops = Rc::new(Cell::new(0));
        struct Dropped(Rc<Cell<usize>>);
        impl Drop for Dropped {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }
        for external_delete in [false, true] {
            let capture = Dropped(drops.clone());
            let mut pressed = true;
            let keypad = Keypad::register(
                move || {
                    let _keep = &capture;
                    let sample = if pressed {
                        KeypadInputData::pressed(0xFEDC_BA98).and_continued()
                    } else {
                        KeypadInputData::released(0xFEDC_BA98)
                    };
                    pressed = false;
                    sample
                },
                &display,
            )
            .unwrap();
            let raw = keypad.raw().unwrap().as_ptr();
            unsafe {
                assert_eq!(
                    rs_lvgl_sys::lv_indev_get_display(raw),
                    display.disp.as_ptr()
                );
                assert_ne!(rs_lvgl_sys::lv_indev_get_display(raw), other.disp.as_ptr());
                let read = rs_lvgl_sys::lv_indev_get_read_cb(raw).unwrap();
                let mut data = rs_lvgl_sys::lv_indev_data_t::default();
                read(raw, &mut data);
                assert_eq!(data.key, 0xFEDC_BA98);
                assert_eq!(
                    data.state,
                    rs_lvgl_sys::lv_indev_state_t_LV_INDEV_STATE_PRESSED
                );
                assert!(data.continue_reading);
                read(raw, &mut data);
                assert_eq!(data.key, 0xFEDC_BA98);
                assert_eq!(
                    data.state,
                    rs_lvgl_sys::lv_indev_state_t_LV_INDEV_STATE_RELEASED
                );
                assert!(!data.continue_reading);
                if external_delete {
                    rs_lvgl_sys::lv_indev_delete(raw);
                    assert!(keypad.raw().is_none());
                }
            }
            drop(keypad);
        }
        assert_eq!(drops.get(), 2);
    }
}
