//! Stack navigation between pages, the replacement for LVGL's deprecated
//! `lv_menu`.
//!
//! The page stack is application state. The application stores a
//! [`NavState`], forwards [`NavMsg`] to [`NavState::update`] and renders the
//! current page itself:
//!
//! ```ignore
//! enum Page { Root, Wifi }
//! enum Msg { Nav(NavMsg<Page>), /* ... */ }
//!
//! fn view(&self, theme: &Theme) -> Node<Msg> {
//!     let page = match self.nav.current() {
//!         Page::Root => list::list(theme, [
//!             nav::link(theme, "Wi-Fi", Msg::Nav(NavMsg::Push(Page::Wifi))),
//!         ]),
//!         Page::Wifi => wifi_page(theme),
//!     };
//!     nav::view(&self.nav, theme, title(self.nav.current()), page, Msg::Nav(NavMsg::Back))
//! }
//! ```

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use super::list::ListItem;
use super::panel::icon_button;
use crate::node::{FlexAlign, Key, LabelLongMode, Node, Size, WidgetKind};
use crate::symbol;
use crate::theme::{Style, Theme};

/// Navigation messages for a [`NavState`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavMsg<P> {
    /// Open a page above the current one.
    Push(P),
    /// Return to the previous page. Ignored on the root page.
    Back,
    /// Return to the root page.
    Root,
    /// Replace the current page without growing the stack.
    Replace(P),
}

/// A non-empty stack of pages; the first entry is the root page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavState<P> {
    stack: Vec<P>,
}

impl<P: Clone> NavState<P> {
    pub fn new(root: P) -> Self {
        Self { stack: vec![root] }
    }

    pub fn current(&self) -> &P {
        self.stack
            .last()
            .expect("the stack always holds the root page")
    }

    /// Pages from the root to the current page.
    pub fn pages(&self) -> &[P] {
        &self.stack
    }

    /// Number of pages above the root.
    pub fn depth(&self) -> usize {
        self.stack.len() - 1
    }

    pub fn can_go_back(&self) -> bool {
        self.stack.len() > 1
    }

    pub fn push(&mut self, page: P) {
        self.stack.push(page);
    }

    /// Pop the current page. Returns `false` on the root page.
    pub fn back(&mut self) -> bool {
        if self.can_go_back() {
            self.stack.pop();
            true
        } else {
            false
        }
    }

    pub fn update(&mut self, msg: NavMsg<P>) {
        match msg {
            NavMsg::Push(page) => self.push(page),
            NavMsg::Back => {
                self.back();
            }
            NavMsg::Root => self.stack.truncate(1),
            NavMsg::Replace(page) => {
                *self.stack.last_mut().expect("non-empty stack") = page;
            }
        }
    }
}

/// A list row that navigates: text with a chevron.
pub fn link<M: Clone + 'static>(theme: &Theme, text: impl Into<String>, msg: M) -> Node<M> {
    ListItem::new(text).chevron(true).on_click(msg).view(theme)
}

/// A header with a back button (hidden on the root page) and `title`, above
/// `page`, filling the parent.
///
/// The page is keyed by stack depth, so navigating creates the new page's
/// widgets (and scroll position) fresh instead of morphing the previous page.
pub fn view<M: Clone + 'static, P: Clone>(
    state: &NavState<P>,
    theme: &Theme,
    title: impl Into<String>,
    page: Node<M>,
    back: M,
) -> Node<M> {
    let mut header = Node::row()
        .key(Key::Named("nav-header"))
        .style(
            Style::default()
                .width(Size::Percent(100))
                .pad(8)
                .gap(8)
                .flex_align(FlexAlign::Start, FlexAlign::Center),
        )
        .scrollable(false);
    if state.can_go_back() {
        header = header.child(icon_button(theme, symbol::LEFT, back).key(Key::Named("back")));
    }
    header = header.child(
        Node::new(WidgetKind::Label {
            text: title.into(),
            long_mode: LabelLongMode::Dots,
        })
        .key(Key::Named("title"))
        .style(Style::title(theme).flex_grow(1)),
    );
    let body = Node::column()
        .key(Key::Id(state.depth() as u64))
        .style(
            Style::default()
                .width(Size::Percent(100))
                .pad(8)
                .gap(8)
                .flex_grow(1),
        )
        .child(page);
    Node::column()
        .style(
            Style::screen(theme)
                .fg(theme.text)
                .font(theme.body_font)
                .gap(0),
        )
        .child(header)
        .child(body)
}
