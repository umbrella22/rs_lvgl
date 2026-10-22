//! A titled container with header actions, the replacement for LVGL's
//! deprecated `lv_win`.
//!
//! ```ignore
//! Panel::new("Logs")
//!     .action(panel::icon_button(theme, symbol::REFRESH, Msg::Reload))
//!     .child(Node::label(log))
//!     .view(theme)
//! ```

use alloc::string::String;
use alloc::vec::Vec;

use crate::node::{FlexAlign, Key, LabelLongMode, Node, Size, State, WidgetKind};
use crate::theme::{Style, Theme};

/// A compact accent button showing one glyph, for header actions.
pub fn icon_button<M: Clone + 'static>(theme: &Theme, icon: &'static str, msg: M) -> Node<M> {
    Node::button(icon)
        .style(
            Style::default()
                .bg(theme.surface)
                .fg(theme.text)
                .pad(6)
                .radius(6)
                .shadow_width(0),
        )
        .state_style(State::PRESSED, Style::default().bg(theme.accent))
        .on_click(msg)
}

/// A column with a header row (title and actions) and a scrollable content
/// area that fills the remaining height.
///
/// The panel fills its parent's width. Give it a height (for example
/// `Size::Percent(100)` or `flex_grow`) through [`Panel::style`] to make the
/// content area scroll instead of growing.
pub struct Panel<M> {
    title: String,
    actions: Vec<Node<M>>,
    children: Vec<Node<M>>,
    style: Option<Style>,
}

impl<M: Clone + 'static> Panel<M> {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            actions: Vec::new(),
            children: Vec::new(),
            style: None,
        }
    }

    /// Add a header action, typically an [`icon_button`]. Actions appear
    /// after the title in insertion order.
    pub fn action(mut self, action: Node<M>) -> Self {
        self.actions.push(action);
        self
    }

    pub fn child(mut self, child: Node<M>) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = Node<M>>) -> Self {
        self.children.extend(children);
        self
    }

    /// Style merged over the panel's surface style.
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    pub fn view(self, theme: &Theme) -> Node<M> {
        let base = Style::surface(theme)
            .pad(0)
            .gap(0)
            .width(Size::Percent(100));
        let base = match self.style {
            Some(style) => base.merged(style),
            None => base,
        };
        let header = Node::row()
            .key(Key::Named("header"))
            .style(
                Style::default()
                    .width(Size::Percent(100))
                    .pad(8)
                    .pad_left(12)
                    .gap(6)
                    .flex_align(FlexAlign::Start, FlexAlign::Center)
                    .bg(theme.background)
                    .bg_opacity(90),
            )
            .scrollable(false)
            .child(
                Node::new(WidgetKind::Label {
                    text: self.title,
                    long_mode: LabelLongMode::Dots,
                })
                .key(Key::Named("title"))
                .style(Style::title(theme).flex_grow(1)),
            )
            .children(self.actions);
        let content = Node::column()
            .key(Key::Named("content"))
            .style(
                Style::body(theme)
                    .width(Size::Percent(100))
                    .pad(12)
                    .gap(8)
                    .flex_grow(1),
            )
            .children(self.children);
        Node::column().style(base).child(header).child(content)
    }
}
