//! Constructors for controls views.

use crate::node::{Node, RollerMode, TextOverflow, WidgetKind};
use crate::View;
#[cfg(feature = "widget-ime-pinyin")]
use crate::{Font, Size, Style};
use alloc::string::String;

/// A text label that wraps long text.
pub fn label<M>(text: impl Into<String>) -> View<M> {
    Node::new(WidgetKind::Label {
        text: text.into(),
        text_overflow: TextOverflow::Wrap,
    })
}

/// A native button with a text label.
pub fn button<M>(text: impl Into<String>) -> View<M> {
    Node::new(WidgetKind::Button { text: text.into() })
}

/// Borrow fixed label text without allocating a Rust String for each view.
/// LVGL still copies changed text into its native label allocation.
pub fn label_static<M>(text: &'static str) -> View<M> {
    let mut node = label(String::new());
    node.static_text = Some(text);
    node
}

/// Borrow fixed button text while preserving the unified View type.
pub fn button_static<M>(text: &'static str) -> View<M> {
    let mut node = button(String::new());
    node.static_text = Some(text);
    node
}

/// A controlled slider with a value in `min..=max`.
#[cfg(feature = "widget-slider")]
pub fn slider<M>(min: i32, max: i32, value: i32) -> View<M> {
    Node::new(WidgetKind::Slider { min, max, value })
}

/// A progress bar with a value in `min..=max`.
#[cfg(feature = "widget-bar")]
pub fn progress_bar<M>(min: i32, max: i32, value: i32) -> View<M> {
    Node::new(WidgetKind::ProgressBar { min, max, value })
}

/// A switch with a controlled checked state.
#[cfg(feature = "widget-switch")]
pub fn switch<M>(checked: bool) -> View<M> {
    Node::new(WidgetKind::Switch { checked })
}

/// A checkbox with a text label and controlled checked state.
#[cfg(feature = "widget-checkbox")]
pub fn checkbox<M>(text: impl Into<String>, checked: bool) -> View<M> {
    Node::new(WidgetKind::Checkbox {
        text: text.into(),
        checked,
    })
}

/// A one-line textarea by default. Use [`View::multiline`] to allow line
/// breaks and [`View::placeholder`]/[`View::password`] for form fields.
#[cfg(feature = "widget-textarea")]
pub fn textarea<M>(text: impl Into<String>) -> View<M> {
    Node::new(WidgetKind::Textarea {
        text: text.into(),
        placeholder: String::new(),
        one_line: true,
        password: false,
    })
}

/// A dropdown with a controlled selected index.
#[cfg(feature = "widget-dropdown")]
pub fn dropdown<M>(options: impl IntoIterator<Item = impl Into<String>>, selected: u32) -> View<M> {
    Node::new(WidgetKind::Dropdown {
        options: options.into_iter().map(Into::into).collect(),
        selected,
    })
}

/// An arc with a controlled value in `min..=max`.
#[cfg(feature = "widget-arc")]
pub fn arc<M>(min: i32, max: i32, value: i32) -> View<M> {
    Node::new(WidgetKind::Arc { min, max, value })
}

/// A keyboard. `target` is matched against a textarea's
/// [`View::keyboard_id`].
#[cfg(feature = "widget-keyboard")]
pub fn keyboard<M>(target: u64) -> View<M> {
    Node::new(WidgetKind::Keyboard { target })
}

/// A roller (scrolling option picker).
///
/// `selected` is controlled; `on_value_changed` receives the selected
/// index. Defaults to three visible rows and [`RollerMode::Normal`].
#[cfg(feature = "widget-roller")]
pub fn roller<M>(options: impl IntoIterator<Item = impl Into<String>>, selected: u32) -> View<M> {
    Node::new(WidgetKind::Roller {
        options: options.into_iter().map(Into::into).collect(),
        selected,
        visible_rows: 3,
        mode: RollerMode::Normal,
    })
}

/// A spinbox (numeric field) with a controlled `value` in `min..=max`.
///
/// Defaults to five digits, no decimal separator and a step of one.
/// `on_value_changed` receives the integer value.
#[cfg(feature = "widget-spinbox")]
pub fn spinbox<M>(min: i32, max: i32, value: i32) -> View<M> {
    Node::new(WidgetKind::Spinbox {
        min,
        max,
        value,
        digit_count: 5,
        decimal_pos: 0,
        step: 1,
    })
}

/// A button matrix, one inner iterator per row.
///
/// `on_value_changed` receives the index of the pressed button, counting
/// row by row, or `-1`. Button texts must be non-empty. The mounted tree
/// owns the map LVGL reads from.
#[cfg(feature = "widget-buttonmatrix")]
pub fn button_matrix<M, R, T>(rows: impl IntoIterator<Item = R>) -> View<M>
where
    R: IntoIterator<Item = T>,
    T: Into<String>,
{
    Node::new(WidgetKind::ButtonMatrix {
        rows: rows
            .into_iter()
            .map(|row| row.into_iter().map(Into::into).collect())
            .collect(),
    })
}

/// A 26-key Pinyin IME with LVGL's bundled dictionary, bound to a textarea's
/// keyboard_id. Its candidate font must contain the requested CJK glyphs.
/// LVGL shares candidate text buffers: use one active Pinyin keyboard at a
/// time. The container includes its keyboard and candidate strip.
#[cfg(feature = "widget-ime-pinyin")]
pub fn pinyin_keyboard<M>(target: u64, font: Font) -> View<M> {
    Node::new(WidgetKind::PinyinKeyboard { target, font })
        .style(
            Style::plain()
                .width(Size::Percent(100))
                .height(Size::Px(200)),
        )
        .scrollable(false)
}
