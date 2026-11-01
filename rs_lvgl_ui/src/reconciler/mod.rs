//! Mounted state and entry points for native tree reconciliation.

mod children;
mod interaction;
mod keyboard;
mod mount;
mod mounted;
mod styles;
mod update;

pub(crate) use keyboard::{bind_keyboards, release_keyboards};
pub(crate) use mount::create;
pub(crate) use mounted::delete_subtree;
pub(crate) use update::update;

use crate::custom::{CustomObject, CustomWidgetKind};
use crate::events::HandlerSlot;
use crate::node::{GridCell, Key, Selector};
use crate::theme::Style;
use crate::widgets::{self, KindTag, Parts, Props};
use alloc::{boxed::Box, vec::Vec};
use core::ptr::NonNull;
use rs_lvgl_sys::lv_obj_t;

#[derive(Default)]
pub(crate) struct Counters {
    pub(crate) created: u64,
    pub(crate) deleted: u64,
    pub(crate) moved: u64,
    /// Keyboard → textarea bindings applied to native keyboards.
    ///
    /// LVGL's keyboard does not observe its textarea's deletion, so every
    /// subtree deletion unbinds keyboards whose textarea disappears before
    /// the native objects are freed.
    pub(crate) keyboards: Vec<(NonNull<lv_obj_t>, NonNull<lv_obj_t>)>,
}

/// The lifecycle state of one mounted custom widget.
pub(crate) struct MountedCustom {
    entry: CustomWidgetKind,
    object: CustomObject,
}

pub(crate) struct MountedNode<M: Clone + 'static> {
    pub(crate) obj: NonNull<lv_obj_t>,
    pub(crate) key: Key,
    tag: KindTag,
    props: Props,
    style: Style,
    font_resource: Option<crate::FontResource>,
    transitions: Vec<crate::animation::NativeTransition>,
    animations: Vec<crate::Animation>,
    slot: Option<NonNull<HandlerSlot<M>>>,
    parts: Parts,
    grid_cell: Option<GridCell>,
    part_styles: Vec<(Selector, Style)>,
    keyboard_id: Option<u64>,
    clickable: Option<bool>,
    scrollable: Option<bool>,
    pub(crate) interaction: crate::scroll::Interaction,
    #[cfg(feature = "input-keypad")]
    pub(crate) focusable: Option<bool>,
    #[cfg(feature = "input-keypad")]
    pub(crate) focus_id: u64,
    #[cfg(feature = "input-keypad")]
    pub(crate) modal: bool,
    #[cfg(feature = "input-gridnav")]
    pub(crate) grid_navigation: Option<crate::GridNavigation>,
    #[cfg(feature = "input-gridnav")]
    pub(crate) grid_state: Option<crate::gridnav::GridState>,
    #[cfg(feature = "input-gridnav")]
    pub(crate) grid_child_focusable: Option<bool>,
    grid_columns: Option<Box<[i32]>>,
    grid_rows: Option<Box<[i32]>>,
    retained: widgets::Retained,
    /// Page roots of a tabview or tileview, one per child.
    pages: Option<Vec<NonNull<lv_obj_t>>>,
    custom: Option<MountedCustom>,
    pub(crate) children: Vec<MountedNode<M>>,
    /// Number of app-level nodes in this subtree, excluding implicit labels.
    pub(crate) count: usize,
    /// Number of lifecycle custom widgets in this subtree; `tick` skips
    /// subtrees without any.
    pub(crate) customs: usize,
}
