//! Exhaustive dispatch of native property updates by widget family.

mod base;
#[cfg(any(
    feature = "widget-switch",
    feature = "widget-checkbox",
    feature = "widget-dropdown",
    feature = "widget-roller",
    feature = "widget-buttonmatrix"
))]
mod choices;
mod common;
#[cfg(any(
    feature = "widget-scale",
    feature = "widget-chart",
    feature = "widget-table",
    feature = "widget-calendar",
    feature = "widget-spinner",
    feature = "widget-led"
))]
mod data;
#[cfg(any(
    feature = "widget-ime-pinyin",
    feature = "widget-qrcode",
    feature = "widget-barcode",
    feature = "widget-gif",
    feature = "widget-lottie",
    feature = "widget-canvas"
))]
mod extensions;
#[cfg(any(
    feature = "widget-image",
    feature = "widget-imagebutton",
    feature = "widget-animimage",
    feature = "widget-line"
))]
mod media;
#[cfg(feature = "widget-msgbox")]
mod messagebox;
#[cfg(any(
    feature = "widget-slider",
    feature = "widget-bar",
    feature = "widget-arc",
    feature = "widget-spinbox"
))]
mod ranges;
#[cfg(any(
    feature = "widget-textarea",
    feature = "widget-span",
    feature = "widget-arclabel"
))]
mod text;
use super::{Parts, Props, Retained};
use crate::events::HandlerSlot;
use crate::UiError;
use core::ptr::NonNull;
use rs_lvgl_sys::lv_obj_t;

/// Apply kind properties, writing only values that changed.
///
/// Interactive values (slider, bar, arc, switch, checkbox, dropdown and
/// textarea text) are controlled: they are compared with the native widget
/// rather than with the previous descriptor, so a value the user changed in
/// LVGL is reset to the application's value on the next render.
pub(crate) fn apply_props<M: Clone + 'static>(
    obj: NonNull<lv_obj_t>,
    parts: Parts,
    new: &Props,
    old: Option<&Props>,
    slot: Option<&HandlerSlot<M>>,
    retained: &mut Retained,
) -> Result<(), UiError> {
    match new {
        Props::Container | Props::Row | Props::Column | Props::Grid { .. } | Props::Custom => {
            Ok(())
        }
        #[cfg(feature = "widget-tabview")]
        Props::Tabview { .. } => Ok(()),
        #[cfg(feature = "widget-tileview")]
        Props::Tileview { .. } => Ok(()),
        #[cfg(feature = "widget-keyboard")]
        Props::Keyboard { .. } => Ok(()),
        Props::Label { .. } => base::apply(obj, new, old, parts),
        Props::Button { .. } => base::apply(obj, new, old, parts),
        #[cfg(feature = "widget-slider")]
        Props::Slider { .. } => ranges::apply(obj, new, old, slot),
        #[cfg(feature = "widget-bar")]
        Props::ProgressBar { .. } => ranges::apply(obj, new, old, slot),
        #[cfg(feature = "widget-arc")]
        Props::Arc { .. } => ranges::apply(obj, new, old, slot),
        #[cfg(feature = "widget-spinbox")]
        Props::Spinbox { .. } => ranges::apply(obj, new, old, slot),
        #[cfg(feature = "widget-switch")]
        Props::Switch { .. } => choices::apply(obj, new, old, slot, retained),
        #[cfg(feature = "widget-checkbox")]
        Props::Checkbox { .. } => choices::apply(obj, new, old, slot, retained),
        #[cfg(feature = "widget-dropdown")]
        Props::Dropdown { .. } => choices::apply(obj, new, old, slot, retained),
        #[cfg(feature = "widget-roller")]
        Props::Roller { .. } => choices::apply(obj, new, old, slot, retained),
        #[cfg(feature = "widget-buttonmatrix")]
        Props::ButtonMatrix { .. } => choices::apply(obj, new, old, slot, retained),
        #[cfg(feature = "widget-scale")]
        Props::Scale { .. } => data::apply(obj, new, old, retained),
        #[cfg(feature = "widget-chart")]
        Props::Chart { .. } => data::apply(obj, new, old, retained),
        #[cfg(feature = "widget-table")]
        Props::Table { .. } => data::apply(obj, new, old, retained),
        #[cfg(feature = "widget-calendar")]
        Props::Calendar { .. } => data::apply(obj, new, old, retained),
        #[cfg(feature = "widget-spinner")]
        Props::Spinner { .. } => data::apply(obj, new, old, retained),
        #[cfg(feature = "widget-led")]
        Props::Led { .. } => data::apply(obj, new, old, retained),
        #[cfg(feature = "widget-image")]
        Props::Image { .. } => media::apply(obj, new, old, retained),
        #[cfg(feature = "widget-imagebutton")]
        Props::ImageButton { .. } => media::apply(obj, new, old, retained),
        #[cfg(feature = "widget-animimage")]
        Props::AnimatedImage { .. } => media::apply(obj, new, old, retained),
        #[cfg(feature = "widget-line")]
        Props::Line { .. } => media::apply(obj, new, old, retained),
        #[cfg(feature = "widget-msgbox")]
        Props::MessageBox { .. } => messagebox::apply(obj, new, old, parts),
        #[cfg(feature = "widget-ime-pinyin")]
        Props::PinyinKeyboard { .. } => extensions::apply(obj, new, old, parts, retained),
        #[cfg(feature = "widget-qrcode")]
        Props::QrCode { .. } => extensions::apply(obj, new, old, parts, retained),
        #[cfg(feature = "widget-barcode")]
        Props::Barcode { .. } => extensions::apply(obj, new, old, parts, retained),
        #[cfg(feature = "widget-gif")]
        Props::Gif { .. } => extensions::apply(obj, new, old, parts, retained),
        #[cfg(feature = "widget-lottie")]
        Props::Lottie { .. } => extensions::apply(obj, new, old, parts, retained),
        #[cfg(feature = "widget-canvas")]
        Props::Canvas { .. } => extensions::apply(obj, new, old, parts, retained),
        #[cfg(feature = "widget-textarea")]
        Props::Textarea { .. } => text::apply(obj, new, old, slot),
        #[cfg(feature = "widget-span")]
        Props::Spans { .. } => text::apply(obj, new, old, slot),
        #[cfg(feature = "widget-arclabel")]
        Props::ArcLabel { .. } => text::apply(obj, new, old, slot),
    }
}

/// Suppress feedback from one synchronous programmatic write.
/// The native slot remains attached to its live object for the entire call.
pub(crate) fn suppress<M: Clone + 'static, R>(
    slot: Option<&HandlerSlot<M>>,
    write: impl FnOnce() -> R,
) -> R {
    if let Some(slot) = slot {
        slot.suppress();
        let result = write();
        slot.clear_suppress();
        result
    } else {
        write()
    }
}
