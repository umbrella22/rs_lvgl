//! Scrollable lists of rows, the replacement for LVGL's deprecated `lv_list`.
//!
//! ```ignore
//! list::list(theme, [
//!     list::section(theme, "Network"),
//!     ListItem::new("Wi-Fi")
//!         .icon(symbol::WIFI)
//!         .detail("Home")
//!         .chevron(true)
//!         .on_click(SettingsAction::OpenWifi)
//!         .view(theme)
//!         .key(Key::Named("wifi")),
//! ])
//! ```

use crate::View;
use alloc::boxed::Box;
use alloc::string::String;

use crate::node::{FlexAlign, Key, LabelLongMode, Size, State, TextAlign, WidgetKind};
use crate::symbol;
use crate::theme::{Style, Theme};

/// A scrollable column of rows on a surface.
pub fn list<M>(theme: &Theme, items: impl IntoIterator<Item = View<M>>) -> View<M> {
    crate::column(items)
        .style(Style::surface(theme))
        .padding(4)
        .gap(2)
        .width(Size::Percent(100))
}

/// A muted heading between groups of rows.
pub fn section<M>(theme: &Theme, title: impl Into<String>) -> View<M> {
    crate::label(title)
        .font(theme.body_font)
        .text_color(theme.muted)
        .padding_left(8)
        .padding_top(8)
        .padding_bottom(2)
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
    leading: Option<Box<View<M>>>,
    content: Option<Box<View<M>>>,
    trailing: Option<Box<View<M>>>,
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
            leading: None,
            content: None,
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
        self.leading = None;
        self
    }

    /// Leading content, replacing the icon. The slot retains its handlers.
    pub fn leading(mut self, node: impl Into<View<M>>) -> Self {
        self.leading = Some(Box::new(node.into()));
        self.icon = None;
        self
    }

    /// Main content, replacing the default text label. Its explicit styles
    /// override the default flex growth. An unkeyed root gets a stable slot key.
    pub fn content(mut self, node: impl Into<View<M>>) -> Self {
        self.content = Some(Box::new(node.into()));
        self
    }

    /// Muted trailing text such as a current value.
    pub fn detail(self, detail: impl Into<String>) -> Self {
        self.trailing(crate::label(detail).key(Key::Named("detail")))
    }

    /// Trailing node such as a switch. It replaces [`ListItem::detail`].
    ///
    /// The node keeps its own handlers; a click on an interactive trailing
    /// widget does not also click the row.
    pub fn trailing(mut self, node: impl Into<View<M>>) -> Self {
        self.trailing = Some(Box::new(node.into()));
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

    pub fn on_click(mut self, action: M) -> Self {
        self.click = Some(action);
        self
    }

    /// Style merged over the row's default style.
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    pub fn view(self, theme: &Theme) -> View<M> {
        let base = Style::body(theme)
            .width(Size::Percent(100))
            .padding(10)
            .gap(10)
            .radius(6)
            .flex_align(FlexAlign::Start, FlexAlign::Center);
        let base = match self.style {
            Some(style) => base.merged(style),
            None => base,
        };
        let mut row = crate::row([]).style(base).scrollable(false).state_style(
            State::CHECKED,
            crate::style()
                .background(theme.accent)
                .text_color(theme.background),
        );
        if let Some(icon) = self.icon {
            // A fixed slot keeps the text aligned across glyphs of different
            // widths.
            row = row.child(
                crate::label(icon)
                    .key(Key::Named("icon"))
                    .width(Size::Px(20))
                    .text_align(TextAlign::Center),
            );
        }
        if let Some(leading) = self.leading {
            row = row.child(super::slot(*leading, "leading", crate::style()));
        }
        let content = self.content.map(|node| *node).unwrap_or_else(|| {
            View::new(WidgetKind::Label {
                text: self.text,
                long_mode: LabelLongMode::Dots,
            })
        });
        row = row.child(super::slot(content, "text", crate::style().grow(1)));
        if let Some(trailing) = self.trailing {
            // The trailing node's own style wins over the muted default.
            row = row.child(super::slot(*trailing, "trailing", Style::muted(theme)));
        }
        if self.chevron {
            row = row.child(
                crate::label(symbol::RIGHT)
                    .key(Key::Named("chevron"))
                    .font(theme.body_font)
                    .text_color(theme.muted),
            );
        }
        match self.click {
            Some(msg) => {
                row = row.on_click(msg).state_style(
                    State::PRESSED,
                    crate::style()
                        .background(theme.muted)
                        .background_opacity(80),
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
