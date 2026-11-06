//! Typed widget descriptions and feature-gated properties.

use super::{
    CalendarDate, ChartSeries, ChartType, Color, Font, GridTrack, ImageSource, RollerMode, Span,
    TextOverflow, Tile,
};
use crate::custom::CustomWidgetKind;
use alloc::{string::String, vec::Vec};

/// The native widget described by a [`crate::Node`].
///
/// The enum is non-exhaustive: new LVGL widgets are added over time.
#[non_exhaustive]
pub enum WidgetKind {
    #[cfg(feature = "widget-ime-pinyin")]
    PinyinKeyboard {
        target: u64,
        font: Font,
    },
    #[cfg(feature = "widget-qrcode")]
    QrCode {
        code: crate::QrCode,
    },
    #[cfg(feature = "widget-barcode")]
    Barcode {
        code: crate::Barcode,
    },
    #[cfg(feature = "widget-gif")]
    Gif {
        animation: crate::Gif,
    },
    #[cfg(feature = "widget-lottie")]
    Lottie {
        animation: crate::Lottie,
    },

    #[cfg(feature = "widget-canvas")]
    Canvas {
        frame: crate::Canvas,
    },
    Container,
    Row,
    Column,
    Grid {
        columns: Vec<GridTrack>,
        rows: Vec<GridTrack>,
    },
    /// A native LVGL tabview. Its children are the tab page roots, one per
    /// title in `tabs`.
    #[cfg(feature = "widget-tabview")]
    Tabview {
        tabs: Vec<String>,
        active: u32,
    },
    Label {
        text: String,
        text_overflow: TextOverflow,
    },
    Button {
        text: String,
    },
    #[cfg(feature = "widget-slider")]
    Slider {
        min: i32,
        max: i32,
        value: i32,
    },
    #[cfg(feature = "widget-bar")]
    ProgressBar {
        min: i32,
        max: i32,
        value: i32,
    },
    #[cfg(feature = "widget-switch")]
    Switch {
        checked: bool,
    },
    #[cfg(feature = "widget-image")]
    Image {
        source: ImageSource,
        /// LVGL zoom factor: 256 is 100%.
        scale: crate::Scale,
    },
    #[cfg(feature = "widget-checkbox")]
    Checkbox {
        text: String,
        checked: bool,
    },
    #[cfg(feature = "widget-textarea")]
    Textarea {
        text: String,
        placeholder: String,
        one_line: bool,
        password: bool,
    },
    #[cfg(feature = "widget-dropdown")]
    Dropdown {
        options: Vec<String>,
        selected: u32,
    },
    /// A gauge-style arc: the background arc and knob are hidden, and the
    /// indicator uses the style's `arc_color`/`arc_width` when set.
    #[cfg(feature = "widget-arc")]
    Arc {
        min: i32,
        max: i32,
        value: i32,
    },
    #[cfg(feature = "widget-scale")]
    Scale {
        min: i32,
        max: i32,
        total_ticks: u32,
        major_tick_every: u32,
    },
    #[cfg(feature = "widget-chart")]
    Chart {
        chart_type: ChartType,
        /// Primary Y-axis minimum.
        min: i32,
        /// Primary Y-axis maximum.
        max: i32,
        series: Vec<ChartSeries>,
    },
    /// A keyboard bound to the textarea registered with `keyboard_id`.
    #[cfg(feature = "widget-keyboard")]
    Keyboard {
        target: u64,
    },
    /// A scrollable option list; the selected option is controlled.
    #[cfg(feature = "widget-roller")]
    Roller {
        options: Vec<String>,
        selected: u32,
        visible_rows: u32,
        mode: RollerMode,
    },
    /// A numeric field. `value` is an integer; `decimal_pos` only places the
    /// decimal separator when displaying it.
    #[cfg(feature = "widget-spinbox")]
    Spinbox {
        min: i32,
        max: i32,
        value: i32,
        digit_count: u32,
        decimal_pos: u32,
        step: u32,
    },
    /// An animated busy indicator.
    #[cfg(feature = "widget-spinner")]
    Spinner {
        duration_ms: u32,
        arc_angle: u32,
    },
    /// A light-emitting diode indicator.
    #[cfg(feature = "widget-led")]
    Led {
        color: Color,
        brightness: u8,
    },
    /// A polyline through `points`, in pixels relative to the widget.
    #[cfg(feature = "widget-line")]
    Line {
        points: Vec<(i32, i32)>,
    },
    /// A grid of text buttons, one inner vector per row.
    #[cfg(feature = "widget-buttonmatrix")]
    ButtonMatrix {
        rows: Vec<Vec<String>>,
    },
    /// A text table, one inner vector per row. Every row has the same number
    /// of cells.
    #[cfg(feature = "widget-table")]
    Table {
        cells: Vec<Vec<String>>,
        column_widths: Vec<i32>,
    },
    /// A swipeable grid of full-size tiles. Its children are the tile roots,
    /// one per entry in `tiles`.
    #[cfg(feature = "widget-tileview")]
    Tileview {
        tiles: Vec<Tile>,
        active: u32,
    },
    /// An inline message box: an optional title, a text, optional footer
    /// buttons, and children placed below the text in its content area.
    #[cfg(feature = "widget-msgbox")]
    MessageBox {
        title: Option<String>,
        text: String,
        buttons: Vec<String>,
    },
    /// A month calendar. The shown month is controlled by the application.
    #[cfg(feature = "widget-calendar")]
    Calendar {
        year: u16,
        month: u8,
        today: Option<CalendarDate>,
        highlighted: Vec<CalendarDate>,
    },
    /// Rich text made of individually styled spans.
    #[cfg(feature = "widget-span")]
    Spans {
        spans: Vec<Span>,
    },
    /// A button drawn from images.
    #[cfg(feature = "widget-imagebutton")]
    ImageButton {
        released: ImageSource,
        pressed: Option<ImageSource>,
    },
    /// An image that cycles through `frames`.
    #[cfg(feature = "widget-animimage")]
    AnimatedImage {
        frames: Vec<ImageSource>,
        duration_ms: u32,
        repeat: u32,
    },
    /// Text laid out along an arc.
    #[cfg(feature = "widget-arclabel")]
    ArcLabel {
        text: String,
        radius: u32,
        start_angle: i32,
        angle_size: i32,
    },
    /// An opaque native subtree: either a static `crate::raw` factory or a
    /// lifecycle widget supplied to `crate::custom`.
    Custom(CustomWidgetKind),
}
