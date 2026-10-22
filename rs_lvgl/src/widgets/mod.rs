use crate::lv_core::obj::NativeObject;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[cfg(lv_use_arc)]
pub use self::arc::ArcMode;
#[cfg(lv_use_bar)]
pub use self::bar::{BarMode, BarOrientation};
#[cfg(lv_use_slider)]
pub use self::slider::{SliderMode, SliderOrientation};

#[cfg(lv_use_arc)]
mod arc;
#[cfg(lv_use_bar)]
mod bar;
#[cfg(lv_use_keyboard)]
mod keyboard;
mod label;
#[cfg(lv_use_slider)]
mod slider;
#[cfg(lv_use_table)]
mod table;
