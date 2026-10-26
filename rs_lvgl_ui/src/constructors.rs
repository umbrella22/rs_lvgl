//! Ordinary constructors for declarative views.
//!
//! These functions only build descriptors. Native objects are created when a
//! [`Ui`](crate::Ui) mounts or updates the view. Optional constructors use the
//! same `widget-*` features as their native widgets and view modifiers.

use crate::custom::{CustomWidget, CustomWidgetKind, RawWidget};
use crate::node::{
    ChartSeries, ChartType, Color, GridTrack, ImageSource, LabelLongMode, Node, RollerMode, Span,
    Tile, WidgetKind, ANIM_REPEAT_INFINITE,
};
#[cfg(feature = "widget-barcode")]
use crate::Barcode;
#[cfg(feature = "widget-canvas")]
use crate::Canvas;
#[cfg(feature = "widget-gif")]
use crate::Gif;
#[cfg(feature = "widget-lottie")]
use crate::Lottie;
#[cfg(feature = "widget-qrcode")]
use crate::QrCode;
use crate::View;
#[cfg(feature = "widget-ime-pinyin")]
use crate::{Font, Size, Style};
use alloc::{boxed::Box, string::String, vec::Vec};
use core::ptr::NonNull;
use rs_lvgl_sys::lv_obj_t;

/// An unstyled container for absolute positioning.
pub fn obj<M>() -> View<M> {
    Node::new(WidgetKind::Obj)
}

/// A horizontal flex container with the supplied children.
///
/// Arrays and iterators preserve their iteration order. See [`column()`] for
/// composition with named child views and ordinary helper functions.
pub fn row<M>(children: impl IntoIterator<Item = View<M>>) -> View<M> {
    Node::new(WidgetKind::Row).children(children)
}

/// A vertical flex container with the supplied children.
///
/// Children in the normal flex flow appear from top to bottom in iteration
/// order. Their sizes, spacing and alignment determine the pixel positions.
/// Extracting a child into a variable or function does not change its position;
/// the order passed to `column` determines placement.
///
/// ```
/// use rs_lvgl_ui::prelude::*;
///
/// fn page_title<M>(text: &str, theme: &Theme) -> View<M> {
///     label(text).style(Style::title(theme))
/// }
///
/// fn settings_view(theme: &Theme) -> View<()> {
///     let content = label("Notification preferences");
///     let title = page_title("Settings", theme);
///     column([title, content]).style(Style::default().gap(12))
/// }
/// # let _ = settings_view(&Theme::light());
/// ```
///
/// A helper returns a view description; it does not create independent state or
/// a render cache. Child order is separate from identity: use stable
/// [`Key`](crate::Key) values for children that can be inserted, removed or
/// reordered. The default `Key::Auto` matches by position among `Auto` siblings.
pub fn column<M>(children: impl IntoIterator<Item = View<M>>) -> View<M> {
    Node::new(WidgetKind::Column).children(children)
}

/// A grid with column tracks, row tracks and child views.
pub fn grid<M>(
    columns: impl IntoIterator<Item = GridTrack>,
    rows: impl IntoIterator<Item = GridTrack>,
    children: impl IntoIterator<Item = View<M>>,
) -> View<M> {
    Node::new(WidgetKind::Grid {
        columns: columns.into_iter().collect(),
        rows: rows.into_iter().collect(),
    })
    .children(children)
}

/// A native tabview. Add exactly one child per tab; each child becomes the
/// root of that tab's page.
#[cfg(feature = "widget-tabview")]
pub fn tabview<M>(tabs: impl IntoIterator<Item = impl Into<String>>, active: u32) -> View<M> {
    Node::new(WidgetKind::Tabview {
        tabs: tabs.into_iter().map(Into::into).collect(),
        active,
    })
}

/// A text label that wraps long text.
pub fn label<M>(text: impl Into<String>) -> View<M> {
    Node::new(WidgetKind::Label {
        text: text.into(),
        long_mode: LabelLongMode::Wrap,
    })
}

/// A native button with a text label.
pub fn button<M>(text: impl Into<String>) -> View<M> {
    Node::new(WidgetKind::Button { text: text.into() })
}

/// A controlled slider with a value in `min..=max`.
#[cfg(feature = "widget-slider")]
pub fn slider<M>(min: i32, max: i32, value: i32) -> View<M> {
    Node::new(WidgetKind::Slider { min, max, value })
}

/// A progress bar with a value in `min..=max`.
#[cfg(feature = "widget-bar")]
pub fn bar<M>(min: i32, max: i32, value: i32) -> View<M> {
    Node::new(WidgetKind::Bar { min, max, value })
}

/// A switch with a controlled checked state.
#[cfg(feature = "widget-switch")]
pub fn switch<M>(checked: bool) -> View<M> {
    Node::new(WidgetKind::Switch { checked })
}

/// An image at its original scale.
#[cfg(feature = "widget-image")]
pub fn image<M>(source: ImageSource) -> View<M> {
    Node::new(WidgetKind::Image { source, scale: 256 })
}

/// An image with a scale of 256 representing its original size.
#[cfg(feature = "widget-image")]
pub fn image_scaled<M>(source: ImageSource, scale: u16) -> View<M> {
    Node::new(WidgetKind::Image { source, scale })
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

/// A tileview. Add exactly one child per tile; each child becomes the
/// root of that tile. `active` indexes `tiles` and is controlled.
#[cfg(feature = "widget-tileview")]
pub fn tileview<M>(tiles: impl IntoIterator<Item = Tile>, active: u32) -> View<M> {
    Node::new(WidgetKind::Tileview {
        tiles: tiles.into_iter().collect(),
        active,
    })
}

/// An inline message box showing `text`. Children are placed below the
/// text. Add a title with [`View::msgbox_title`] and footer buttons with
/// [`View::msgbox_buttons`].
#[cfg(feature = "widget-msgbox")]
pub fn msgbox<M>(text: impl Into<String>) -> View<M> {
    Node::new(WidgetKind::Msgbox {
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

/// A button drawn with `released`; add a pressed image with
/// [`View::pressed_image`].
#[cfg(feature = "widget-imagebutton")]
pub fn image_button<M>(released: ImageSource) -> View<M> {
    Node::new(WidgetKind::ImageButton {
        released,
        pressed: None,
    })
}

/// An image that shows every frame once per `duration_ms`, looping
/// forever unless [`View::repeat`] is set.
#[cfg(feature = "widget-animimage")]
pub fn anim_image<M>(frames: impl IntoIterator<Item = ImageSource>, duration_ms: u32) -> View<M> {
    Node::new(WidgetKind::AnimImage {
        frames: frames.into_iter().collect(),
        duration_ms,
        repeat: ANIM_REPEAT_INFINITE,
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

/// A chart with one or more series.
///
/// All series must have the same number of points. The primary Y axis is
/// fixed to `min..=max`.
#[cfg(feature = "widget-chart")]
pub fn chart<M>(chart_type: ChartType, min: i32, max: i32, series: Vec<ChartSeries>) -> View<M> {
    Node::new(WidgetKind::Chart {
        chart_type,
        min,
        max,
        series,
    })
}

/// Create an opaque native subtree.
///
/// The reconciler calls `create` with a valid LVGL parent and never
/// traverses the returned object's children. Deletion happens when the
/// mounted node is removed or the UI is unmounted.
///
/// # Safety
///
/// The returned object must be a live direct child of `parent`, must remain
/// valid until LVGL deletes the mounted subtree, and must not be deleted by
/// the caller. The factory must not call back into [`crate::Ui`]. Any native
/// callbacks it registers are its own responsibility.
pub unsafe fn raw<M, F>(create: F) -> View<M>
where
    F: Fn(NonNull<lv_obj_t>) -> Option<NonNull<lv_obj_t>> + 'static,
{
    Node::new(WidgetKind::Custom(CustomWidgetKind::raw(RawWidget::new(
        Box::new(create),
    ))))
}

/// Create a custom widget with a native lifecycle.
///
/// The widget's [`CustomWidget::create`] runs when the node is mounted;
/// [`CustomWidget::update`] runs once per [`crate::Ui::tick`];
/// [`CustomWidget::delete`] runs exactly once before the native object is
/// deleted by a key change, a kind change, subtree removal or
/// [`crate::Ui::unmount`].
///
/// The mounted tree keeps the widget instance. A widget supplied by a later
/// `App::view` call replaces it only when the node key or the custom
/// variant changes; otherwise the new descriptor instance is dropped. Use a
/// key change to swap implementations, or keep shared application state in
/// an `Rc`/`RefCell` visible to both the widget and the application.
///
/// Messages from native callbacks are delivered through the node's explicit
/// handler (`on_event`, `on_click`, ...), which is registered on the custom
/// object.
pub fn custom<M>(widget: impl CustomWidget) -> View<M> {
    Node::new(WidgetKind::Custom(CustomWidgetKind::lifecycle(widget)))
}

/// A canvas rendered from an owned frame description.
#[cfg(feature = "widget-canvas")]
pub fn canvas<M>(frame: Canvas) -> View<M> {
    Node::new(WidgetKind::Canvas { frame })
}

/// A QR code with a length-delimited binary payload.
#[cfg(feature = "widget-qrcode")]
pub fn qrcode<M>(code: QrCode) -> View<M> {
    Node::new(WidgetKind::QrCode { code })
}

/// A barcode rendered from its encoding and appearance description.
#[cfg(feature = "widget-barcode")]
pub fn barcode<M>(code: Barcode) -> View<M> {
    let style = crate::Style::default().height(crate::Size::Px(code.height as i16));
    Node::new(WidgetKind::Barcode { code }).style(style)
}

/// A GIF animation with retained encoded bytes.
#[cfg(feature = "widget-gif")]
pub fn gif<M>(animation: Gif) -> View<M> {
    Node::new(WidgetKind::Gif { animation })
}

/// A Lottie animation with retained JSON and a sized drawing buffer.
#[cfg(feature = "widget-lottie")]
pub fn lottie<M>(animation: Lottie) -> View<M> {
    let style = crate::Style::default()
        .width(crate::Size::Px(animation.width as i16))
        .height(crate::Size::Px(animation.height as i16));
    Node::new(WidgetKind::Lottie { animation }).style(style)
}

/// A 26-key Pinyin IME with LVGL's bundled dictionary, bound to a textarea's
/// keyboard_id. Its candidate font must contain the requested CJK glyphs.
/// LVGL shares candidate text buffers: use one active Pinyin keyboard at a
/// time. The container includes its keyboard and candidate strip.
#[cfg(feature = "widget-ime-pinyin")]
pub fn pinyin_keyboard<M>(target: u64, font: Font) -> View<M> {
    Node::new(WidgetKind::PinyinKeyboard { target, font })
        .style(
            Style::neutral()
                .width(Size::Percent(100))
                .height(Size::Px(200)),
        )
        .scrollable(false)
}
