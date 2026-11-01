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
    table_cell, Align, CalendarDate, ChartSeries, ChartType, Color, FlexAlign, Font, GridAlign,
    GridCell, GridTrack, ImageSource, LabelLongMode, Part, RawImage, RollerMode, ScrollDir, Size,
    Span, State, TextAlign, Tile, ANIM_IMAGE_MAX_FRAMES, ANIM_REPEAT_INFINITE, SPINBOX_MAX_DIGITS,
    TABLE_MAX_CELLS_PER_AXIS,
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
pub struct Node<M> {
    #[cfg(any(feature = "diagnostics", feature = "strict-properties"))]
    invalid_property: Option<crate::Property>,
    pub(crate) static_text: Option<&'static str>,
    pub(crate) font_resource: Option<crate::FontResource>,
    #[cfg(feature = "widget-image")]
    pub(crate) image_resource: Option<crate::ImageResource>,
    pub(crate) key: Key,
    pub(crate) kind: WidgetKind,
    pub(crate) style: Style,
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

impl<M> Node<M> {
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
            WidgetKind::Obj
            | WidgetKind::Row
            | WidgetKind::Column
            | WidgetKind::Grid { .. }
            | WidgetKind::Label { .. } => Style::neutral(),
            #[cfg(feature = "widget-span")]
            WidgetKind::Spans { .. } => Style::neutral(),
            #[cfg(feature = "widget-arclabel")]
            WidgetKind::ArcLabel { .. } => Style::neutral(),
            _ => Style::default(),
        };
        Self {
            #[cfg(any(feature = "diagnostics", feature = "strict-properties"))]
            invalid_property: None,
            static_text: None,
            font_resource: None,
            #[cfg(feature = "widget-image")]
            image_resource: None,
            key: Key::Auto,
            kind,
            style,
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
}
