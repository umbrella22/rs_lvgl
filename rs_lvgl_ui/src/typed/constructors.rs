//! Typed adapters share the ordinary constructors and descriptor storage.
use super::kind;
use crate::binding::{NumericInput, TextInput, ToggleInput};
use crate::prelude::*;
use crate::Node;
use alloc::{string::String, vec::Vec};

/// Typed version of [`crate::container`].
pub fn container<M>() -> Node<M, kind::Container> {
    crate::container().cast_kind()
}

/// Typed version of [`crate::row`].
pub fn row<M>(children: impl IntoIterator<Item = View<M>>) -> Node<M, kind::Row> {
    crate::row(children).cast_kind()
}

/// Typed version of [`crate::column`].
pub fn column<M>(children: impl IntoIterator<Item = View<M>>) -> Node<M, kind::Column> {
    crate::column(children).cast_kind()
}

/// Typed version of [`crate::grid`].
pub fn grid<M>(
    columns: impl IntoIterator<Item = GridTrack>,
    rows: impl IntoIterator<Item = GridTrack>,
    children: impl IntoIterator<Item = View<M>>,
) -> Node<M, kind::Grid> {
    crate::grid(columns, rows, children).cast_kind()
}

#[cfg(feature = "widget-tabview")]
/// Typed version of [`crate::tabview`].
pub fn tabview<M>(
    tabs: impl IntoIterator<Item = impl Into<String>>,
    active: u32,
) -> Node<M, kind::Tabview> {
    crate::tabview(tabs, active).cast_kind()
}

#[cfg(feature = "widget-tileview")]
/// Typed version of [`crate::tileview`].
pub fn tileview<M>(tiles: impl IntoIterator<Item = Tile>, active: u32) -> Node<M, kind::Tileview> {
    crate::tileview(tiles, active).cast_kind()
}

/// Typed version of [`crate::label`].
pub fn label<M>(text: impl Into<String>) -> Node<M, kind::Label> {
    crate::label(text).cast_kind()
}

/// Typed version of [`crate::button`].
pub fn button<M>(text: impl Into<String>) -> Node<M, kind::Button> {
    crate::button(text).cast_kind()
}

/// Typed version of [`crate::label_static`].
pub fn label_static<M>(text: &'static str) -> Node<M, kind::Label> {
    crate::label_static(text).cast_kind()
}

/// Typed version of [`crate::button_static`].
pub fn button_static<M>(text: &'static str) -> Node<M, kind::Button> {
    crate::button_static(text).cast_kind()
}

#[cfg(feature = "widget-slider")]
/// Typed version of [`crate::slider`].
pub fn slider<M>(min: i32, max: i32, value: impl NumericInput<M>) -> Node<M, kind::Slider> {
    value.into_numeric_node(|value| crate::slider(min, max, value).cast_kind())
}

#[cfg(feature = "widget-bar")]
/// Typed version of [`crate::progress_bar`].
pub fn progress_bar<M>(
    min: i32,
    max: i32,
    value: impl NumericInput<M>,
) -> Node<M, kind::ProgressBar> {
    value.into_numeric_node(|value| crate::progress_bar(min, max, value).cast_kind())
}

#[cfg(feature = "widget-switch")]
/// Typed version of [`crate::switch`].
pub fn switch<M>(checked: impl ToggleInput<M>) -> Node<M, kind::Switch> {
    checked.into_toggle_node(|checked| crate::switch(checked).cast_kind())
}

#[cfg(feature = "widget-checkbox")]
/// Typed version of [`crate::checkbox`].
pub fn checkbox<M>(
    text: impl Into<String>,
    checked: impl ToggleInput<M>,
) -> Node<M, kind::Checkbox> {
    checked.into_toggle_node(|checked| crate::checkbox(text, checked).cast_kind())
}

#[cfg(feature = "widget-textarea")]
/// Typed version of [`crate::textarea`].
pub fn textarea<M>(text: impl TextInput<M>) -> Node<M, kind::Textarea> {
    text.into_text_node(|text| crate::textarea(text).cast_kind())
}

#[cfg(feature = "widget-dropdown")]
/// Typed version of [`crate::dropdown`].
pub fn dropdown<M>(
    options: impl IntoIterator<Item = impl Into<String>>,
    selected: u32,
) -> Node<M, kind::Dropdown> {
    crate::dropdown(options, selected).cast_kind()
}

#[cfg(feature = "widget-arc")]
/// Typed version of [`crate::arc`].
pub fn arc<M>(min: i32, max: i32, value: impl NumericInput<M>) -> Node<M, kind::Arc> {
    value.into_numeric_node(|value| crate::arc(min, max, value).cast_kind())
}

#[cfg(feature = "widget-keyboard")]
/// Typed version of [`crate::keyboard`].
pub fn keyboard<M>(target: u64) -> Node<M, kind::Keyboard> {
    crate::keyboard(target).cast_kind()
}

#[cfg(feature = "widget-roller")]
/// Typed version of [`crate::roller`].
pub fn roller<M>(
    options: impl IntoIterator<Item = impl Into<String>>,
    selected: u32,
) -> Node<M, kind::Roller> {
    crate::roller(options, selected).cast_kind()
}

#[cfg(feature = "widget-spinbox")]
/// Typed version of [`crate::spinbox`].
pub fn spinbox<M>(min: i32, max: i32, value: impl NumericInput<M>) -> Node<M, kind::Spinbox> {
    value.into_numeric_node(|value| crate::spinbox(min, max, value).cast_kind())
}

#[cfg(feature = "widget-buttonmatrix")]
/// Typed version of [`crate::button_matrix`].
pub fn button_matrix<M, R, T>(rows: impl IntoIterator<Item = R>) -> Node<M, kind::ButtonMatrix>
where
    R: IntoIterator<Item = T>,
    T: Into<String>,
{
    crate::button_matrix(rows).cast_kind()
}

#[cfg(feature = "widget-ime-pinyin")]
/// Typed version of [`crate::pinyin_keyboard`].
pub fn pinyin_keyboard<M>(target: u64, font: Font) -> Node<M, kind::PinyinKeyboard> {
    crate::pinyin_keyboard(target, font).cast_kind()
}

#[cfg(feature = "widget-scale")]
/// Typed version of [`crate::scale`].
pub fn scale<M>(
    min: i32,
    max: i32,
    total_ticks: u32,
    major_tick_every: u32,
) -> Node<M, kind::Scale> {
    crate::scale(min, max, total_ticks, major_tick_every).cast_kind()
}

#[cfg(feature = "widget-spinner")]
/// Typed version of [`crate::spinner`].
pub fn spinner<M>() -> Node<M, kind::Spinner> {
    crate::spinner().cast_kind()
}

#[cfg(feature = "widget-led")]
/// Typed version of [`crate::led`].
pub fn led<M>(color: Color) -> Node<M, kind::Led> {
    crate::led(color).cast_kind()
}

#[cfg(feature = "widget-line")]
/// Typed version of [`crate::line`].
pub fn line<M>(points: impl IntoIterator<Item = (i32, i32)>) -> Node<M, kind::Line> {
    crate::line(points).cast_kind()
}

#[cfg(feature = "widget-table")]
/// Typed version of [`crate::table`].
pub fn table<M, R, T>(cells: impl IntoIterator<Item = R>) -> Node<M, kind::Table>
where
    R: IntoIterator<Item = T>,
    T: Into<String>,
{
    crate::table(cells).cast_kind()
}

#[cfg(feature = "widget-msgbox")]
/// Typed version of [`crate::message_box`].
pub fn message_box<M>(text: impl Into<String>) -> Node<M, kind::MessageBox> {
    crate::message_box(text).cast_kind()
}

#[cfg(feature = "widget-calendar")]
/// Typed version of [`crate::calendar`].
pub fn calendar<M>(year: u16, month: u8) -> Node<M, kind::Calendar> {
    crate::calendar(year, month).cast_kind()
}

#[cfg(feature = "widget-span")]
/// Typed version of [`crate::spans`].
pub fn spans<M>(spans: impl IntoIterator<Item = Span>) -> Node<M, kind::Spans> {
    crate::spans(spans).cast_kind()
}

#[cfg(feature = "widget-arclabel")]
/// Typed version of [`crate::arc_label`].
pub fn arc_label<M>(
    text: impl Into<String>,
    radius: u32,
    start_angle: i32,
    angle_size: i32,
) -> Node<M, kind::ArcLabel> {
    crate::arc_label(text, radius, start_angle, angle_size).cast_kind()
}

#[cfg(feature = "widget-chart")]
/// Typed version of [`crate::chart`].
pub fn chart<M>(
    chart_type: ChartType,
    min: i32,
    max: i32,
    series: Vec<ChartSeries>,
) -> Node<M, kind::Chart> {
    crate::chart(chart_type, min, max, series).cast_kind()
}

#[cfg(feature = "widget-image")]
/// Typed version of [`crate::image`].
pub fn image<M>(source: ImageSource) -> Node<M, kind::Image> {
    crate::image(source).cast_kind()
}

#[cfg(feature = "widget-image")]
/// Typed version of [`crate::image_scaled`].
pub fn image_scaled<M>(source: ImageSource, scale: crate::Scale) -> Node<M, kind::Image> {
    crate::image_scaled(source, scale).cast_kind()
}

#[cfg(feature = "widget-image")]
/// Typed version of [`crate::image_resource`].
pub fn image_resource<M>(source: crate::ImageResource) -> Node<M, kind::Image> {
    crate::image_resource(source).cast_kind()
}

#[cfg(feature = "widget-image")]
/// Typed version of [`crate::image_resource_scaled`].
pub fn image_resource_scaled<M>(
    source: crate::ImageResource,
    scale: crate::Scale,
) -> Node<M, kind::Image> {
    crate::image_resource_scaled(source, scale).cast_kind()
}

#[cfg(feature = "widget-imagebutton")]
/// Typed version of [`crate::image_button`].
pub fn image_button<M>(released: ImageSource) -> Node<M, kind::ImageButton> {
    crate::image_button(released).cast_kind()
}

#[cfg(feature = "widget-animimage")]
/// Typed version of [`crate::animated_image`].
pub fn animated_image<M>(
    frames: impl IntoIterator<Item = ImageSource>,
    duration_ms: u32,
) -> Node<M, kind::AnimatedImage> {
    crate::animated_image(frames, duration_ms).cast_kind()
}

#[cfg(feature = "widget-canvas")]
/// Typed version of [`crate::canvas`].
pub fn canvas<M>(frame: Canvas) -> Node<M, kind::Canvas> {
    crate::canvas(frame).cast_kind()
}

#[cfg(feature = "widget-qrcode")]
/// Typed version of [`crate::qrcode`].
pub fn qrcode<M>(code: QrCode) -> Node<M, kind::QrCode> {
    crate::qrcode(code).cast_kind()
}

#[cfg(feature = "widget-barcode")]
/// Typed version of [`crate::barcode`].
pub fn barcode<M>(code: Barcode) -> Node<M, kind::Barcode> {
    crate::barcode(code).cast_kind()
}

#[cfg(feature = "widget-gif")]
/// Typed version of [`crate::gif`].
pub fn gif<M>(animation: Gif) -> Node<M, kind::Gif> {
    crate::gif(animation).cast_kind()
}

#[cfg(feature = "widget-lottie")]
/// Typed version of [`crate::lottie`].
pub fn lottie<M>(animation: Lottie) -> Node<M, kind::Lottie> {
    crate::lottie(animation).cast_kind()
}
