//! Driver utilities for LVGL.
//!
//! The `lv_drivers` C library was removed during the v9 migration.
//! To create display and input drivers, use [`crate::Display::register`] and
//! [`crate::input_device::pointer::Pointer::register`] directly.
//!
//! # Example
//!
//! ```no_run
//! use rs_lvgl::Display;
//!
//! const HOR_RES: u32 = 240;
//! const VER_RES: u32 = 240;
//!
//! let display = Display::register::<_, { (HOR_RES * VER_RES / 10) as usize }>(
//!     HOR_RES,
//!     VER_RES,
//!     |refresh| {
//!         // Send refreshed pixels to your hardware display
//!     },
//! )
//! .unwrap();
//! ```

// NOTE: The lv_drivers C submodule was removed in the v9 migration.
// Users should implement display and input drivers directly using the
// Display::register and input_device APIs.
