//! Style presets operations.

use super::{Style, Theme};
use crate::node::Size;

impl Style {
    /// Title text using the theme's title font and text color.
    pub fn title(theme: &Theme) -> Self {
        Self::default()
            .text_color(theme.text)
            .font(theme.title_font)
    }

    /// Body text using the theme's body font and text color.
    pub fn body(theme: &Theme) -> Self {
        Self::default().text_color(theme.text).font(theme.body_font)
    }

    /// Muted body text.
    pub fn muted(theme: &Theme) -> Self {
        Self::default()
            .text_color(theme.muted)
            .font(theme.body_font)
    }

    /// A raised surface with padding and rounded corners.
    pub fn surface(theme: &Theme) -> Self {
        Self::default()
            .background(theme.surface)
            .text_color(theme.text)
            .padding(12)
            .radius(8)
    }

    /// A flat accent-filled surface with no inherited native-theme shadow.
    /// Set `shadow_width` afterwards to opt into an explicit shadow.
    pub fn accent(theme: &Theme) -> Self {
        Self::default()
            .background(theme.accent)
            .text_color(theme.background)
            .padding(8)
            .radius(8)
            .shadow_width(0)
    }

    /// Full-screen background.
    pub fn screen(theme: &Theme) -> Self {
        Self::default()
            .background(theme.background)
            .width(Size::Percent(100))
            .height(Size::Percent(100))
            .padding(0)
    }
}
