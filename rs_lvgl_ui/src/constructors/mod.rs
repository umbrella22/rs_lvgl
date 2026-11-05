//! Ordinary descriptor constructors grouped by function.

mod controls;
mod custom;
mod data;
mod layout;
mod media;

/// Build a reusable style patch with all properties initially unset.
///
/// Ordinary views initialize their own embedded style. Use flat modifiers such
/// as `column([]).gap(12)` for local properties and this constructor when sharing
/// a style or targeting a part/state selector. No native object is created.
pub fn style() -> crate::Style {
    crate::Style::default()
}
#[cfg(feature = "widget-arc")]
pub use controls::arc;
#[cfg(feature = "widget-bar")]
pub use controls::bar;
pub use controls::button;
#[cfg(feature = "widget-buttonmatrix")]
pub use controls::button_matrix;
pub use controls::button_static;
#[cfg(feature = "widget-checkbox")]
pub use controls::checkbox;
#[cfg(feature = "widget-dropdown")]
pub use controls::dropdown;
#[cfg(feature = "widget-keyboard")]
pub use controls::keyboard;
pub use controls::label;
pub use controls::label_static;
#[cfg(feature = "widget-ime-pinyin")]
pub use controls::pinyin_keyboard;
#[cfg(feature = "widget-roller")]
pub use controls::roller;
#[cfg(feature = "widget-slider")]
pub use controls::slider;
#[cfg(feature = "widget-spinbox")]
pub use controls::spinbox;
#[cfg(feature = "widget-switch")]
pub use controls::switch;
#[cfg(feature = "widget-textarea")]
pub use controls::textarea;
pub use custom::custom;
pub use custom::raw;
#[cfg(feature = "widget-arclabel")]
pub use data::arc_label;
#[cfg(feature = "widget-calendar")]
pub use data::calendar;
#[cfg(feature = "widget-chart")]
pub use data::chart;
#[cfg(feature = "widget-led")]
pub use data::led;
#[cfg(feature = "widget-line")]
pub use data::line;
#[cfg(feature = "widget-msgbox")]
pub use data::msgbox;
#[cfg(feature = "widget-scale")]
pub use data::scale;
#[cfg(feature = "widget-span")]
pub use data::spans;
#[cfg(feature = "widget-spinner")]
pub use data::spinner;
#[cfg(feature = "widget-table")]
pub use data::table;
pub use layout::column;
pub use layout::grid;
pub use layout::obj;
pub use layout::row;
#[cfg(feature = "widget-tabview")]
pub use layout::tabview;
#[cfg(feature = "widget-tileview")]
pub use layout::tileview;
pub use layout::{content, percent, px};
#[cfg(feature = "widget-animimage")]
pub use media::anim_image;
#[cfg(feature = "widget-barcode")]
pub use media::barcode;
#[cfg(feature = "widget-canvas")]
pub use media::canvas;
#[cfg(feature = "widget-gif")]
pub use media::gif;
#[cfg(feature = "widget-image")]
pub use media::image;
#[cfg(feature = "widget-imagebutton")]
pub use media::image_button;
#[cfg(feature = "widget-image")]
pub use media::image_resource;
#[cfg(feature = "widget-image")]
pub use media::image_resource_scaled;
#[cfg(feature = "widget-image")]
pub use media::image_scaled;
#[cfg(feature = "widget-lottie")]
pub use media::lottie;
#[cfg(feature = "widget-qrcode")]
pub use media::qrcode;
