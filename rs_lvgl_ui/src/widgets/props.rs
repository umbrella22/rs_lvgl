//! Retained property projection and structural replacement decisions.

use crate::custom::CustomWidgetKind;
use crate::node::{
    CalendarDate, ChartSeries, ChartType, Color, GridTrack, ImageSource, RollerMode, Span,
    TextOverflow, Tile, WidgetKind,
};
use alloc::{borrow::Cow, string::String, vec::Vec};

pub(crate) enum Props {
    #[cfg(feature = "widget-ime-pinyin")]
    PinyinKeyboard {
        target: u64,
        font: crate::Font,
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
    #[cfg(feature = "widget-tabview")]
    Tabview {
        tabs: Vec<String>,
        active: u32,
    },
    #[cfg(feature = "widget-keyboard")]
    Keyboard {
        target: u64,
    },
    Label {
        text: Cow<'static, str>,
        text_overflow: TextOverflow,
    },
    Button {
        text: Cow<'static, str>,
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
        min: i32,
        max: i32,
        series: Vec<ChartSeries>,
    },
    #[cfg(feature = "widget-roller")]
    Roller {
        options: Vec<String>,
        selected: u32,
        visible_rows: u32,
        mode: RollerMode,
    },
    #[cfg(feature = "widget-spinbox")]
    Spinbox {
        min: i32,
        max: i32,
        value: i32,
        digit_count: u32,
        decimal_pos: u32,
        step: u32,
    },
    #[cfg(feature = "widget-spinner")]
    Spinner {
        duration_ms: u32,
        arc_angle: u32,
    },
    #[cfg(feature = "widget-led")]
    Led {
        color: Color,
        brightness: u8,
    },
    #[cfg(feature = "widget-line")]
    Line {
        points: Vec<(i32, i32)>,
    },
    #[cfg(feature = "widget-buttonmatrix")]
    ButtonMatrix {
        rows: Vec<Vec<String>>,
    },
    #[cfg(feature = "widget-table")]
    Table {
        cells: Vec<Vec<String>>,
        column_widths: Vec<i32>,
    },
    #[cfg(feature = "widget-tileview")]
    Tileview {
        tiles: Vec<Tile>,
        active: u32,
    },
    #[cfg(feature = "widget-msgbox")]
    MessageBox {
        title: Option<String>,
        text: String,
        buttons: Vec<String>,
    },
    #[cfg(feature = "widget-calendar")]
    Calendar {
        year: u16,
        month: u8,
        today: Option<CalendarDate>,
        highlighted: Vec<CalendarDate>,
    },
    #[cfg(feature = "widget-span")]
    Spans {
        spans: Vec<Span>,
    },
    #[cfg(feature = "widget-imagebutton")]
    ImageButton {
        released: ImageSource,
        pressed: Option<ImageSource>,
    },
    #[cfg(feature = "widget-animimage")]
    AnimatedImage {
        frames: Vec<ImageSource>,
        duration_ms: u32,
        repeat: u32,
    },
    #[cfg(feature = "widget-arclabel")]
    ArcLabel {
        text: String,
        radius: u32,
        start_angle: i32,
        angle_size: i32,
    },
    Custom,
}

pub(crate) fn retain_static_text(props: &mut Props, text: Option<&'static str>) {
    if let Some(text) = text {
        match props {
            Props::Label {
                text: destination, ..
            }
            | Props::Button { text: destination } => *destination = Cow::Borrowed(text),
            _ => unreachable!("only label/button descriptors carry static text"),
        }
    }
}

/// Whether a kind change cannot be patched in place and the native subtree
/// must be recreated.
pub(crate) fn needs_recreate(old: &Props, new: &WidgetKind) -> bool {
    match (old, new) {
        #[cfg(feature = "widget-lottie")]
        (Props::Lottie { animation: old }, WidgetKind::Lottie { animation: new }) => old != new,
        #[cfg(feature = "widget-tabview")]
        (Props::Tabview { tabs: old, .. }, WidgetKind::Tabview { tabs: new, .. }) => {
            old.len() != new.len()
        }
        // Tiles are positioned when they are created.
        #[cfg(feature = "widget-tileview")]
        (Props::Tileview { tiles: old, .. }, WidgetKind::Tileview { tiles: new, .. }) => old != new,
        // LVGL can add a message box header and footer buttons but not
        // remove them.
        #[cfg(feature = "widget-msgbox")]
        (
            Props::MessageBox {
                title: old_title,
                buttons: old_buttons,
                ..
            },
            WidgetKind::MessageBox { title, buttons, .. },
        ) => old_title.is_some() != title.is_some() || old_buttons.len() != buttons.len(),
        _ => false,
    }
}

/// Move a descriptor's kind into retained props.
///
/// The custom factory, if any, is returned separately because the mounted
/// node owns it outside `Props`.
pub(crate) fn into_props(kind: WidgetKind) -> (Props, Option<CustomWidgetKind>) {
    let props = match kind {
        #[cfg(feature = "widget-ime-pinyin")]
        WidgetKind::PinyinKeyboard { target, font } => Props::PinyinKeyboard { target, font },
        #[cfg(feature = "widget-qrcode")]
        WidgetKind::QrCode { code } => Props::QrCode { code },
        #[cfg(feature = "widget-barcode")]
        WidgetKind::Barcode { code } => Props::Barcode { code },
        #[cfg(feature = "widget-gif")]
        WidgetKind::Gif { animation } => Props::Gif { animation },
        #[cfg(feature = "widget-lottie")]
        WidgetKind::Lottie { animation } => Props::Lottie { animation },

        #[cfg(feature = "widget-canvas")]
        WidgetKind::Canvas { frame } => Props::Canvas { frame },
        WidgetKind::Container => Props::Container,
        WidgetKind::Row => Props::Row,
        WidgetKind::Column => Props::Column,
        WidgetKind::Grid { columns, rows } => Props::Grid { columns, rows },
        #[cfg(feature = "widget-tabview")]
        WidgetKind::Tabview { tabs, active } => Props::Tabview { tabs, active },
        #[cfg(feature = "widget-keyboard")]
        WidgetKind::Keyboard { target } => Props::Keyboard { target },
        WidgetKind::Label {
            text,
            text_overflow,
        } => Props::Label {
            text: text.into(),
            text_overflow,
        },
        WidgetKind::Button { text } => Props::Button { text: text.into() },
        #[cfg(feature = "widget-slider")]
        WidgetKind::Slider { min, max, value } => Props::Slider { min, max, value },
        #[cfg(feature = "widget-bar")]
        WidgetKind::ProgressBar { min, max, value } => Props::ProgressBar { min, max, value },
        #[cfg(feature = "widget-switch")]
        WidgetKind::Switch { checked } => Props::Switch { checked },
        #[cfg(feature = "widget-image")]
        WidgetKind::Image { source, scale } => Props::Image { source, scale },
        #[cfg(feature = "widget-checkbox")]
        WidgetKind::Checkbox { text, checked } => Props::Checkbox { text, checked },
        #[cfg(feature = "widget-textarea")]
        WidgetKind::Textarea {
            text,
            placeholder,
            one_line,
            password,
        } => Props::Textarea {
            text,
            placeholder,
            one_line,
            password,
        },
        #[cfg(feature = "widget-dropdown")]
        WidgetKind::Dropdown { options, selected } => Props::Dropdown { options, selected },
        #[cfg(feature = "widget-arc")]
        WidgetKind::Arc { min, max, value } => Props::Arc { min, max, value },
        #[cfg(feature = "widget-scale")]
        WidgetKind::Scale {
            min,
            max,
            total_ticks,
            major_tick_every,
        } => Props::Scale {
            min,
            max,
            total_ticks,
            major_tick_every,
        },
        #[cfg(feature = "widget-chart")]
        WidgetKind::Chart {
            chart_type,
            min,
            max,
            series,
        } => Props::Chart {
            chart_type,
            min,
            max,
            series,
        },
        #[cfg(feature = "widget-roller")]
        WidgetKind::Roller {
            options,
            selected,
            visible_rows,
            mode,
        } => Props::Roller {
            options,
            selected,
            visible_rows,
            mode,
        },
        #[cfg(feature = "widget-spinbox")]
        WidgetKind::Spinbox {
            min,
            max,
            value,
            digit_count,
            decimal_pos,
            step,
        } => Props::Spinbox {
            min,
            max,
            value,
            digit_count,
            decimal_pos,
            step,
        },
        #[cfg(feature = "widget-spinner")]
        WidgetKind::Spinner {
            duration_ms,
            arc_angle,
        } => Props::Spinner {
            duration_ms,
            arc_angle,
        },
        #[cfg(feature = "widget-led")]
        WidgetKind::Led { color, brightness } => Props::Led { color, brightness },
        #[cfg(feature = "widget-line")]
        WidgetKind::Line { points } => Props::Line { points },
        #[cfg(feature = "widget-buttonmatrix")]
        WidgetKind::ButtonMatrix { rows } => Props::ButtonMatrix { rows },
        #[cfg(feature = "widget-table")]
        WidgetKind::Table {
            cells,
            column_widths,
        } => Props::Table {
            cells,
            column_widths,
        },
        #[cfg(feature = "widget-tileview")]
        WidgetKind::Tileview { tiles, active } => Props::Tileview { tiles, active },
        #[cfg(feature = "widget-msgbox")]
        WidgetKind::MessageBox {
            title,
            text,
            buttons,
        } => Props::MessageBox {
            title,
            text,
            buttons,
        },
        #[cfg(feature = "widget-calendar")]
        WidgetKind::Calendar {
            year,
            month,
            today,
            highlighted,
        } => Props::Calendar {
            year,
            month,
            today,
            highlighted,
        },
        #[cfg(feature = "widget-span")]
        WidgetKind::Spans { spans } => Props::Spans { spans },
        #[cfg(feature = "widget-imagebutton")]
        WidgetKind::ImageButton { released, pressed } => Props::ImageButton { released, pressed },
        #[cfg(feature = "widget-animimage")]
        WidgetKind::AnimatedImage {
            frames,
            duration_ms,
            repeat,
        } => Props::AnimatedImage {
            frames,
            duration_ms,
            repeat,
        },
        #[cfg(feature = "widget-arclabel")]
        WidgetKind::ArcLabel {
            text,
            radius,
            start_angle,
            angle_size,
        } => Props::ArcLabel {
            text,
            radius,
            start_angle,
            angle_size,
        },
        WidgetKind::Custom(custom) => return (Props::Custom, Some(custom)),
    };
    (props, None)
}
