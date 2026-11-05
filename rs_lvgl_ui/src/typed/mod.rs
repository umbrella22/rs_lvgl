//! Widget types retained during construction, before conversion to [`crate::View`].
//!
//! Common styles and action mapping retain the widget type. Single-child
//! composition accepts a typed node directly; heterogeneous arrays use
//! `.into_view()` on their elements. Erased views keep the existing runtime
//! property validation contract.
mod constructors;
pub use constructors::*;

mod private {
    pub trait Sealed {}
}

/// A descriptor whose concrete widget type has been erased.
pub struct Erased;
impl private::Sealed for Erased {}

/// Internal property capability bounds; applications use the constructors.
#[doc(hidden)]
pub mod capability {
    pub struct Text;
    pub struct Numeric;
    pub struct Toggle;
    pub struct Selection;
    pub struct Cell;
    pub struct Date;
    pub struct Children;
    pub struct Roller;
    pub struct Spinbox;
    pub struct Spinner;
    pub struct Led;
    pub struct Table;
    pub struct Msgbox;
    pub struct Calendar;
    pub struct ImageButton;
    pub struct AnimImage;
}

/// Sealed capability relation used by typed descriptor modifiers.
#[doc(hidden)]
pub trait Supports<Property>: private::Sealed {}
impl<Property> Supports<Property> for Erased {}

/// Zero-sized widget markers. These carry no runtime widget state.
pub mod kind {
    use super::{capability::*, private, Supports};
    macro_rules! kinds {
        ($($(#[$attribute:meta])* $name:ident: [$($property:ty),*];)*) => {$(
            $(#[$attribute])*
            pub struct $name;
            $(#[$attribute])*
            const _: () = {
                impl private::Sealed for $name {}
                impl<M> From<crate::Node<M, $name>> for crate::View<M> {
                    fn from(node: crate::Node<M, $name>) -> Self { node.into_view() }
                }
                $(impl Supports<$property> for $name {})*
            };
        )*};
    }
    kinds! {
        Obj: [Children]; Row: [Children]; Column: [Children]; Grid: [Children];
        Label: []; Button: [];
        #[cfg(feature = "widget-switch")] Switch: [Toggle];
        #[cfg(feature = "widget-slider")] Slider: [Numeric];
        #[cfg(feature = "widget-bar")] Bar: [Numeric];
        #[cfg(feature = "widget-arc")] Arc: [Numeric];
        #[cfg(feature = "widget-checkbox")] Checkbox: [Toggle];
        #[cfg(feature = "widget-textarea")] Textarea: [Text];
        #[cfg(feature = "widget-dropdown")] Dropdown: [Selection];
        #[cfg(feature = "widget-roller")] Roller: [Selection, super::capability::Roller];
        #[cfg(feature = "widget-spinbox")] Spinbox: [Numeric, super::capability::Spinbox];
        #[cfg(feature = "widget-buttonmatrix")] ButtonMatrix: [Selection];
        #[cfg(feature = "widget-tabview")] Tabview: [Children, Selection];
        #[cfg(feature = "widget-tileview")] Tileview: [Children, Selection];
        #[cfg(feature = "widget-calendar")] Calendar: [Date, super::capability::Calendar];
        #[cfg(feature = "widget-table")] Table: [Cell, super::capability::Table];
        #[cfg(feature = "widget-msgbox")] Msgbox: [Children, Selection, super::capability::Msgbox];
        #[cfg(feature = "widget-keyboard")] Keyboard: [];
        #[cfg(feature = "widget-ime-pinyin")] PinyinKeyboard: [];
        #[cfg(feature = "widget-scale")] Scale: [];
        #[cfg(feature = "widget-chart")] Chart: [];
        #[cfg(feature = "widget-span")] Spans: [];
        #[cfg(feature = "widget-arclabel")] ArcLabel: [];
        #[cfg(feature = "widget-line")] Line: [];
        #[cfg(feature = "widget-spinner")] Spinner: [super::capability::Spinner];
        #[cfg(feature = "widget-led")] Led: [super::capability::Led];
        #[cfg(feature = "widget-image")] Image: [];
        #[cfg(feature = "widget-imagebutton")] ImageButton: [super::capability::ImageButton];
        #[cfg(feature = "widget-animimage")] AnimImage: [super::capability::AnimImage];
        #[cfg(feature = "widget-canvas")] Canvas: [];
        #[cfg(feature = "widget-qrcode")] QrCode: [];
        #[cfg(feature = "widget-barcode")] Barcode: [];
        #[cfg(feature = "widget-gif")] Gif: [];
        #[cfg(feature = "widget-lottie")] Lottie: [];
    }
}
