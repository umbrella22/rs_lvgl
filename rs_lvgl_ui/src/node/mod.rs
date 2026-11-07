//! Pure descriptor tree and its internal construction defaults.

mod builders;
mod handlers;
mod kind;
mod mapping;
mod types;
mod validation;

pub(crate) use handlers::HandlerSet;
pub use handlers::{EventHandler, Handlers, TextChangedHandler, ValueChangedHandler};
pub use kind::WidgetKind;
pub(crate) use types::Selector;
pub use types::{
    table_cell, Align, CalendarDate, Color, FlexAlign, Font, GridAlign, GridCell, GridTrack,
    ImageSource, ItemAlign, NativePart, RawImage, RollerMode, ScrollDirection, Size, Span, State,
    TableCell, TextAlign, TextOverflow, Tile, ANIMATED_IMAGE_MAX_FRAMES, ANIMATION_REPEAT_INFINITE,
    SPINBOX_MAX_DIGITS, TABLE_MAX_CELLS_PER_AXIS,
};
pub(crate) use validation::validate;

use crate::Style;
use alloc::vec::Vec;

/// A child identity used for sibling reconciliation.
///
/// `Auto` children match by position among `Auto` siblings. `Named` and `Id`
/// children match by value, which preserves native object identity across
/// inserts, removals and reorders.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Key {
    #[default]
    Auto,
    Named(&'static str),
    Id(u64),
}

/// The backing descriptor type for [`crate::View`].
///
/// Constructing a descriptor never touches LVGL. Application and component
/// functions return [`crate::View`] and use ordinary constructors such as
/// [`crate::column`], [`crate::label`] and [`crate::button`].
pub struct Node<M, K = crate::typed::Erased> {
    pub(crate) marker: core::marker::PhantomData<fn() -> K>,
    #[cfg(any(feature = "diagnostics", feature = "strict-properties"))]
    invalid_property: Option<crate::Property>,
    pub(crate) static_text: Option<&'static str>,
    pub(crate) font_resource: Option<crate::FontResource>,
    #[cfg(feature = "widget-image")]
    pub(crate) image_resource: Option<crate::ImageResource>,
    pub(crate) key: Key,
    pub(crate) kind: WidgetKind,
    pub(crate) style: Style,
    pub(crate) flex_flow: (bool, bool),
    pub(crate) part_styles: Vec<(Selector, Style)>,
    pub(crate) keyboard_id: Option<u64>,
    pub(crate) grid_cell: Option<GridCell>,
    pub(crate) children: Vec<Node<M>>,
    pub(crate) handlers: HandlerSet<M>,
    pub(crate) clickable: Option<bool>,
    pub(crate) scrollable: Option<bool>,
    pub(crate) checked: Option<bool>,
    pub(crate) interaction: crate::scroll::Interaction,
    pub(crate) animations: Vec<crate::Animation>,
    #[cfg(feature = "input-keypad")]
    pub(crate) focusable: Option<bool>,
    #[cfg(feature = "input-keypad")]
    pub(crate) modal: bool,
    #[cfg(feature = "input-gridnav")]
    pub(crate) grid_navigation: Option<crate::GridNavigation>,
}

impl<M, K> Node<M, K> {
    fn record_invalid_property(&mut self, property: crate::Property) {
        #[cfg(any(feature = "diagnostics", feature = "strict-properties"))]
        {
            self.invalid_property.get_or_insert(property);
        }
        #[cfg(not(any(feature = "diagnostics", feature = "strict-properties")))]
        let _ = property;
    }

    pub(crate) fn new(kind: WidgetKind) -> Self {
        let style = match &kind {
            WidgetKind::Container
            | WidgetKind::Row
            | WidgetKind::Column
            | WidgetKind::Grid { .. }
            | WidgetKind::Label { .. } => Style::plain(),
            #[cfg(feature = "widget-span")]
            WidgetKind::Spans { .. } => Style::plain(),
            #[cfg(feature = "widget-arclabel")]
            WidgetKind::ArcLabel { .. } => Style::plain(),
            _ => Style::default(),
        };
        Self {
            marker: core::marker::PhantomData,
            #[cfg(any(feature = "diagnostics", feature = "strict-properties"))]
            invalid_property: None,
            static_text: None,
            font_resource: None,
            #[cfg(feature = "widget-image")]
            image_resource: None,
            key: Key::Auto,
            kind,
            style,
            flex_flow: (false, false),
            part_styles: Vec::new(),
            keyboard_id: None,
            grid_cell: None,
            children: Vec::new(),
            handlers: HandlerSet::default(),
            clickable: None,
            scrollable: None,
            checked: None,
            interaction: crate::scroll::Interaction::default(),
            animations: Vec::new(),
            #[cfg(feature = "input-keypad")]
            focusable: None,
            #[cfg(feature = "input-keypad")]
            modal: false,
            #[cfg(feature = "input-gridnav")]
            grid_navigation: None,
        }
    }

    /// Erase the widget type at a composition or component return boundary.
    /// Moves the existing descriptor without allocating or creating native state.
    pub fn into_view(self) -> crate::View<M> {
        self.cast_kind()
    }

    pub(crate) fn cast_kind<L>(self) -> Node<M, L> {
        Node {
            marker: core::marker::PhantomData,
            #[cfg(any(feature = "diagnostics", feature = "strict-properties"))]
            invalid_property: self.invalid_property,
            static_text: self.static_text,
            font_resource: self.font_resource,
            #[cfg(feature = "widget-image")]
            image_resource: self.image_resource,
            key: self.key,
            kind: self.kind,
            style: self.style,
            flex_flow: self.flex_flow,
            part_styles: self.part_styles,
            keyboard_id: self.keyboard_id,
            grid_cell: self.grid_cell,
            children: self.children,
            handlers: self.handlers,
            clickable: self.clickable,
            scrollable: self.scrollable,
            checked: self.checked,
            interaction: self.interaction,
            animations: self.animations,
            #[cfg(feature = "input-keypad")]
            focusable: self.focusable,
            #[cfg(feature = "input-keypad")]
            modal: self.modal,
            #[cfg(feature = "input-gridnav")]
            grid_navigation: self.grid_navigation,
        }
    }
}
