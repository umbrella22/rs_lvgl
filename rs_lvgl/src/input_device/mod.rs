use crate::LvResult;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Data {
    Pointer(PointerInputData),
    Encoder(EncoderInputData),
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum InputState {
    Pressed(Data),
    Released(Data),
}

impl InputState {
    pub fn once(self) -> BufferStatus {
        BufferStatus::Once(self)
    }

    pub fn and_continued(self) -> BufferStatus {
        BufferStatus::Buffered(self)
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum BufferStatus {
    Once(InputState),
    Buffered(InputState),
}

pub trait InputDriver<D> {
    fn register<F>(handler: F, display: &crate::Display) -> LvResult<D>
    where
        F: Fn() -> BufferStatus;

    fn get_descriptor(&self) -> Option<*mut rs_lvgl_sys::lv_indev_t>;

    unsafe fn set_descriptor(&mut self, descriptor: *mut rs_lvgl_sys::lv_indev_t) -> LvResult<()>;
}

mod encoder;
#[cfg(feature = "input-keypad")]
mod keypad;
mod pointer;

pub use encoder::*;
#[cfg(feature = "input-keypad")]
pub use keypad::*;
pub use pointer::*;
