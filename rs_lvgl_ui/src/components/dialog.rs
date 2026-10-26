//! Modal dialogs for [`crate::App::overlay`].
//!
//! A dialog is shown while the application state says so; closing it is a
//! message like any other:
//!
//! ```ignore
//! fn overlay(&self, theme: &Theme) -> Option<View<Msg>> {
//!     self.confirm.then(|| {
//!         Dialog::new("Delete file?")
//!             .text("This cannot be undone.")
//!             .button("Cancel", Msg::CloseDialog)
//!             .primary("Delete", Msg::Delete)
//!             .on_dismiss(Msg::CloseDialog)
//!             .view(theme)
//!     })
//! }
//! ```

use crate::View;
use alloc::string::String;
use alloc::vec::Vec;

use super::full_size;
use crate::node::{Align, Color, FlexAlign, Key, Size, State};
use crate::theme::{Style, Theme};

const FOCUS_WIDTH: i16 = 2;
const FOCUS_PAD: i16 = 3;
// Keep the complete outer focus ring inside the footer's clipping area.
const FOCUS_SPACE: i16 = FOCUS_WIDTH + FOCUS_PAD + 1;

struct Button<M> {
    text: String,
    msg: M,
    primary: bool,
}

/// A centered card over a dimming backdrop.
///
/// The backdrop covers the whole display and absorbs input, so the view
/// beneath cannot be used while the dialog is shown. Clicking the backdrop
/// sends [`Dialog::on_dismiss`], if set.
pub struct Dialog<M> {
    title: String,
    body: Vec<View<M>>,
    buttons: Vec<Button<M>>,
    dismiss: Option<M>,
    width: Size,
}

impl<M: Clone + 'static> Dialog<M> {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: Vec::new(),
            buttons: Vec::new(),
            dismiss: None,
            width: Size::Percent(80),
        }
    }

    /// Add a paragraph of body text.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.body
            .push(crate::label(text).style(Style::neutral().width(Size::Percent(100))));
        self
    }

    /// Add arbitrary body content, such as a textarea or a list.
    pub fn child(mut self, child: View<M>) -> Self {
        self.body.push(child);
        self
    }

    /// Add a footer button. Buttons appear right-aligned in insertion order.
    pub fn button(mut self, text: impl Into<String>, msg: M) -> Self {
        self.buttons.push(Button {
            text: text.into(),
            msg,
            primary: false,
        });
        self
    }

    /// Add an accent footer button for the main action.
    pub fn primary(mut self, text: impl Into<String>, msg: M) -> Self {
        self.buttons.push(Button {
            text: text.into(),
            msg,
            primary: true,
        });
        self
    }

    /// Message sent when the backdrop is clicked.
    pub fn on_dismiss(mut self, msg: M) -> Self {
        self.dismiss = Some(msg);
        self
    }

    /// Card width; defaults to 80% of the display, capped at 360 px.
    pub fn width(mut self, width: Size) -> Self {
        self.width = width;
        self
    }

    pub fn view(self, theme: &Theme) -> View<M> {
        let mut card = crate::column([crate::label(self.title)
            .key(Key::Named("title"))
            .style(Style::title(theme).width(Size::Percent(100)))])
        .key(Key::Named("card"))
        .style(
            Style::surface(theme)
                .font(theme.body_font)
                .width(self.width)
                .max_width(Size::Px(360))
                .max_height(Size::Percent(90))
                .pad(16)
                .gap(12)
                .align(Align::Center),
        )
        .children(self.body);
        if !self.buttons.is_empty() {
            let focus = Style::default()
                .outline_color(theme.accent)
                .outline_width(FOCUS_WIDTH)
                .outline_opacity(255)
                .outline_pad(FOCUS_PAD);
            let buttons = self.buttons.into_iter().enumerate().map(|(index, button)| {
                let style = if button.primary {
                    Style::accent(theme)
                } else {
                    Style::default()
                        .bg(theme.background)
                        .fg(theme.text)
                        .pad(8)
                        .radius(8)
                        .shadow_width(0)
                };
                crate::button(button.text)
                    .key(Key::Id(index as u64))
                    .style(style.outline_width(0))
                    .state_style(State::FOCUSED, focus)
                    .state_style(State::FOCUS_KEY, focus)
                    .state_style(State::PRESSED, Style::default().opacity(200))
                    .on_click(button.msg)
            });
            card = card.child(
                crate::row(buttons)
                    .key(Key::Named("buttons"))
                    .style(
                        Style::default()
                            .width(Size::Percent(100))
                            .pad(FOCUS_SPACE)
                            .gap(FOCUS_SPACE * 2)
                            .flex_align(FlexAlign::End, FlexAlign::Center),
                    )
                    .scrollable(false),
            );
        }
        let mut backdrop = crate::obj()
            .key(Key::Named("dialog"))
            .style(full_size().bg(Color::rgb(0, 0, 0)).bg_opacity(128))
            .scrollable(false)
            .child(card);
        #[cfg(feature = "input-keypad")]
        {
            backdrop = backdrop.modal(true);
        }
        if let Some(msg) = self.dismiss {
            backdrop = backdrop.on_click(msg);
        }
        backdrop
    }
}
