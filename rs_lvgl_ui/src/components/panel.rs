//! A titled container with header actions, the replacement for LVGL's
//! deprecated `lv_win`.
//!
//! ```ignore
//! Panel::new("Logs")
//!     .action(panel::icon_button(theme, symbol::REFRESH, LogPanelAction::Reload))
//!     .child(crate::label(log))
//!     .view(theme)
//! ```

use crate::View;
use alloc::string::String;
use alloc::vec::Vec;

use crate::node::{FlexAlign, ItemAlign, Key, Size, State, TextOverflow, WidgetKind};
use crate::theme::{Style, Theme};

/// A compact accent button showing one glyph, for header actions.
pub fn icon_button<M: Clone + 'static>(theme: &Theme, icon: &'static str, action: M) -> View<M> {
    crate::button(icon)
        .background(theme.surface)
        .text_color(theme.text)
        .padding(6)
        .radius(6)
        .shadow_width(0)
        .state_style(State::PRESSED, crate::style().background(theme.accent))
        .on_click(action)
}

/// A column with a header row (title and actions) and a scrollable content
/// area that fills the remaining height.
///
/// The panel fills its parent's width. Give it a height (for example
/// `Size::Percent(100)` or `grow`) through [`Panel::style`] to make the
/// content area scroll instead of growing.
pub struct Panel<M> {
    title: String,
    actions: Vec<View<M>>,
    children: Vec<View<M>>,
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
    pub fn header_action(mut self, action: impl Into<View<M>>) -> Self {
        self.actions.push(action.into());
        self
    }

    /// Append header widgets in iterator order.
    pub fn header_actions(mut self, actions: impl IntoIterator<Item = View<M>>) -> Self {
        self.actions.extend(actions);
        self
    }

    /// Compatibility spelling for [`Panel::header_action`].
    pub fn action(self, action: impl Into<View<M>>) -> Self {
        self.header_action(action)
    }

    /// Replace the body with one content view. Later child/children calls append.
    pub fn content(mut self, content: impl Into<View<M>>) -> Self {
        self.children.clear();
        self.children.push(content.into());
        self
    }

    pub fn child(mut self, child: impl Into<View<M>>) -> Self {
        self.children.push(child.into());
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = View<M>>) -> Self {
        self.children.extend(children);
        self
    }

    /// Style merged over the panel's surface style.
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    pub fn view(self, theme: &Theme) -> View<M> {
        let base = Style::surface(theme)
            .padding(0)
            .gap(0)
            .width(Size::Percent(100));
        let base = match self.style {
            Some(style) => base.merged(style),
            None => base,
        };
        let header = crate::row([View::new(WidgetKind::Label {
            text: self.title,
            text_overflow: TextOverflow::Ellipsis,
        })
        .key(Key::Named("title"))
        .font(theme.title_font)
        .text_color(theme.text)
        .grow(1)])
        .key(Key::Named("header"))
        .width(Size::Percent(100))
        .padding(8)
        .padding_left(12)
        .gap(6)
        .justify_content(FlexAlign::Start)
        .align_items(ItemAlign::Center)
        .align_content(FlexAlign::Center)
        .background(theme.background)
        .background_opacity(90)
        .scrollable(false)
        .children(self.actions);
        let content = crate::column(self.children)
            .key(Key::Named("content"))
            .font(theme.body_font)
            .text_color(theme.text)
            .width(Size::Percent(100))
            .padding(12)
            .gap(8)
            .grow(1);
        crate::column([header, content]).style(base)
    }
}
