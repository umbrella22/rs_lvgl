//! Transient notifications for [`crate::App::overlay`].
//!
//! [`Toasts`] is application state. Toasts expire through
//! [`Toasts::advance`], which the application calls with the elapsed time,
//! typically from a tick message the platform posts every frame:
//!
//! ```ignore
//! Msg::Tick(ms) => { self.toasts.advance(ms); }
//! Msg::Saved => { self.toasts.push("Saved", 2000); }
//! Msg::DismissToast(id) => self.toasts.dismiss(id),
//!
//! fn overlay(&self, theme: &Theme) -> Option<View<Msg>> {
//!     (!self.toasts.is_empty()).then(|| toast::view(&self.toasts, theme, Msg::DismissToast))
//! }
//! ```

use crate::View;
use alloc::string::String;
use alloc::vec::Vec;

use crate::node::{Align, Color, FlexAlign, Key, Size, TextAlign};
use crate::theme::{Style, Theme};

/// Visual kind of a toast.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToastKind {
    #[default]
    Info,
    Success,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Toast {
    id: u64,
    text: String,
    kind: ToastKind,
    remaining_ms: u32,
}

/// Queue of visible toasts, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Toasts {
    items: Vec<Toast>,
    next_id: u64,
    max_visible: Option<usize>,
}

impl Toasts {
    pub fn new() -> Self {
        Self::default()
    }

    /// Keep at most `max` toasts; pushing beyond it drops the oldest.
    pub fn with_max_visible(max: usize) -> Self {
        Self {
            max_visible: Some(max.max(1)),
            ..Self::default()
        }
    }

    /// Show an info toast for `duration_ms`. Returns its id.
    pub fn push(&mut self, text: impl Into<String>, duration_ms: u32) -> u64 {
        self.push_kind(ToastKind::Info, text, duration_ms)
    }

    pub fn push_kind(&mut self, kind: ToastKind, text: impl Into<String>, duration_ms: u32) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(Toast {
            id,
            text: text.into(),
            kind,
            remaining_ms: duration_ms,
        });
        if let Some(max) = self.max_visible {
            if self.items.len() > max {
                let excess = self.items.len() - max;
                self.items.drain(..excess);
            }
        }
        id
    }

    pub fn dismiss(&mut self, id: u64) {
        self.items.retain(|toast| toast.id != id);
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Count down every toast by `elapsed_ms` and remove expired ones.
    /// Returns whether any toast was removed, that is, whether the view
    /// changed.
    pub fn advance(&mut self, elapsed_ms: u32) -> bool {
        let before = self.items.len();
        self.items.retain_mut(|toast| {
            toast.remaining_ms = toast.remaining_ms.saturating_sub(elapsed_ms);
            toast.remaining_ms > 0
        });
        self.items.len() != before
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Visible toast texts, oldest first.
    pub fn texts(&self) -> impl Iterator<Item = &str> {
        self.items.iter().map(|toast| toast.text.as_str())
    }
}

/// Toasts stacked and centered at the bottom of the display, newest at the
/// bottom.
///
/// Only the toasts take input; clicking one sends `dismiss(id)`. Each toast
/// is keyed by its id, so expiring one keeps the others' widgets.
pub fn view<M: Clone + 'static>(
    toasts: &Toasts,
    theme: &Theme,
    dismiss: impl Fn(u64) -> M,
) -> View<M> {
    crate::column(toasts.items.iter().map(|toast| {
        let (bg, fg) = match toast.kind {
            ToastKind::Info => (theme.surface, theme.text),
            ToastKind::Success => (theme.accent, theme.background),
            ToastKind::Error => (theme.danger, Color::rgb(0xff, 0xff, 0xff)),
        };
        crate::label(toast.text.clone())
            .key(Key::Id(toast.id))
            .style(
                Style::default()
                    .bg(bg)
                    .fg(fg)
                    .font(theme.body_font)
                    .pad(10)
                    .radius(8)
                    .text_align(TextAlign::Center),
            )
            .clickable(true)
            .on_click(dismiss(toast.id))
    }))
    .key(Key::Named("toasts"))
    .style(
        Style::neutral()
            .width(Size::Content)
            .height(Size::Content)
            .max_width(Size::Percent(90))
            .gap(6)
            .flex_align(FlexAlign::Start, FlexAlign::Center)
            .align(Align::BottomMid)
            .offset(0, -16),
    )
    .clickable(false)
    .scrollable(false)
}
