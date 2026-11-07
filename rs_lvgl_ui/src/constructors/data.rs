//! Constructors for data views.

use crate::node::{Color, Node, Span, WidgetKind};
use crate::View;
use alloc::{string::String, vec::Vec};

/// A scale with a range, total tick count and major tick interval.
#[cfg(feature = "widget-scale")]
pub fn scale<M>(min: i32, max: i32, total_ticks: u32, major_tick_every: u32) -> View<M> {
    Node::new(WidgetKind::Scale {
        min,
        max,
        total_ticks,
        major_tick_every,
    })
}

/// A spinner with LVGL's default animation (1000 ms, 200° arc).
#[cfg(feature = "widget-spinner")]
pub fn spinner<M>() -> View<M> {
    Node::new(WidgetKind::Spinner {
        duration_ms: 1000,
        arc_angle: 200,
    })
}

/// An LED indicator at full brightness.
#[cfg(feature = "widget-led")]
pub fn led<M>(color: Color) -> View<M> {
    Node::new(WidgetKind::Led {
        color,
        brightness: 255,
    })
}

/// A polyline. The mounted tree owns the point buffer LVGL reads from.
#[cfg(feature = "widget-line")]
pub fn line<M>(points: impl IntoIterator<Item = (i32, i32)>) -> View<M> {
    Node::new(WidgetKind::Line {
        points: points.into_iter().collect(),
    })
}

/// A text table, one inner iterator per row.
///
/// `on_value_changed` receives the selected cell; decode it with
/// [`table_cell`](crate::table_cell).
#[cfg(feature = "widget-table")]
pub fn table<M, R, T>(cells: impl IntoIterator<Item = R>) -> View<M>
where
    R: IntoIterator<Item = T>,
    T: Into<String>,
{
    Node::new(WidgetKind::Table {
        cells: cells
            .into_iter()
            .map(|row| row.into_iter().map(Into::into).collect())
            .collect(),
        column_widths: Vec::new(),
    })
}

/// An inline message box showing `text`. Children are placed below the
/// text. Add a title with [`View::message_box_title`] and footer buttons with
/// [`View::message_box_buttons`].
#[cfg(feature = "widget-msgbox")]
pub fn message_box<M>(text: impl Into<String>) -> View<M> {
    Node::new(WidgetKind::MessageBox {
        title: None,
        text: text.into(),
        buttons: Vec::new(),
    })
}

/// A calendar showing `month` (1-12) of `year`.
#[cfg(feature = "widget-calendar")]
pub fn calendar<M>(year: u16, month: u8) -> View<M> {
    Node::new(WidgetKind::Calendar {
        year,
        month,
        today: None,
        highlighted: Vec::new(),
    })
}

/// Rich text made of styled spans.
#[cfg(feature = "widget-span")]
pub fn spans<M>(spans: impl IntoIterator<Item = Span>) -> View<M> {
    Node::new(WidgetKind::Spans {
        spans: spans.into_iter().collect(),
    })
}

/// Text along an arc of `radius` pixels, starting at `start_angle` and
/// spanning `angle_size` degrees.
#[cfg(feature = "widget-arclabel")]
pub fn arc_label<M>(
    text: impl Into<String>,
    radius: u32,
    start_angle: i32,
    angle_size: i32,
) -> View<M> {
    Node::new(WidgetKind::ArcLabel {
        text: text.into(),
        radius,
        start_angle,
        angle_size,
    })
}

/// A chart with owned data, explicit axes, stable series identities and cursors.
#[cfg(feature = "widget-chart")]
pub fn chart<M>(spec: crate::ChartSpec) -> View<M> {
    Node::new(WidgetKind::Chart(alloc::boxed::Box::new(spec)))
}
