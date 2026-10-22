//! Scrollable lists of rows, the replacement for LVGL's deprecated `lv_list`.
//!
//! ```ignore
//! list::list(theme, [
//!     list::section(theme, "Network"),
//!     ListItem::new("Wi-Fi")
//!         .icon(symbol::WIFI)
//!         .detail("Home")
//!         .chevron(true)
//!         .on_click(Msg::OpenWifi)
//!         .view(theme)
//!         .key(Key::Named("wifi")),
//! ])
//! ```

use alloc::boxed::Box;
use alloc::string::String;

use crate::node::{FlexAlign, Key, LabelLongMode, Node, Size, State, TextAlign, WidgetKind};
use crate::symbol;
use crate::theme::{Style, Theme};

/// A scrollable column of rows on a surface.
pub fn list<M>(theme: &Theme, items: impl IntoIterator<Item = Node<M>>) -> Node<M> {
    Node::column()
        .style(
            Style::surface(theme)
                .pad(4)
                .gap(2)
                .width(Size::Percent(100)),
        )
        .children(items)
}

/// A muted heading between groups of rows.
pub fn section<M>(theme: &Theme, title: impl Into<String>) -> Node<M> {
    Node::label(title).style(Style::muted(theme).pad_left(8).pad_top(8).pad_bottom(2))
}

/// One list row: optional icon, text, optional trailing content and an
/// optional chevron.
///
/// A row with [`ListItem::on_click`] highlights while pressed. A
/// [`ListItem::selected`] row is drawn with the accent color through
/// `State::CHECKED`, so selection is application state like any other
/// controlled value.
pub struct ListItem<M> {
    text: String,
    icon: Option<&'static str>,
    trailing: Option<Box<Node<M>>>,
    chevron: bool,
    selected: Option<bool>,
    click: Option<M>,
    style: Option<Style>,
}

impl<M: Clone + 'static> ListItem<M> {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            icon: None,
            trailing: None,
            chevron: false,
            selected: None,
            click: None,
            style: None,
        }
    }

    /// Leading glyph, usually a [`crate::symbol`] constant.
    pub fn icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Muted trailing text such as a current value.
    pub fn detail(self, detail: impl Into<String>) -> Self {
        self.trailing(Node::label(detail).key(Key::Named("detail")))
    }

    /// Trailing node such as a switch. It replaces [`ListItem::detail`].
    ///
    /// The node keeps its own handlers; a click on an interactive trailing
    /// widget does not also click the row.
    pub fn trailing(mut self, node: Node<M>) -> Self {
        self.trailing = Some(Box::new(node));
        self
    }

    /// Show a right chevron, conventionally for rows that navigate.
    pub fn chevron(mut self, chevron: bool) -> Self {
        self.chevron = chevron;
        self
    }

    /// Draw the row as selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    pub fn on_click(mut self, msg: M) -> Self {
        self.click = Some(msg);
        self
    }

    /// Style merged over the row's default style.
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    pub fn view(self, theme: &Theme) -> Node<M> {
        let base = Style::body(theme)
            .width(Size::Percent(100))
            .pad(10)
            .gap(10)
            .radius(6)
            .flex_align(FlexAlign::Start, FlexAlign::Center);
        let base = match self.style {
            Some(style) => base.merged(style),
            None => base,
        };
        let mut row = Node::row().style(base).scrollable(false).state_style(
            State::CHECKED,
            Style::default().bg(theme.accent).fg(theme.background),
        );
        if let Some(icon) = self.icon {
            // A fixed slot keeps the text aligned across glyphs of different
            // widths.
            row = row.child(
                Node::label(icon).key(Key::Named("icon")).style(
                    Style::default()
                        .width(Size::Px(20))
                        .text_align(TextAlign::Center),
                ),
            );
        }
        row = row.child(
            Node::new(WidgetKind::Label {
                text: self.text,
                long_mode: LabelLongMode::Dots,
            })
            .key(Key::Named("text"))
            .style(Style::default().flex_grow(1)),
        );
        if let Some(trailing) = self.trailing {
            let mut trailing = *trailing;
            // The trailing node's own style wins over the muted default.
            trailing.style = Style::muted(theme).merged(trailing.style);
            row = row.child(trailing);
        }
        if self.chevron {
            row = row.child(
                Node::label(symbol::RIGHT)
                    .key(Key::Named("chevron"))
                    .style(Style::muted(theme)),
            );
        }
        match self.click {
            Some(msg) => {
                row = row.on_click(msg).state_style(
                    State::PRESSED,
                    Style::default().bg(theme.muted).bg_opacity(80),
                );
            }
            None => row = row.clickable(false),
        }
        if let Some(selected) = self.selected {
            row = row.checked(selected);
        }
        row
    }
}
