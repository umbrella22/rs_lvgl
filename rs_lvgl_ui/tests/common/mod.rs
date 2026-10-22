#![allow(dead_code)]

//! Shared helpers for the host integration tests.
//!
//! LVGL owns global native state, so every test holds [`lock`] for its whole
//! duration and calls [`init`] to start from a clean display.

use core::ptr::NonNull;
use std::sync::{Mutex, MutexGuard};

use rs_lvgl::Display;
use rs_lvgl_sys::lv_obj_t;

static LOCK: Mutex<()> = Mutex::new(());

pub fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn init() -> Display {
    // SAFETY: the lock guarantees that no other test uses LVGL, and every
    // test drops its `Ui` before returning.
    unsafe { rs_lvgl::deinit() };
    rs_lvgl::init();
    Display::register::<_, { 64 * 64 }>(64, 64, |_| {}).expect("memory display registration")
}

pub fn screen() -> NonNull<lv_obj_t> {
    // SAFETY: a display was registered by `init`.
    let raw = unsafe { rs_lvgl_sys::lv_screen_active() };
    NonNull::new(raw).expect("active screen")
}

pub fn child(parent: NonNull<lv_obj_t>, index: i32) -> NonNull<lv_obj_t> {
    // SAFETY: the caller passes a live parent and a valid child index.
    let raw = unsafe { rs_lvgl_sys::lv_obj_get_child(parent.as_ptr(), index) };
    NonNull::new(raw).expect("child object")
}

/// The application tree root: the first child of `Ui`'s root container.
pub fn app_root() -> NonNull<lv_obj_t> {
    child(child(screen(), 0), 0)
}

pub fn theme() -> rs_lvgl_ui::Theme {
    rs_lvgl_ui::Theme::dark()
}
