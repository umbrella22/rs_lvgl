//! Composite components built from [`View`] trees.
//!
//! Components follow the same Elm-style split as applications:
//!
//! - Stateless components ([`list`], [`panel`], [`dialog`]) are builders
//!   whose `view` returns a `View<M>` that emits the application's messages.
//! - Stateful components ([`nav::NavState`], [`toast::Toasts`]) keep their
//!   state in a plain struct owned by the application and change it in
//!   `update`; their `view` functions only read it.
//!
//! Components never hold native handles. They are ordinary subtrees, so keys,
//! controlled values, styles and [`View::map`] behave as in hand-written
//! views. Overlay components ([`dialog`], [`toast`]) are meant for
//! [`crate::App::overlay`]; combine several with [`layers`].

use crate::node::Size;
use crate::theme::Style;
use crate::View;

pub mod dialog;
pub mod file_explorer;
pub mod list;
pub mod nav;
pub mod panel;
pub mod toast;

/// A full-size, transparent, non-clickable container for overlay layers.
///
/// Children are placed with [`Style::align`] and [`Style::offset`]; later
/// children draw above earlier ones. Input passes through the container to
/// the view, except where a child is hit.
pub fn layers<M>(children: impl IntoIterator<Item = View<M>>) -> View<M> {
    crate::obj()
        .style(full_size())
        .clickable(false)
        .scrollable(false)
        .children(children)
}

pub(crate) fn full_size() -> Style {
    Style::neutral()
        .width(Size::Percent(100))
        .height(Size::Percent(100))
}
