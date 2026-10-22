#[cfg(feature = "widget-qrcode")]
mod qrcode;
#[cfg(feature = "widget-qrcode")]
pub use qrcode::QrCode;
#[cfg(feature = "widget-barcode")]
mod barcode;
#[cfg(feature = "widget-barcode")]
pub use barcode::{Barcode, BarcodeEncoding};
#[cfg(feature = "widget-gif")]
mod gif;
#[cfg(feature = "widget-gif")]
pub use gif::Gif;
#[cfg(feature = "widget-lottie")]
mod lottie;
#[cfg(feature = "widget-lottie")]
pub use lottie::Lottie;

#[cfg(feature = "widget-ime-pinyin")]
pub(crate) mod pinyin;
