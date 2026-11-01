//! Constructors for layout views.

use crate::node::{GridTrack, Node, Tile, WidgetKind};
use crate::View;
use alloc::string::String;

/// An unstyled container for absolute positioning.
pub fn obj<M>() -> View<M> {
    Node::new(WidgetKind::Obj)
}

/// A horizontal flex container with the supplied children.
///
/// Arrays and iterators preserve their iteration order. See [`column()`] for
/// composition with named child views and ordinary helper functions.
pub fn row<M>(children: impl IntoIterator<Item = View<M>>) -> View<M> {
    Node::new(WidgetKind::Row).children(children)
}

/// A vertical flex container with the supplied children.
///
/// Children in the normal flex flow appear from top to bottom in iteration
/// order. Their sizes, spacing and alignment determine the pixel positions.
/// Extracting a child into a variable or function does not change its position;
/// the order passed to `column` determines placement.
///
/// ```
/// use rs_lvgl_ui::prelude::*;
///
/// fn page_title<M>(text: &str, theme: &Theme) -> View<M> {
///     label(text).style(Style::title(theme))
/// }
///
/// fn settings_view(theme: &Theme) -> View<()> {
///     let content = label("Notification preferences");
///     let title = page_title("Settings", theme);
///     column([title, content]).style(Style::default().gap(12))
/// }
/// # let _ = settings_view(&Theme::light());
/// ```
///
/// A helper returns a view description; it does not create independent state or
/// a render cache. Child order is separate from identity: use stable
/// [`Key`](crate::Key) values for children that can be inserted, removed or
/// reordered. The default `Key::Auto` matches by position among `Auto` siblings.
pub fn column<M>(children: impl IntoIterator<Item = View<M>>) -> View<M> {
    Node::new(WidgetKind::Column).children(children)
}

/// A grid with column tracks, row tracks and child views.
pub fn grid<M>(
    columns: impl IntoIterator<Item = GridTrack>,
    rows: impl IntoIterator<Item = GridTrack>,
    children: impl IntoIterator<Item = View<M>>,
) -> View<M> {
    Node::new(WidgetKind::Grid {
        columns: columns.into_iter().collect(),
        rows: rows.into_iter().collect(),
    })
    .children(children)
}

/// A native tabview. Add exactly one child per tab; each child becomes the
/// root of that tab's page.
#[cfg(feature = "widget-tabview")]
pub fn tabview<M>(tabs: impl IntoIterator<Item = impl Into<String>>, active: u32) -> View<M> {
    Node::new(WidgetKind::Tabview {
        tabs: tabs.into_iter().map(Into::into).collect(),
        active,
    })
}

/// A tileview. Add exactly one child per tile; each child becomes the
/// root of that tile. `active` indexes `tiles` and is controlled.
#[cfg(feature = "widget-tileview")]
pub fn tileview<M>(tiles: impl IntoIterator<Item = Tile>, active: u32) -> View<M> {
    Node::new(WidgetKind::Tileview {
        tiles: tiles.into_iter().collect(),
        active,
    })
}
