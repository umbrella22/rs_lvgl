//! Declarative, typed Rust UI layer for LVGL 9.6.
//!
//! `rs_lvgl_ui` adds a retained, declarative composition layer above the
//! imperative [`rs_lvgl`] crate. An application describes a screen as a
//! [`View<Msg>`] tree, changes one application state value through messages,
//! and lets [`Ui`] create, update, reorder and delete the native LVGL objects.
//!
//! # Runtime rule
//!
//! One UI thread has exclusive access to LVGL. C event callbacks only enqueue
//! messages. [`App::update`] changes application state, [`App::view`] returns a
//! new descriptor tree, and the reconciler is the only code that mutates LVGL
//! objects:
//!
//! ```text
//! platform event -> callback -> EventQueue -> Ui::drain -> App::update
//!   -> App::view -> View tree -> reconciler -> LVGL objects
//! ```
//!
//! # Example
//!
//! ```
//! use rs_lvgl_ui::prelude::*;
//!
//! #[derive(Clone)]
//! enum Msg { Increment }
//!
//! struct Counter { value: u32 }
//!
//! impl App for Counter {
//!     type Msg = Msg;
//!
//!     fn update(&mut self, _: Msg) {
//!         self.value += 1;
//!     }
//!
//!     fn view(&self, theme: &Theme) -> View<Msg> {
//!         column([
//!             label(format!("Count: {}", self.value)),
//!             button("Add one")
//!                 .key(Key::Named("increment"))
//!                 .style(Style::accent(theme))
//!                 .on_click(Msg::Increment),
//!         ])
//!         .style(Style::default().gap(12))
//!     }
//! }
//! ```
//!
//! # Feature and platform contract
//!
//! The crate is `no_std` and requires only `alloc`. It never registers displays,
//! input devices, timers or RTOS tasks. The platform supplies an initialized
//! LVGL instance and a [`rs_lvgl::Display`], calls [`Ui::timer_handler`] at its
//! chosen cadence, calls [`Ui::drain`] when input or external events arrive,
//! and calls [`Ui::tick`] from the render loop to advance custom widgets.
//! Target builds that cannot use the `ctor`-based auto-initialization
//! enable the `no-autoinit` feature and call `rs_lvgl::init()` themselves.
//!
//! `Ui<M>` is neither `Send` nor `Sync`:
//!
//! ```compile_fail
//! fn require_send<T: Send>() {}
//! require_send::<rs_lvgl_ui::Ui<()>>();
//! ```
//!
//! # Lifecycle contract
//!
//! Call [`Ui::unmount`] before deleting the native display, before
//! `rs_lvgl::deinit`, and before dropping the [`rs_lvgl::Display`] used to create the
//! `Ui`. Dropping a `Ui` also deletes its root container; it must not be the
//! last live LVGL user during display teardown.

#![cfg_attr(not(test), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]
// Helpers shared by several widgets become unused when only some of them are
// enabled; the full widget set keeps every lint.
#![cfg_attr(
    not(feature = "widgets-all"),
    allow(dead_code, unused_imports, unused_variables, unused_unsafe)
)]

extern crate alloc;

mod app;
mod constructors;
pub use constructors::*;
#[cfg(feature = "components")]
pub mod components;
mod custom;
mod events;
#[cfg(feature = "input-keypad")]
mod focus;
#[cfg(feature = "input-gridnav")]
mod gridnav;
pub mod prelude;
#[cfg(feature = "input-gridnav")]
pub use gridnav::{GridNavigation, NavigationAxis};
mod extensions;
mod node;
mod reconciler;
mod scroll;
pub use extensions::*;
mod graphics;
pub use graphics::Snapshot;
#[cfg(feature = "widget-canvas")]
mod canvas;
#[cfg(feature = "widget-canvas")]
pub use canvas::{Canvas, DrawCommand};
mod screens;
pub use screens::{ScreenEffect, ScreenTransition, Screens};
pub mod symbol;
mod theme;
mod widgets;

pub use app::{App, DrainStats, Ui, UiStats};
pub use custom::{CustomObject, CustomWidget, CustomWidgetKind};
pub use events::{Event, GestureDirection, InputEvent, PointerEvent, ScrollOffset};
pub use node::{
    table_cell, Align, CalendarDate, ChartSeries, ChartType, Color, EventHandler, FlexAlign, Font,
    GridAlign, GridCell, GridTrack, Handlers, ImageSource, Key, LabelLongMode, Node, Part,
    RawImage, RollerMode, ScrollDir, Size, Span, State, TextAlign, TextChangedHandler, Tile,
    ValueChangedHandler, WidgetKind, ANIM_IMAGE_MAX_FRAMES, ANIM_REPEAT_INFINITE,
    SPINBOX_MAX_DIGITS, TABLE_MAX_CELLS_PER_AXIS,
};
pub use scroll::{ScrollSnap, ScrollbarMode};
mod animation;
pub use animation::{Animation, AnimationProperty, Easing, StyleTransition, TransitionProperty};
mod style_properties;
pub use style_properties::{BlendMode, GradientDirection, TextDecoration};
pub use theme::{Style, Theme};

/// A declarative view description returned by [`App::view`] and component functions.
///
/// [`Node`] is the backing descriptor type; `View` is an alias for that same type.
/// Use ordinary
/// constructors such as [`column()`], [`label()`] and [`button()`] to build views;
/// constructing a view does not create any native LVGL objects.
pub type View<M> = Node<M>;

// Re-exported so custom widgets can implement [`CustomWidget`] without adding
// a direct `rs_lvgl` dependency.
pub use rs_lvgl::{NativeObject, Obj};

use core::fmt;

/// Errors reported by the declarative UI layer.
///
/// Descriptor validation runs before the reconciler mutates the native tree.
/// Native media decoding can also report `InvalidProperty` during mount or
/// update. Native failures delete the affected partial subtree before returning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiError {
    /// Two siblings use the same `Key::Named` or `Key::Id` value.
    DuplicateKey,
    /// A property cannot be represented safely or violates its range.
    InvalidProperty,
    /// LVGL could not allocate an object or an event registration.
    Allocation,
    /// A mutating `Ui` method was called while a reconcile was in progress.
    Reentrant,
    /// The display has no active screen.
    DisplayUnavailable,
}

impl fmt::Display for UiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            UiError::DuplicateKey => "duplicate sibling key",
            UiError::InvalidProperty => "invalid widget property",
            UiError::Allocation => "LVGL allocation failed",
            UiError::Reentrant => "reentrant UI mutation",
            UiError::DisplayUnavailable => "display has no active screen",
        };
        f.write_str(text)
    }
}

#[cfg(feature = "nightly")]
impl core::error::Error for UiError {}
