//! Native widget adapters: projection, creation, properties, layout and styling.

mod create;
mod kind;
mod layout;
mod pages;
mod properties;
mod props;
mod retained;
mod style;

pub(crate) use create::{create, event_inner_target, remove_main_theme};
pub use kind::KindTag;
pub(crate) use kind::{tag, value_source};
pub(crate) use layout::{apply_grid_cell, grid_descriptors, set_grid_descriptors};
#[cfg(feature = "widget-tabview")]
pub(crate) use pages::{set_tabview_active, set_tabview_titles, tabview_pages};
#[cfg(feature = "widget-tileview")]
pub(crate) use pages::{set_tileview_active, tileview_is_active, tileview_pages};
pub(crate) use properties::{apply_props, suppress};
pub(crate) use props::{into_props, needs_recreate, retain_static_text, Props};
pub(crate) use retained::{Parts, Retained};
pub(crate) use style::{apply_style, apply_style_for_selector, clear_selector_style};
