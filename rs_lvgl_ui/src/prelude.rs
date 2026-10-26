//! Common types and ordinary constructors for declarative UI code.
//!
//! ```
//! use rs_lvgl_ui::prelude::*;
//!
//! let view: View<()> = column([
//!     label("Hello"),
//!     row([button("Save").on_click(())]),
//! ]);
//! ```
//!
//! Optional constructors and types follow the crate's feature selection. An
//! import from this module does not enable an optional widget or font.

#[cfg(feature = "components")]
pub use crate::components;
pub use crate::constructors::*;
#[cfg(feature = "widget-gif")]
pub use crate::Gif;
#[cfg(feature = "widget-lottie")]
pub use crate::Lottie;
#[cfg(feature = "widget-qrcode")]
pub use crate::QrCode;
pub use crate::{
    symbol, table_cell, Align, Animation, AnimationProperty, App, BlendMode, CalendarDate,
    ChartSeries, ChartType, Color, CustomObject, CustomWidget, CustomWidgetKind, DrainStats,
    Easing, Event, FlexAlign, Font, GestureDirection, GradientDirection, GridAlign, GridCell,
    GridTrack, ImageSource, InputEvent, Key, LabelLongMode, NativeObject, Obj, Part, PointerEvent,
    RawImage, RollerMode, ScreenEffect, ScreenTransition, Screens, ScrollDir, ScrollOffset,
    ScrollSnap, ScrollbarMode, Size, Snapshot, Span, State, Style, StyleTransition, TextAlign,
    TextDecoration, Theme, Tile, TransitionProperty, Ui, UiError, UiStats, View,
    ANIM_IMAGE_MAX_FRAMES, ANIM_REPEAT_INFINITE, SPINBOX_MAX_DIGITS, TABLE_MAX_CELLS_PER_AXIS,
};
#[cfg(feature = "widget-barcode")]
pub use crate::{Barcode, BarcodeEncoding};
#[cfg(feature = "widget-canvas")]
pub use crate::{Canvas, DrawCommand};
#[cfg(feature = "input-gridnav")]
pub use crate::{GridNavigation, NavigationAxis};
