//! LVGL font handling logic and helper structures.
//!
//! Fonts can be built into LVGL or custom-generated with the [official online
//! converter].
//!
//! # Built-in fonts
//! LVGL offers several fonts for Latin (only up to ASCII), Arabic, Persian,
//! and Hebrew, along with CJK glyphs. Fonts enabled in the vendored
//! `lv_conf.h` (e.g. `montserrat_14`) are usable on stable Rust through
//! their `rs_lvgl_sys::lv_font_*` symbols (see the example below); the
//! `Font::montserrat_*()` helper constructors additionally require the
//! `nightly` feature:
//! ```ignore
//! use rs_lvgl::font::Font;
//! use rs_lvgl::style::Style;
//!
//! fn main() {
//!     let mut my_style = Style::default();
//!     my_style.set_text_font(Font::montserrat_48());
//!     // Use the style
//! }
//! ```
//!
//! # Custom fonts
//! Custom fonts converted to a C source file (e.g. with the [official online
//! converter]) can be compiled into `rs_lvgl_sys` by dropping the file into its
//! `shims/` directory and declaring the symbol in `shims/rs_lvgl_sys.h`. The
//! symbol can then unsafely be converted into a `Font` struct, exactly like
//! the built-in font in this example:
//! ```
//! use rs_lvgl::font::Font;
//! use rs_lvgl::style::Style;
//!
//! let montserrat_14 = unsafe {
//!     Font::new_raw(rs_lvgl_sys::lv_font_montserrat_14)
//! };
//! let mut my_style = Style::default();
//! my_style.set_text_font(montserrat_14);
//! // Use the style
//! ```
//! This operation is inherently unsafe as it instantiates and uses arbitrary
//! data structures that the Rust compiler can't check.
//!
//! [official online converter]: https://lvgl.io/tools/fontconverter

mod generic;
pub use generic::*;

#[cfg(feature = "nightly")]
mod builtin;
