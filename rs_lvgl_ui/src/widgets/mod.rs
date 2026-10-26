use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::ptr::NonNull;

use cstr_core::CString;
use rs_lvgl_sys::lv_obj_t;

use crate::custom::CustomWidgetKind;
use crate::events::{HandlerSlot, ValueSource};
use crate::node::{
    CalendarDate, ChartSeries, ChartType, Color, GridCell, GridTrack, ImageSource, LabelLongMode,
    Part, RollerMode, Selector, Size, Span, Tile, WidgetKind,
};
use crate::theme::{
    Style, STYLE_ALIGN, STYLE_ANIM_DURATION, STYLE_ARC_COLOR, STYLE_ARC_WIDTH, STYLE_BG,
    STYLE_BG_OPA, STYLE_BORDER, STYLE_DISABLED, STYLE_FG, STYLE_FLEX_ALIGN, STYLE_FLOATING,
    STYLE_FONT, STYLE_GAP, STYLE_GROW, STYLE_HEIGHT, STYLE_HIDDEN, STYLE_MAX_HEIGHT,
    STYLE_MAX_WIDTH, STYLE_MIN_HEIGHT, STYLE_MIN_WIDTH, STYLE_OFFSET, STYLE_OPACITY,
    STYLE_PAD_BOTTOM, STYLE_PAD_LEFT, STYLE_PAD_RIGHT, STYLE_PAD_TOP, STYLE_RADIUS,
    STYLE_TEXT_ALIGN, STYLE_WIDTH,
};
use crate::UiError;

impl KindTag {
    /// Widgets that own their checked state (written through their props,
    /// not the generic `checked` modifier).
    pub(crate) fn owns_checked(self) -> bool {
        match self {
            #[cfg(feature = "widget-switch")]
            KindTag::Switch => true,
            #[cfg(feature = "widget-checkbox")]
            KindTag::Checkbox => true,
            _ => false,
        }
    }

    /// Widgets whose children are pages with a controlled active index.
    pub(crate) fn is_pager(self) -> bool {
        match self {
            #[cfg(feature = "widget-tabview")]
            KindTag::Tabview => true,
            #[cfg(feature = "widget-tileview")]
            KindTag::Tileview => true,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum KindTag {
    #[cfg(feature = "widget-ime-pinyin")]
    PinyinKeyboard,
    #[cfg(feature = "widget-qrcode")]
    QrCode,
    #[cfg(feature = "widget-barcode")]
    Barcode,
    #[cfg(feature = "widget-gif")]
    Gif,
    #[cfg(feature = "widget-lottie")]
    Lottie,

    #[cfg(feature = "widget-canvas")]
    Canvas,
    Obj,
    Row,
    Column,
    Grid,
    #[cfg(feature = "widget-tabview")]
    Tabview,
    #[cfg(feature = "widget-keyboard")]
    Keyboard,
    Label,
    Button,
    #[cfg(feature = "widget-slider")]
    Slider,
    #[cfg(feature = "widget-bar")]
    Bar,
    #[cfg(feature = "widget-switch")]
    Switch,
    #[cfg(feature = "widget-image")]
    Image,
    #[cfg(feature = "widget-checkbox")]
    Checkbox,
    #[cfg(feature = "widget-textarea")]
    Textarea,
    #[cfg(feature = "widget-dropdown")]
    Dropdown,
    #[cfg(feature = "widget-arc")]
    Arc,
    #[cfg(feature = "widget-scale")]
    Scale,
    #[cfg(feature = "widget-chart")]
    Chart,
    #[cfg(feature = "widget-roller")]
    Roller,
    #[cfg(feature = "widget-spinbox")]
    Spinbox,
    #[cfg(feature = "widget-spinner")]
    Spinner,
    #[cfg(feature = "widget-led")]
    Led,
    #[cfg(feature = "widget-line")]
    Line,
    #[cfg(feature = "widget-buttonmatrix")]
    ButtonMatrix,
    #[cfg(feature = "widget-table")]
    Table,
    #[cfg(feature = "widget-tileview")]
    Tileview,
    #[cfg(feature = "widget-msgbox")]
    Msgbox,
    #[cfg(feature = "widget-calendar")]
    Calendar,
    #[cfg(feature = "widget-span")]
    Spans,
    #[cfg(feature = "widget-imagebutton")]
    ImageButton,
    #[cfg(feature = "widget-animimage")]
    AnimImage,
    #[cfg(feature = "widget-arclabel")]
    ArcLabel,
    Custom,
}

pub(crate) fn tag(kind: &WidgetKind) -> KindTag {
    match kind {
        #[cfg(feature = "widget-ime-pinyin")]
        WidgetKind::PinyinKeyboard { .. } => KindTag::PinyinKeyboard,
        #[cfg(feature = "widget-qrcode")]
        WidgetKind::QrCode { .. } => KindTag::QrCode,
        #[cfg(feature = "widget-barcode")]
        WidgetKind::Barcode { .. } => KindTag::Barcode,
        #[cfg(feature = "widget-gif")]
        WidgetKind::Gif { .. } => KindTag::Gif,
        #[cfg(feature = "widget-lottie")]
        WidgetKind::Lottie { .. } => KindTag::Lottie,

        #[cfg(feature = "widget-canvas")]
        WidgetKind::Canvas { .. } => KindTag::Canvas,
        WidgetKind::Obj => KindTag::Obj,
        WidgetKind::Row => KindTag::Row,
        WidgetKind::Column => KindTag::Column,
        WidgetKind::Grid { .. } => KindTag::Grid,
        #[cfg(feature = "widget-tabview")]
        WidgetKind::Tabview { .. } => KindTag::Tabview,
        #[cfg(feature = "widget-keyboard")]
        WidgetKind::Keyboard { .. } => KindTag::Keyboard,
        WidgetKind::Label { .. } => KindTag::Label,
        WidgetKind::Button { .. } => KindTag::Button,
        #[cfg(feature = "widget-slider")]
        WidgetKind::Slider { .. } => KindTag::Slider,
        #[cfg(feature = "widget-bar")]
        WidgetKind::Bar { .. } => KindTag::Bar,
        #[cfg(feature = "widget-switch")]
        WidgetKind::Switch { .. } => KindTag::Switch,
        #[cfg(feature = "widget-image")]
        WidgetKind::Image { .. } => KindTag::Image,
        #[cfg(feature = "widget-checkbox")]
        WidgetKind::Checkbox { .. } => KindTag::Checkbox,
        #[cfg(feature = "widget-textarea")]
        WidgetKind::Textarea { .. } => KindTag::Textarea,
        #[cfg(feature = "widget-dropdown")]
        WidgetKind::Dropdown { .. } => KindTag::Dropdown,
        #[cfg(feature = "widget-arc")]
        WidgetKind::Arc { .. } => KindTag::Arc,
        #[cfg(feature = "widget-scale")]
        WidgetKind::Scale { .. } => KindTag::Scale,
        #[cfg(feature = "widget-chart")]
        WidgetKind::Chart { .. } => KindTag::Chart,
        #[cfg(feature = "widget-roller")]
        WidgetKind::Roller { .. } => KindTag::Roller,
        #[cfg(feature = "widget-spinbox")]
        WidgetKind::Spinbox { .. } => KindTag::Spinbox,
        #[cfg(feature = "widget-spinner")]
        WidgetKind::Spinner { .. } => KindTag::Spinner,
        #[cfg(feature = "widget-led")]
        WidgetKind::Led { .. } => KindTag::Led,
        #[cfg(feature = "widget-line")]
        WidgetKind::Line { .. } => KindTag::Line,
        #[cfg(feature = "widget-buttonmatrix")]
        WidgetKind::ButtonMatrix { .. } => KindTag::ButtonMatrix,
        #[cfg(feature = "widget-table")]
        WidgetKind::Table { .. } => KindTag::Table,
        #[cfg(feature = "widget-tileview")]
        WidgetKind::Tileview { .. } => KindTag::Tileview,
        #[cfg(feature = "widget-msgbox")]
        WidgetKind::Msgbox { .. } => KindTag::Msgbox,
        #[cfg(feature = "widget-calendar")]
        WidgetKind::Calendar { .. } => KindTag::Calendar,
        #[cfg(feature = "widget-span")]
        WidgetKind::Spans { .. } => KindTag::Spans,
        #[cfg(feature = "widget-imagebutton")]
        WidgetKind::ImageButton { .. } => KindTag::ImageButton,
        #[cfg(feature = "widget-animimage")]
        WidgetKind::AnimImage { .. } => KindTag::AnimImage,
        #[cfg(feature = "widget-arclabel")]
        WidgetKind::ArcLabel { .. } => KindTag::ArcLabel,
        WidgetKind::Custom(_) => KindTag::Custom,
    }
}

pub(crate) fn value_source(tag: KindTag) -> ValueSource {
    match tag {
        #[cfg(feature = "widget-slider")]
        KindTag::Slider => ValueSource::Slider,
        #[cfg(feature = "widget-bar")]
        KindTag::Bar => ValueSource::Bar,
        #[cfg(feature = "widget-switch")]
        KindTag::Switch => ValueSource::Switch,
        #[cfg(feature = "widget-checkbox")]
        KindTag::Checkbox => ValueSource::Checkbox,
        #[cfg(feature = "widget-dropdown")]
        KindTag::Dropdown => ValueSource::Dropdown,
        #[cfg(feature = "widget-arc")]
        KindTag::Arc => ValueSource::Arc,
        #[cfg(feature = "widget-tabview")]
        KindTag::Tabview => ValueSource::Tabview,
        #[cfg(feature = "widget-roller")]
        KindTag::Roller => ValueSource::Roller,
        #[cfg(feature = "widget-spinbox")]
        KindTag::Spinbox => ValueSource::Spinbox,
        #[cfg(feature = "widget-buttonmatrix")]
        KindTag::ButtonMatrix => ValueSource::ButtonMatrix,
        #[cfg(feature = "widget-table")]
        KindTag::Table => ValueSource::Table,
        #[cfg(feature = "widget-tileview")]
        KindTag::Tileview => ValueSource::Tileview,
        #[cfg(feature = "widget-msgbox")]
        KindTag::Msgbox => ValueSource::EventIndex,
        #[cfg(feature = "widget-calendar")]
        KindTag::Calendar => ValueSource::Calendar,
        _ => ValueSource::None,
    }
}

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
    Obj,
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
        text: String,
        long_mode: LabelLongMode,
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
    Bar {
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
        scale: u16,
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
    Msgbox {
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
    AnimImage {
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

/// Native buffers LVGL references without copying, owned by a mounted node.
///
/// Heap buffers keep their address when the owning `MountedNode` moves.
/// Replacements are installed in LVGL before the previous buffer is dropped.
#[derive(Default)]
// Descriptors stay boxed: LVGL holds pointers to each one, and boxing keeps
// them stable while the vectors grow.
#[allow(clippy::vec_box)]
pub(crate) enum Retained {
    #[cfg(feature = "widget-gif")]
    Gif {
        _data: alloc::rc::Rc<[u8]>,
        _descriptor: Box<rs_lvgl_sys::lv_image_dsc_t>,
    },
    #[cfg(feature = "widget-lottie")]
    Lottie(#[allow(dead_code)] crate::graphics::DrawBuffer),
    #[cfg(feature = "widget-canvas")]
    Canvas(#[allow(dead_code)] crate::graphics::DrawBuffer),
    #[default]
    None,
    // The fields are only held, never read: LVGL reads them through the
    // pointers installed when they were created.
    /// A generated image descriptor (`lv_image_set_src` stores the pointer).
    Image(#[allow(dead_code)] Box<rs_lvgl_sys::lv_image_dsc_t>),
    /// Line points (`lv_line_set_points` stores the pointer).
    Points(#[allow(dead_code)] Vec<rs_lvgl_sys::lv_point_precise_t>),
    /// A button matrix map (`lv_buttonmatrix_set_map` stores the array and
    /// the strings it points to).
    Map {
        _texts: Vec<CString>,
        _pointers: Vec<*const cty::c_char>,
    },
    /// Generated image descriptors referenced by an image button
    /// (`lv_imagebutton_set_src` stores the pointers).
    Images(#[allow(dead_code)] Vec<Box<rs_lvgl_sys::lv_image_dsc_t>>),
    /// Animated image frames (`lv_animimg_set_src` stores the pointer
    /// array, and the image stores the current frame's pointer).
    Frames {
        _descriptors: Vec<Box<rs_lvgl_sys::lv_image_dsc_t>>,
        _pointers: Vec<*const cty::c_void>,
    },
    /// Calendar highlighted dates (`lv_calendar_set_highlighted_dates`
    /// stores the pointer).
    #[cfg(feature = "widget-calendar")]
    Dates(#[allow(dead_code)] Vec<rs_lvgl_sys::lv_calendar_date_t>),
}

/// Native sub-objects created together with a widget.
#[derive(Clone, Copy, Default)]
pub(crate) struct Parts {
    #[cfg(feature = "widget-ime-pinyin")]
    pub(crate) input: Option<NonNull<lv_obj_t>>,
    #[cfg(feature = "widget-ime-pinyin")]
    pub(crate) ime: Option<NonNull<lv_obj_t>>,
    /// A label created by the wrapper and owned by the widget (the text of a
    /// button or message box). It is always the first child of the object
    /// that holds the node's children.
    pub(crate) label: Option<NonNull<lv_obj_t>>,
    /// Where children are mounted when it is not the widget itself.
    pub(crate) content: Option<NonNull<lv_obj_t>>,
}

impl Parts {
    /// The object that holds the node's children.
    pub(crate) fn children_parent(&self, obj: NonNull<lv_obj_t>) -> NonNull<lv_obj_t> {
        self.content.unwrap_or(obj)
    }

    /// Index of the first application child in [`Parts::children_parent`].
    pub(crate) fn first_child_index(&self) -> i32 {
        i32::from(self.label.is_some())
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
            Props::Msgbox {
                title: old_title,
                buttons: old_buttons,
                ..
            },
            WidgetKind::Msgbox { title, buttons, .. },
        ) => old_title.is_some() != title.is_some() || old_buttons.len() != buttons.len(),
        _ => false,
    }
}

/// An inner object whose events count as the widget's own.
///
/// A calendar is a container around a button matrix whose events bubble to
/// the calendar; the handlers of the calendar node must observe them.
pub(crate) fn event_inner_target(tag: KindTag, obj: NonNull<lv_obj_t>) -> *mut lv_obj_t {
    match tag {
        // SAFETY: `obj` is a live calendar; the button matrix is its child.
        #[cfg(feature = "widget-calendar")]
        KindTag::Calendar => unsafe { rs_lvgl_sys::lv_calendar_get_btnmatrix(obj.as_ptr()) },
        _ => core::ptr::null_mut(),
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
        WidgetKind::Obj => Props::Obj,
        WidgetKind::Row => Props::Row,
        WidgetKind::Column => Props::Column,
        WidgetKind::Grid { columns, rows } => Props::Grid { columns, rows },
        #[cfg(feature = "widget-tabview")]
        WidgetKind::Tabview { tabs, active } => Props::Tabview { tabs, active },
        #[cfg(feature = "widget-keyboard")]
        WidgetKind::Keyboard { target } => Props::Keyboard { target },
        WidgetKind::Label { text, long_mode } => Props::Label { text, long_mode },
        WidgetKind::Button { text } => Props::Button { text },
        #[cfg(feature = "widget-slider")]
        WidgetKind::Slider { min, max, value } => Props::Slider { min, max, value },
        #[cfg(feature = "widget-bar")]
        WidgetKind::Bar { min, max, value } => Props::Bar { min, max, value },
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
        WidgetKind::Msgbox {
            title,
            text,
            buttons,
        } => Props::Msgbox {
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
        WidgetKind::AnimImage {
            frames,
            duration_ms,
            repeat,
        } => Props::AnimImage {
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

/// Create the native object for a typed kind.
///
/// Custom kinds are created by the caller because their factory owns the
/// object.
pub(crate) fn create(
    tag: KindTag,
    parent: NonNull<lv_obj_t>,
) -> Result<(NonNull<lv_obj_t>, Parts), UiError> {
    // SAFETY: `parent` is a live LVGL object supplied by the mounted tree. Each
    // create function returns either a new child object or null.
    unsafe {
        let raw = match tag {
            KindTag::Obj | KindTag::Row | KindTag::Column | KindTag::Grid => {
                rs_lvgl_sys::lv_obj_create(parent.as_ptr())
            }
            #[cfg(feature = "widget-tabview")]
            KindTag::Tabview => rs_lvgl_sys::lv_tabview_create(parent.as_ptr()),
            #[cfg(feature = "widget-keyboard")]
            KindTag::Keyboard => rs_lvgl_sys::lv_keyboard_create(parent.as_ptr()),
            #[cfg(feature = "widget-canvas")]
            KindTag::Canvas => rs_lvgl_sys::lv_canvas_create(parent.as_ptr()),
            #[cfg(feature = "widget-qrcode")]
            KindTag::QrCode => rs_lvgl_sys::lv_qrcode_create(parent.as_ptr()),
            #[cfg(feature = "widget-barcode")]
            KindTag::Barcode => rs_lvgl_sys::lv_barcode_create(parent.as_ptr()),
            #[cfg(feature = "widget-gif")]
            KindTag::Gif => rs_lvgl_sys::lv_gif_create(parent.as_ptr()),
            #[cfg(feature = "widget-lottie")]
            KindTag::Lottie => rs_lvgl_sys::lv_lottie_create(parent.as_ptr()),
            #[cfg(feature = "widget-ime-pinyin")]
            KindTag::PinyinKeyboard => return crate::extensions::pinyin::create(parent),
            KindTag::Label => rs_lvgl_sys::lv_label_create(parent.as_ptr()),
            KindTag::Button => rs_lvgl_sys::lv_button_create(parent.as_ptr()),
            #[cfg(feature = "widget-slider")]
            KindTag::Slider => rs_lvgl_sys::lv_slider_create(parent.as_ptr()),
            #[cfg(feature = "widget-bar")]
            KindTag::Bar => rs_lvgl_sys::lv_bar_create(parent.as_ptr()),
            #[cfg(feature = "widget-switch")]
            KindTag::Switch => rs_lvgl_sys::lv_switch_create(parent.as_ptr()),
            #[cfg(feature = "widget-image")]
            KindTag::Image => rs_lvgl_sys::lv_image_create(parent.as_ptr()),
            #[cfg(feature = "widget-checkbox")]
            KindTag::Checkbox => rs_lvgl_sys::lv_checkbox_create(parent.as_ptr()),
            #[cfg(feature = "widget-textarea")]
            KindTag::Textarea => rs_lvgl_sys::lv_textarea_create(parent.as_ptr()),
            #[cfg(feature = "widget-dropdown")]
            KindTag::Dropdown => rs_lvgl_sys::lv_dropdown_create(parent.as_ptr()),
            #[cfg(feature = "widget-arc")]
            KindTag::Arc => {
                let arc = rs_lvgl_sys::lv_arc_create(parent.as_ptr());
                if !arc.is_null() {
                    // Gauge defaults: hide the background arc and the knob,
                    // keep a visible indicator that the style can recolor.
                    rs_lvgl_sys::lv_obj_set_style_arc_opa(
                        arc,
                        0,
                        rs_lvgl_sys::lv_part_t_LV_PART_MAIN,
                    );
                    rs_lvgl_sys::lv_obj_set_style_bg_opa(
                        arc,
                        0,
                        rs_lvgl_sys::lv_part_t_LV_PART_KNOB,
                    );
                    rs_lvgl_sys::lv_obj_set_style_arc_width(
                        arc,
                        10,
                        rs_lvgl_sys::lv_part_t_LV_PART_INDICATOR,
                    );
                    rs_lvgl_sys::lv_obj_set_clickable(arc, false);
                }
                arc
            }
            #[cfg(feature = "widget-scale")]
            KindTag::Scale => rs_lvgl_sys::lv_scale_create(parent.as_ptr()),
            #[cfg(feature = "widget-chart")]
            KindTag::Chart => rs_lvgl_sys::lv_chart_create(parent.as_ptr()),
            #[cfg(feature = "widget-roller")]
            KindTag::Roller => rs_lvgl_sys::lv_roller_create(parent.as_ptr()),
            #[cfg(feature = "widget-spinbox")]
            KindTag::Spinbox => rs_lvgl_sys::lv_spinbox_create(parent.as_ptr()),
            #[cfg(feature = "widget-spinner")]
            KindTag::Spinner => rs_lvgl_sys::lv_spinner_create(parent.as_ptr()),
            #[cfg(feature = "widget-led")]
            KindTag::Led => rs_lvgl_sys::lv_led_create(parent.as_ptr()),
            #[cfg(feature = "widget-line")]
            KindTag::Line => rs_lvgl_sys::lv_line_create(parent.as_ptr()),
            #[cfg(feature = "widget-buttonmatrix")]
            KindTag::ButtonMatrix => rs_lvgl_sys::lv_buttonmatrix_create(parent.as_ptr()),
            #[cfg(feature = "widget-table")]
            KindTag::Table => rs_lvgl_sys::lv_table_create(parent.as_ptr()),
            #[cfg(feature = "widget-tileview")]
            KindTag::Tileview => rs_lvgl_sys::lv_tileview_create(parent.as_ptr()),
            // A non-null parent keeps the message box inline: a null parent
            // would create a modal backdrop on the top layer that the
            // mounted tree does not own.
            #[cfg(feature = "widget-msgbox")]
            KindTag::Msgbox => rs_lvgl_sys::lv_msgbox_create(parent.as_ptr()),
            #[cfg(feature = "widget-calendar")]
            KindTag::Calendar => rs_lvgl_sys::lv_calendar_create(parent.as_ptr()),
            #[cfg(feature = "widget-span")]
            KindTag::Spans => rs_lvgl_sys::lv_spangroup_create(parent.as_ptr()),
            #[cfg(feature = "widget-imagebutton")]
            KindTag::ImageButton => rs_lvgl_sys::lv_imagebutton_create(parent.as_ptr()),
            #[cfg(feature = "widget-animimage")]
            KindTag::AnimImage => rs_lvgl_sys::lv_animimg_create(parent.as_ptr()),
            #[cfg(feature = "widget-arclabel")]
            KindTag::ArcLabel => rs_lvgl_sys::lv_arclabel_create(parent.as_ptr()),
            KindTag::Custom => unreachable!("custom widgets are created by their factory"),
        };
        let Some(obj) = NonNull::new(raw) else {
            return Err(UiError::Allocation);
        };

        if matches!(
            tag,
            KindTag::Obj | KindTag::Row | KindTag::Column | KindTag::Grid
        ) {
            remove_main_theme(obj);
        }

        if matches!(tag, KindTag::Row | KindTag::Column) {
            rs_lvgl_sys::lv_obj_set_layout(obj.as_ptr(), rs_lvgl_sys::lv_layout_t_LV_LAYOUT_FLEX);
            rs_lvgl_sys::lv_obj_set_flex_flow(
                obj.as_ptr(),
                if tag == KindTag::Row {
                    rs_lvgl_sys::lv_flex_flow_t_LV_FLEX_FLOW_ROW
                } else {
                    rs_lvgl_sys::lv_flex_flow_t_LV_FLEX_FLOW_COLUMN
                },
            );
            rs_lvgl_sys::lv_obj_set_flex_align(
                obj.as_ptr(),
                rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_START,
                rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_START,
                rs_lvgl_sys::lv_flex_align_t_LV_FLEX_ALIGN_START,
            );
        }

        let mut parts = Parts::default();
        match tag {
            KindTag::Button => {
                let label = rs_lvgl_sys::lv_label_create(obj.as_ptr());
                let Some(label) = NonNull::new(label) else {
                    rs_lvgl_sys::lv_obj_delete(obj.as_ptr());
                    return Err(UiError::Allocation);
                };
                rs_lvgl_sys::lv_obj_center(label.as_ptr());
                parts.label = Some(label);
            }
            #[cfg(feature = "widget-msgbox")]
            KindTag::Msgbox => {
                // The text label is the first child of the content area;
                // the node's children follow it.
                let label = rs_lvgl_sys::lv_msgbox_add_text(obj.as_ptr(), c"".as_ptr().cast());
                let content = rs_lvgl_sys::lv_msgbox_get_content(obj.as_ptr());
                let (Some(label), Some(content)) = (NonNull::new(label), NonNull::new(content))
                else {
                    rs_lvgl_sys::lv_obj_delete(obj.as_ptr());
                    return Err(UiError::Allocation);
                };
                parts.label = Some(label);
                parts.content = Some(content);
            }
            _ => {}
        }

        Ok((obj, parts))
    }
}

/// Drop the LVGL theme styles from a plain container's main part.
///
/// Plain containers are layout primitives. The default theme gives every
/// `lv_obj` a card style (background, border, padding and a text color);
/// that text color would stop text properties set on an ancestor from
/// reaching descendants. Scrollbar part styles are kept.
pub(crate) fn remove_main_theme(obj: NonNull<lv_obj_t>) {
    // SAFETY: `obj` is a live object; removing theme styles only detaches
    // shared style references from this object.
    unsafe {
        rs_lvgl_sys::lv_obj_remove_theme(
            obj.as_ptr(),
            rs_lvgl_sys::lv_part_t_LV_PART_MAIN | rs_lvgl_sys::lv_state_t_LV_STATE_ANY,
        );
    }
}

/// Apply kind properties, writing only values that changed.
///
/// Interactive values (slider, bar, arc, switch, checkbox, dropdown and
/// textarea text) are controlled: they are compared with the native widget
/// rather than with the previous descriptor, so a value the user changed in
/// LVGL is reset to the application's value on the next render.
pub(crate) fn apply_props<M: Clone + 'static>(
    obj: NonNull<lv_obj_t>,
    parts: Parts,
    new: &Props,
    old: Option<&Props>,
    slot: Option<&HandlerSlot<M>>,
    retained: &mut Retained,
) -> Result<(), UiError> {
    match (new, old) {
        #[cfg(feature = "widget-ime-pinyin")]
        (Props::PinyinKeyboard { font, .. }, _) => {
            // SAFETY: the hidden IME and its candidate panel are live inside
            // this private container; LVGL propagates its font to candidates.
            unsafe {
                rs_lvgl_sys::lv_obj_set_style_text_font(
                    parts.ime.expect("IME part").as_ptr(),
                    font.0,
                    0,
                );
            }
        }
        #[cfg(feature = "widget-qrcode")]
        (Props::QrCode { code }, old) => {
            if !matches!(old, Some(Props::QrCode { code: previous }) if previous == code) {
                code.apply(obj)?;
            }
        }
        #[cfg(feature = "widget-barcode")]
        (Props::Barcode { code }, old) => {
            if !matches!(old, Some(Props::Barcode { code: previous }) if previous == code) {
                code.apply(obj)?;
            }
        }
        #[cfg(feature = "widget-gif")]
        (Props::Gif { animation }, old) => {
            let old = match old {
                Some(Props::Gif { animation }) => Some(animation),
                _ => None,
            };
            if old != Some(animation) {
                animation.apply(obj, old, retained)?;
            }
        }
        #[cfg(feature = "widget-lottie")]
        (Props::Lottie { animation }, old) => {
            if !matches!(old, Some(Props::Lottie { animation: previous }) if previous == animation)
            {
                animation.apply(obj, retained)?;
            }
        }
        #[cfg(feature = "widget-canvas")]
        (Props::Canvas { frame }, old) => {
            if matches!(old, Some(Props::Canvas { frame: previous }) if previous == frame) {
                return Ok(());
            }
            let resized = !matches!(old, Some(Props::Canvas { frame: previous }) if previous.width == frame.width && previous.height == frame.height);
            if resized {
                let buffer = crate::graphics::DrawBuffer::new(frame.width, frame.height)?;
                // SAFETY: the new stable buffer is installed before releasing
                // the old buffer retained by this live canvas.
                unsafe {
                    rs_lvgl_sys::lv_canvas_set_draw_buf(obj.as_ptr(), buffer.0.as_ptr());
                }
                *retained = Retained::Canvas(buffer);
            }
            crate::canvas::draw(obj, frame);
        }
        (Props::Obj | Props::Row | Props::Column | Props::Grid { .. } | Props::Custom, _) => {}
        #[cfg(feature = "widget-tabview")]
        (Props::Tabview { .. }, _) => {}
        #[cfg(feature = "widget-tileview")]
        (Props::Tileview { .. }, _) => {}
        #[cfg(feature = "widget-keyboard")]
        (Props::Keyboard { .. }, _) => {}
        (Props::Label { text, long_mode }, old) => {
            let (old_text, old_mode) = match old {
                Some(Props::Label { text, long_mode }) => (Some(text.as_str()), Some(*long_mode)),
                _ => (None, None),
            };
            if old_text != Some(text.as_str()) {
                set_label_text(obj, text);
            }
            if old_mode != Some(*long_mode) {
                // SAFETY: `obj` is a live label object.
                unsafe { rs_lvgl_sys::lv_label_set_long_mode(obj.as_ptr(), long_mode.to_raw()) };
            }
        }
        (Props::Button { text }, old) => {
            let changed = match old {
                Some(Props::Button { text: old }) => old != text,
                _ => true,
            };
            if changed {
                if let Some(label) = parts.label {
                    set_label_text(label, text);
                }
            }
        }
        #[cfg(feature = "widget-slider")]
        (Props::Slider { min, max, value }, old) => {
            let range_changed = match old {
                Some(Props::Slider {
                    min: old_min,
                    max: old_max,
                    ..
                }) => old_min != min || old_max != max,
                _ => true,
            };
            // SAFETY: `obj` is a live slider object.
            unsafe {
                if range_changed {
                    rs_lvgl_sys::lv_slider_set_range(obj.as_ptr(), *min, *max);
                }
                if rs_lvgl_sys::lv_slider_get_value(obj.as_ptr()) != *value {
                    suppress(slot, || {
                        rs_lvgl_sys::lv_slider_set_value(obj.as_ptr(), *value, false);
                    });
                }
            }
        }
        #[cfg(feature = "widget-bar")]
        (Props::Bar { min, max, value }, old) => {
            let range_changed = match old {
                Some(Props::Bar {
                    min: old_min,
                    max: old_max,
                    ..
                }) => old_min != min || old_max != max,
                _ => true,
            };
            // SAFETY: `obj` is a live bar object.
            unsafe {
                if range_changed {
                    rs_lvgl_sys::lv_bar_set_range(obj.as_ptr(), *min, *max);
                }
                if rs_lvgl_sys::lv_bar_get_value(obj.as_ptr()) != *value {
                    suppress(slot, || {
                        rs_lvgl_sys::lv_bar_set_value(obj.as_ptr(), *value, false);
                    });
                }
            }
        }
        #[cfg(feature = "widget-switch")]
        (Props::Switch { checked }, _) => {
            if is_checked(obj) != *checked {
                // SAFETY: `obj` is a live switch object and the state bit is a
                // valid LVGL object state.
                unsafe {
                    suppress(slot, || {
                        if *checked {
                            rs_lvgl_sys::lv_obj_add_state(
                                obj.as_ptr(),
                                rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED,
                            );
                        } else {
                            rs_lvgl_sys::lv_obj_remove_state(
                                obj.as_ptr(),
                                rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED,
                            );
                        }
                    });
                }
            }
        }
        #[cfg(feature = "widget-image")]
        (Props::Image { source, scale }, old) => {
            let (old_source, old_scale) = match old {
                Some(Props::Image { source, scale }) => (Some(*source), *scale),
                _ => (None, 0),
            };
            let source_changed = old_source != Some(*source);
            // SAFETY: `obj` is a live image object. `retained` keeps a
            // generated RGB565 descriptor alive for as long as the mounted
            // node exists; raw descriptors are the caller's responsibility.
            unsafe {
                let rebuild = source_changed
                    || (!matches!(retained, Retained::Image(_))
                        && matches!(source, ImageSource::Rgb565 { .. }));
                if rebuild {
                    let new_descriptor = source.descriptor();
                    let pointer = source.ptr(new_descriptor.as_deref());
                    rs_lvgl_sys::lv_image_set_src(obj.as_ptr(), pointer);
                    *retained = match new_descriptor {
                        Some(descriptor) => Retained::Image(descriptor),
                        None => Retained::None,
                    };
                }
                if old_scale != *scale {
                    rs_lvgl_sys::lv_image_set_scale(obj.as_ptr(), *scale as u32);
                }
            }
        }
        #[cfg(feature = "widget-checkbox")]
        (Props::Checkbox { text, checked }, old) => {
            let text_changed = match old {
                Some(Props::Checkbox { text: old_text, .. }) => old_text != text,
                _ => true,
            };
            let checked_changed = is_checked(obj) != *checked;
            // SAFETY: `obj` is a live checkbox object.
            unsafe {
                if text_changed {
                    if let Ok(text) = CString::new(text.as_str()) {
                        rs_lvgl_sys::lv_checkbox_set_text(obj.as_ptr(), text.as_ptr());
                    }
                }
                if checked_changed {
                    suppress(slot, || {
                        if *checked {
                            rs_lvgl_sys::lv_obj_add_state(
                                obj.as_ptr(),
                                rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED,
                            );
                        } else {
                            rs_lvgl_sys::lv_obj_remove_state(
                                obj.as_ptr(),
                                rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED,
                            );
                        }
                    });
                }
            }
        }
        #[cfg(feature = "widget-textarea")]
        (
            Props::Textarea {
                text,
                placeholder,
                one_line,
                password,
            },
            old,
        ) => {
            let (old_placeholder, old_one_line, old_password) = match old {
                Some(Props::Textarea {
                    placeholder,
                    one_line,
                    password,
                    ..
                }) => (Some(placeholder.as_str()), Some(*one_line), Some(*password)),
                _ => (None, None, None),
            };
            // SAFETY: `obj` is a live textarea object.
            unsafe {
                if old_one_line != Some(*one_line) {
                    rs_lvgl_sys::lv_textarea_set_one_line(obj.as_ptr(), *one_line);
                }
                if old_password != Some(*password) {
                    rs_lvgl_sys::lv_textarea_set_password_mode(obj.as_ptr(), *password);
                }
                if old_placeholder != Some(placeholder.as_str()) {
                    if let Ok(text) = CString::new(placeholder.as_str()) {
                        rs_lvgl_sys::lv_textarea_set_placeholder_text(obj.as_ptr(), text.as_ptr());
                    }
                }
                if !textarea_text_is(obj, text) {
                    if let Ok(text) = CString::new(text.as_str()) {
                        // `lv_textarea_set_text` emits VALUE_CHANGED; a view
                        // update must not feed back into the queue.
                        suppress(slot, || {
                            rs_lvgl_sys::lv_textarea_set_text(obj.as_ptr(), text.as_ptr());
                        });
                    }
                }
            }
        }
        #[cfg(feature = "widget-dropdown")]
        (Props::Dropdown { options, selected }, old) => {
            let options_changed = match old {
                Some(Props::Dropdown {
                    options: old_options,
                    ..
                }) => old_options != options,
                _ => true,
            };
            // SAFETY: `obj` is a live dropdown object.
            unsafe {
                if options_changed {
                    let joined = options.join("\n");
                    if let Ok(options) = CString::new(joined) {
                        suppress(slot, || {
                            rs_lvgl_sys::lv_dropdown_set_options(obj.as_ptr(), options.as_ptr());
                            rs_lvgl_sys::lv_dropdown_set_selected(obj.as_ptr(), *selected);
                        });
                    }
                } else if rs_lvgl_sys::lv_dropdown_get_selected(obj.as_ptr()) != *selected {
                    suppress(slot, || {
                        rs_lvgl_sys::lv_dropdown_set_selected(obj.as_ptr(), *selected);
                    });
                }
            }
        }
        #[cfg(feature = "widget-arc")]
        (Props::Arc { min, max, value }, old) => {
            let range_changed = match old {
                Some(Props::Arc {
                    min: old_min,
                    max: old_max,
                    ..
                }) => old_min != min || old_max != max,
                _ => true,
            };
            // SAFETY: `obj` is a live arc object.
            unsafe {
                if range_changed {
                    rs_lvgl_sys::lv_arc_set_range(obj.as_ptr(), *min, *max);
                }
                if rs_lvgl_sys::lv_arc_get_value(obj.as_ptr()) != *value {
                    suppress(slot, || {
                        rs_lvgl_sys::lv_arc_set_value(obj.as_ptr(), *value);
                    });
                }
            }
        }
        #[cfg(feature = "widget-scale")]
        (
            Props::Scale {
                min,
                max,
                total_ticks,
                major_tick_every,
            },
            old,
        ) => {
            let (range_changed, ticks_changed, major_changed) = match old {
                Some(Props::Scale {
                    min: old_min,
                    max: old_max,
                    total_ticks: old_ticks,
                    major_tick_every: old_major,
                }) => (
                    old_min != min || old_max != max,
                    old_ticks != total_ticks,
                    old_major != major_tick_every,
                ),
                _ => (true, true, true),
            };
            // SAFETY: `obj` is a live scale object.
            unsafe {
                if range_changed {
                    rs_lvgl_sys::lv_scale_set_range(obj.as_ptr(), *min, *max);
                }
                if ticks_changed {
                    rs_lvgl_sys::lv_scale_set_total_tick_count(obj.as_ptr(), *total_ticks);
                }
                if major_changed {
                    rs_lvgl_sys::lv_scale_set_major_tick_every(obj.as_ptr(), *major_tick_every);
                }
            }
        }
        #[cfg(feature = "widget-chart")]
        (
            Props::Chart {
                chart_type,
                min,
                max,
                series,
            },
            old,
        ) => {
            let (old_points, old_range, old_type, old_series) = match old {
                Some(Props::Chart {
                    chart_type,
                    min,
                    max,
                    series,
                }) => (
                    series.first().map(|series| series.values.len()),
                    Some((*min, *max)),
                    Some(*chart_type),
                    Some(series.as_slice()),
                ),
                _ => (None, None, None, None),
            };
            let points = series
                .first()
                .map(|series| series.values.len())
                .unwrap_or(0);
            let points_changed = old_points != Some(points);
            let range_changed = old_range != Some((*min, *max));
            let type_changed = old_type != Some(*chart_type);
            let series_changed = old_series != Some(series.as_slice());
            if !(points_changed || range_changed || type_changed || series_changed) {
                return Ok(());
            }
            // SAFETY: `obj` is a live chart object. Series pointers returned by
            // LVGL stay valid until that series is removed.
            unsafe {
                if points_changed {
                    rs_lvgl_sys::lv_chart_set_point_count(obj.as_ptr(), points as u32);
                }
                if range_changed {
                    rs_lvgl_sys::lv_chart_set_axis_range(
                        obj.as_ptr(),
                        rs_lvgl_sys::lv_chart_axis_t_LV_CHART_AXIS_PRIMARY_Y,
                        *min,
                        *max,
                    );
                }
                if type_changed {
                    rs_lvgl_sys::lv_chart_set_type(obj.as_ptr(), chart_type.to_raw());
                }

                // A changed point count reallocates every series buffer, so
                // the values are rewritten for all series in that case.
                let mut current =
                    rs_lvgl_sys::lv_chart_get_series_next(obj.as_ptr(), core::ptr::null());
                for (index, series) in series.iter().enumerate() {
                    let previous = old_series.and_then(|old| old.get(index));
                    let pointer = if current.is_null() {
                        let added = rs_lvgl_sys::lv_chart_add_series(
                            obj.as_ptr(),
                            series.color.to_raw(),
                            rs_lvgl_sys::lv_chart_axis_t_LV_CHART_AXIS_PRIMARY_Y,
                        );
                        if added.is_null() {
                            return Err(UiError::Allocation);
                        }
                        added
                    } else {
                        let pointer = current;
                        current = rs_lvgl_sys::lv_chart_get_series_next(
                            obj.as_ptr(),
                            pointer as *const _,
                        );
                        pointer
                    };
                    // Native series mirror the previous descriptor one to one,
                    // so an index present there names an existing series.
                    let existing = previous.is_some();
                    if !existing || previous.map(|old| old.color) != Some(series.color) {
                        rs_lvgl_sys::lv_chart_set_series_color(
                            obj.as_ptr(),
                            pointer,
                            series.color.to_raw(),
                        );
                    }
                    if !existing
                        || points_changed
                        || previous.map(|old| old.values.as_slice())
                            != Some(series.values.as_slice())
                    {
                        rs_lvgl_sys::lv_chart_set_series_values(
                            obj.as_ptr(),
                            pointer,
                            series.values.as_ptr(),
                            series.values.len(),
                        );
                    }
                }
                while !current.is_null() {
                    let next =
                        rs_lvgl_sys::lv_chart_get_series_next(obj.as_ptr(), current as *const _);
                    rs_lvgl_sys::lv_chart_remove_series(obj.as_ptr(), current);
                    current = next;
                }
                rs_lvgl_sys::lv_chart_refresh(obj.as_ptr());
            }
        }
        #[cfg(feature = "widget-roller")]
        (
            Props::Roller {
                options,
                selected,
                visible_rows,
                mode,
            },
            old,
        ) => {
            let (options_changed, rows_changed) = match old {
                Some(Props::Roller {
                    options: old_options,
                    visible_rows: old_rows,
                    mode: old_mode,
                    ..
                }) => (
                    old_options != options || old_mode != mode,
                    old_rows != visible_rows,
                ),
                _ => (true, true),
            };
            if options_changed {
                let joined =
                    CString::new(options.join("\n")).map_err(|_| UiError::InvalidProperty)?;
                // SAFETY: `obj` is a live roller; LVGL copies the options.
                suppress(slot, || unsafe {
                    rs_lvgl_sys::lv_roller_set_options(
                        obj.as_ptr(),
                        joined.as_ptr(),
                        mode.to_raw(),
                    );
                });
            }
            // SAFETY: `obj` is a live roller.
            unsafe {
                if rows_changed {
                    rs_lvgl_sys::lv_roller_set_visible_row_count(obj.as_ptr(), *visible_rows);
                }
                if rs_lvgl_sys::lv_roller_get_selected(obj.as_ptr()) != *selected {
                    suppress(slot, || {
                        rs_lvgl_sys::lv_roller_set_selected(obj.as_ptr(), *selected, false);
                    });
                }
            }
        }
        #[cfg(feature = "widget-spinbox")]
        (
            Props::Spinbox {
                min,
                max,
                value,
                digit_count,
                decimal_pos,
                step,
            },
            old,
        ) => {
            let (format_changed, range_changed, step_changed) = match old {
                Some(Props::Spinbox {
                    min: old_min,
                    max: old_max,
                    digit_count: old_digits,
                    decimal_pos: old_pos,
                    step: old_step,
                    ..
                }) => (
                    old_digits != digit_count || old_pos != decimal_pos,
                    old_min != min || old_max != max,
                    old_step != step,
                ),
                _ => (true, true, true),
            };
            // SAFETY: `obj` is a live spinbox. Each write re-renders the text
            // through the textarea base class, which emits a value event; the
            // write is suppressed so only user edits reach the handlers.
            unsafe {
                if format_changed {
                    suppress(slot, || {
                        rs_lvgl_sys::lv_spinbox_set_digit_format(
                            obj.as_ptr(),
                            *digit_count,
                            *decimal_pos,
                        );
                    });
                }
                if range_changed || format_changed {
                    // The digit format may have clamped the range.
                    suppress(slot, || {
                        rs_lvgl_sys::lv_spinbox_set_range(obj.as_ptr(), *min, *max);
                    });
                }
                if step_changed {
                    // Changing the step also moves the cursor and re-renders.
                    suppress(slot, || {
                        rs_lvgl_sys::lv_spinbox_set_step(obj.as_ptr(), *step);
                    });
                }
                if rs_lvgl_sys::lv_spinbox_get_value(obj.as_ptr()) != *value {
                    suppress(slot, || {
                        rs_lvgl_sys::lv_spinbox_set_value(obj.as_ptr(), *value);
                    });
                }
            }
        }
        #[cfg(feature = "widget-spinner")]
        (
            Props::Spinner {
                duration_ms,
                arc_angle,
            },
            old,
        ) => {
            let changed = match old {
                Some(Props::Spinner {
                    duration_ms: old_duration,
                    arc_angle: old_angle,
                }) => old_duration != duration_ms || old_angle != arc_angle,
                _ => true,
            };
            if changed {
                // SAFETY: `obj` is a live spinner; LVGL restarts its own
                // animation, which is deleted with the object.
                unsafe {
                    rs_lvgl_sys::lv_spinner_set_anim_params(obj.as_ptr(), *duration_ms, *arc_angle)
                };
            }
        }
        #[cfg(feature = "widget-led")]
        (Props::Led { color, brightness }, old) => {
            let (old_color, old_brightness) = match old {
                Some(Props::Led { color, brightness }) => (Some(*color), Some(*brightness)),
                _ => (None, None),
            };
            // SAFETY: `obj` is a live LED.
            unsafe {
                if old_color != Some(*color) {
                    rs_lvgl_sys::lv_led_set_color(obj.as_ptr(), color.to_raw());
                }
                if old_brightness != Some(*brightness) {
                    rs_lvgl_sys::lv_led_set_brightness(obj.as_ptr(), *brightness);
                }
            }
        }
        #[cfg(feature = "widget-line")]
        (Props::Line { points }, old) => {
            let changed = match old {
                Some(Props::Line { points: old_points }) => old_points != points,
                _ => true,
            } || !matches!(retained, Retained::Points(_));
            if changed {
                let buffer: Vec<rs_lvgl_sys::lv_point_precise_t> = points
                    .iter()
                    .map(|&(x, y)| rs_lvgl_sys::lv_point_precise_t {
                        x: x as _,
                        y: y as _,
                    })
                    .collect();
                // SAFETY: `obj` is a live line. LVGL stores the pointer; the
                // buffer is moved into `retained` (its heap address does not
                // change) and outlives every use, because the old buffer is
                // only dropped after LVGL points at the new one.
                unsafe {
                    rs_lvgl_sys::lv_line_set_points(
                        obj.as_ptr(),
                        buffer.as_ptr(),
                        buffer.len() as u32,
                    )
                };
                *retained = Retained::Points(buffer);
            }
        }
        #[cfg(feature = "widget-buttonmatrix")]
        (Props::ButtonMatrix { rows }, old) => {
            let changed = match old {
                Some(Props::ButtonMatrix { rows: old_rows }) => old_rows != rows,
                _ => true,
            } || !matches!(retained, Retained::Map { .. });
            if changed {
                const ROW_BREAK: &[u8] = b"\n\0";
                const END: &[u8] = b"\0";
                let mut texts = Vec::new();
                for text in rows.iter().flatten() {
                    texts.push(CString::new(text.as_str()).map_err(|_| UiError::InvalidProperty)?);
                }
                let mut pointers = Vec::with_capacity(texts.len() + rows.len());
                let mut next = texts.iter();
                for (index, row) in rows.iter().enumerate() {
                    if index > 0 {
                        pointers.push(ROW_BREAK.as_ptr().cast());
                    }
                    for _ in row {
                        if let Some(text) = next.next() {
                            pointers.push(text.as_ptr());
                        }
                    }
                }
                pointers.push(END.as_ptr().cast());
                // SAFETY: `obj` is a live button matrix. LVGL keeps the map
                // pointer and the strings; both heap buffers move into
                // `retained` without changing address, and the old map is
                // dropped only after LVGL switched to the new one.
                unsafe { rs_lvgl_sys::lv_buttonmatrix_set_map(obj.as_ptr(), pointers.as_ptr()) };
                *retained = Retained::Map {
                    _texts: texts,
                    _pointers: pointers,
                };
            }
        }
        #[cfg(feature = "widget-table")]
        (
            Props::Table {
                cells,
                column_widths,
            },
            old,
        ) => {
            let rows = cells.len() as u32;
            let columns = cells.first().map(Vec::len).unwrap_or(0) as u32;
            let (old_cells, old_widths) = match old {
                Some(Props::Table {
                    cells,
                    column_widths,
                }) => (Some(cells), Some(column_widths)),
                _ => (None, None),
            };
            let same_shape = old_cells
                .map(|old| {
                    old.len() == cells.len()
                        && old.first().map(Vec::len) == cells.first().map(Vec::len)
                })
                .unwrap_or(false);
            // SAFETY: `obj` is a live table; LVGL copies every cell text.
            unsafe {
                if !same_shape {
                    rs_lvgl_sys::lv_table_set_column_count(obj.as_ptr(), columns);
                    rs_lvgl_sys::lv_table_set_row_count(obj.as_ptr(), rows);
                }
                for (row, values) in cells.iter().enumerate() {
                    for (column, text) in values.iter().enumerate() {
                        let unchanged = same_shape
                            && old_cells
                                .map(|old| old[row][column] == *text)
                                .unwrap_or(false);
                        if unchanged {
                            continue;
                        }
                        let text =
                            CString::new(text.as_str()).map_err(|_| UiError::InvalidProperty)?;
                        rs_lvgl_sys::lv_table_set_cell_value(
                            obj.as_ptr(),
                            row as u32,
                            column as u32,
                            text.as_ptr(),
                        );
                    }
                }
                if !same_shape || old_widths != Some(column_widths) {
                    for (column, width) in column_widths.iter().enumerate() {
                        rs_lvgl_sys::lv_table_set_column_width(obj.as_ptr(), column as u32, *width);
                    }
                }
            }
        }
        #[cfg(feature = "widget-msgbox")]
        (
            Props::Msgbox {
                title,
                text,
                buttons,
            },
            old,
        ) => {
            let (old_title, old_text, old_buttons) = match old {
                Some(Props::Msgbox {
                    title,
                    text,
                    buttons,
                }) => (
                    Some(title.as_deref()),
                    Some(text.as_str()),
                    Some(buttons.as_slice()),
                ),
                _ => (None, None, None),
            };
            if old_text != Some(text.as_str()) {
                if let Some(label) = parts.label {
                    set_label_text(label, text);
                }
            }
            if let Some(title) = title {
                if old_title.is_none() {
                    let title =
                        CString::new(title.as_str()).map_err(|_| UiError::InvalidProperty)?;
                    // SAFETY: `obj` is a live message box; LVGL copies the text.
                    let label =
                        unsafe { rs_lvgl_sys::lv_msgbox_add_title(obj.as_ptr(), title.as_ptr()) };
                    if label.is_null() {
                        return Err(UiError::Allocation);
                    }
                } else if old_title != Some(Some(title.as_str())) {
                    // SAFETY: `obj` is a live message box that has a title.
                    let label = unsafe { rs_lvgl_sys::lv_msgbox_get_title(obj.as_ptr()) };
                    if let Some(label) = NonNull::new(label) {
                        set_label_text(label, title);
                    }
                }
            }
            match old_buttons {
                None => {
                    for (index, text) in buttons.iter().enumerate() {
                        add_footer_button(obj, index, text)?;
                    }
                }
                Some(old_buttons) => {
                    // `needs_recreate` guarantees the same button count.
                    // SAFETY: `obj` is a live message box.
                    let footer = unsafe { rs_lvgl_sys::lv_msgbox_get_footer(obj.as_ptr()) };
                    for (index, (old, text)) in old_buttons.iter().zip(buttons).enumerate() {
                        if old == text || footer.is_null() {
                            continue;
                        }
                        // SAFETY: the footer holds one button per entry, each
                        // with its label as the first child.
                        let label = unsafe {
                            let button = rs_lvgl_sys::lv_obj_get_child(footer, index as i32);
                            if button.is_null() {
                                continue;
                            }
                            rs_lvgl_sys::lv_obj_get_child(button, 0)
                        };
                        if let Some(label) = NonNull::new(label) {
                            set_label_text(label, text);
                        }
                    }
                }
            }
        }
        #[cfg(feature = "widget-calendar")]
        (
            Props::Calendar {
                year,
                month,
                today,
                highlighted,
            },
            old,
        ) => {
            let (old_shown, old_today, old_highlighted) = match old {
                Some(Props::Calendar {
                    year,
                    month,
                    today,
                    highlighted,
                }) => (
                    Some((*year, *month)),
                    Some(*today),
                    Some(highlighted.as_slice()),
                ),
                _ => (None, None, None),
            };
            // SAFETY: `obj` is a live calendar. Changing the shown month only
            // rewrites the calendar's own button matrix map; it emits no
            // value event to the calendar.
            unsafe {
                if old_shown != Some((*year, *month)) {
                    rs_lvgl_sys::lv_calendar_set_month_shown(
                        obj.as_ptr(),
                        u32::from(*year),
                        u32::from(*month),
                    );
                }
                if old_today != Some(*today) {
                    // A zero date matches no day, which clears LVGL's default
                    // "today" marker.
                    let date = today.unwrap_or(CalendarDate::new(0, 0, 0));
                    rs_lvgl_sys::lv_calendar_set_today_date(
                        obj.as_ptr(),
                        u32::from(date.year),
                        u32::from(date.month),
                        u32::from(date.day),
                    );
                }
            }
            let changed = old_highlighted != Some(highlighted.as_slice())
                || !matches!(retained, Retained::Dates(_));
            if changed {
                let mut dates: Vec<rs_lvgl_sys::lv_calendar_date_t> =
                    highlighted.iter().map(|date| date.to_raw()).collect();
                let pointer = if dates.is_empty() {
                    core::ptr::null_mut()
                } else {
                    dates.as_mut_ptr()
                };
                // SAFETY: `obj` is a live calendar. LVGL stores the pointer;
                // the buffer moves into `retained` without changing address
                // and the previous buffer is dropped only afterwards.
                unsafe {
                    rs_lvgl_sys::lv_calendar_set_highlighted_dates(
                        obj.as_ptr(),
                        pointer,
                        dates.len(),
                    )
                };
                *retained = Retained::Dates(dates);
            }
        }
        #[cfg(feature = "widget-span")]
        (Props::Spans { spans }, old) => {
            let old_spans = match old {
                Some(Props::Spans { spans }) => Some(spans.as_slice()),
                _ => None,
            };
            if old_spans == Some(spans.as_slice()) {
                return Ok(());
            }
            let texts = spans
                .iter()
                .map(|span| CString::new(span.text.as_str()).map_err(|_| UiError::InvalidProperty))
                .collect::<Result<Vec<_>, _>>()?;
            let reuse = old_spans.filter(|old| old.len() == spans.len());
            // SAFETY: `obj` is a live span group. Span handles returned by
            // LVGL stay valid until they are deleted from the group; texts
            // are copied.
            unsafe {
                if reuse.is_none() {
                    loop {
                        let span = rs_lvgl_sys::lv_spangroup_get_child(obj.as_ptr(), 0);
                        if span.is_null() {
                            break;
                        }
                        rs_lvgl_sys::lv_spangroup_delete_span(obj.as_ptr(), span);
                    }
                    for _ in spans {
                        if rs_lvgl_sys::lv_spangroup_add_span(obj.as_ptr()).is_null() {
                            return Err(UiError::Allocation);
                        }
                    }
                }
                for (index, (span, text)) in spans.iter().zip(&texts).enumerate() {
                    let previous = reuse.map(|old| &old[index]);
                    if previous == Some(span) {
                        continue;
                    }
                    let native = rs_lvgl_sys::lv_spangroup_get_child(obj.as_ptr(), index as i32);
                    if native.is_null() {
                        return Err(UiError::Allocation);
                    }
                    if previous.map(|old| old.text.as_str()) != Some(span.text.as_str()) {
                        rs_lvgl_sys::lv_span_set_text(native, text.as_ptr());
                    }
                    let style = rs_lvgl_sys::lv_span_get_style(native);
                    match span.color {
                        Some(color) => rs_lvgl_sys::lv_style_set_text_color(style, color.to_raw()),
                        None => {
                            rs_lvgl_sys::lv_style_remove_prop(
                                style,
                                rs_lvgl_sys::_lv_style_id_t_LV_STYLE_TEXT_COLOR
                                    as rs_lvgl_sys::lv_style_prop_t,
                            );
                        }
                    }
                    match span.font {
                        Some(font) => rs_lvgl_sys::lv_style_set_text_font(style, font.0),
                        None => {
                            rs_lvgl_sys::lv_style_remove_prop(
                                style,
                                rs_lvgl_sys::_lv_style_id_t_LV_STYLE_TEXT_FONT
                                    as rs_lvgl_sys::lv_style_prop_t,
                            );
                        }
                    }
                }
                rs_lvgl_sys::lv_spangroup_refresh(obj.as_ptr());
            }
        }
        #[cfg(feature = "widget-imagebutton")]
        (Props::ImageButton { released, pressed }, old) => {
            let changed = match old {
                Some(Props::ImageButton {
                    released: old_released,
                    pressed: old_pressed,
                }) => old_released != released || old_pressed != pressed,
                _ => true,
            } || (!matches!(retained, Retained::Images(_))
                && core::iter::once(released)
                    .chain(pressed)
                    .any(|source| matches!(source, ImageSource::Rgb565 { .. })));
            if changed {
                let (descriptors, pointers) =
                    image_pointers(core::iter::once(*released).chain(*pressed));
                let pressed_pointer = if pressed.is_some() {
                    pointers[1]
                } else {
                    core::ptr::null()
                };
                // SAFETY: `obj` is a live image button. LVGL stores the
                // pointers; generated descriptors move into `retained`
                // without changing address, after LVGL switched to them.
                unsafe {
                    rs_lvgl_sys::lv_imagebutton_set_src(
                        obj.as_ptr(),
                        rs_lvgl_sys::lv_imagebutton_state_t_LV_IMAGEBUTTON_STATE_RELEASED,
                        core::ptr::null(),
                        pointers[0],
                        core::ptr::null(),
                    );
                    rs_lvgl_sys::lv_imagebutton_set_src(
                        obj.as_ptr(),
                        rs_lvgl_sys::lv_imagebutton_state_t_LV_IMAGEBUTTON_STATE_PRESSED,
                        core::ptr::null(),
                        pressed_pointer,
                        core::ptr::null(),
                    );
                }
                *retained = Retained::Images(descriptors);
            }
        }
        #[cfg(feature = "widget-animimage")]
        (
            Props::AnimImage {
                frames,
                duration_ms,
                repeat,
            },
            old,
        ) => {
            let changed = match old {
                Some(Props::AnimImage {
                    frames: old_frames,
                    duration_ms: old_duration,
                    repeat: old_repeat,
                }) => old_frames != frames || old_duration != duration_ms || old_repeat != repeat,
                _ => true,
            } || !matches!(retained, Retained::Frames { .. });
            if changed {
                let (descriptors, mut pointers) = image_pointers(frames.iter().copied());
                // SAFETY: `obj` is a live animated image. LVGL stores the
                // pointer array and the image stores the current frame; the
                // first frame is shown before the previous buffers are
                // dropped, and restarting replaces the running animation
                // (LVGL deletes it together with the object).
                unsafe {
                    rs_lvgl_sys::lv_animimg_set_src(
                        obj.as_ptr(),
                        pointers.as_mut_ptr(),
                        pointers.len(),
                    );
                    rs_lvgl_sys::lv_image_set_src(obj.as_ptr(), pointers[0]);
                    rs_lvgl_sys::lv_animimg_set_duration(obj.as_ptr(), *duration_ms);
                    rs_lvgl_sys::lv_animimg_set_repeat_count(obj.as_ptr(), *repeat);
                    rs_lvgl_sys::lv_animimg_start(obj.as_ptr());
                }
                *retained = Retained::Frames {
                    _descriptors: descriptors,
                    _pointers: pointers,
                };
            }
        }
        #[cfg(feature = "widget-arclabel")]
        (
            Props::ArcLabel {
                text,
                radius,
                start_angle,
                angle_size,
            },
            old,
        ) => {
            let (old_text, old_radius, old_start, old_size) = match old {
                Some(Props::ArcLabel {
                    text,
                    radius,
                    start_angle,
                    angle_size,
                }) => (
                    Some(text.as_str()),
                    Some(*radius),
                    Some(*start_angle),
                    Some(*angle_size),
                ),
                _ => (None, None, None, None),
            };
            // SAFETY: `obj` is a live arc label; LVGL copies the text.
            unsafe {
                if old_text != Some(text.as_str()) {
                    let text = CString::new(text.as_str()).map_err(|_| UiError::InvalidProperty)?;
                    rs_lvgl_sys::lv_arclabel_set_text(obj.as_ptr(), text.as_ptr());
                }
                if old_radius != Some(*radius) {
                    rs_lvgl_sys::lv_arclabel_set_radius(obj.as_ptr(), *radius);
                }
                if old_start != Some(*start_angle) {
                    rs_lvgl_sys::lv_arclabel_set_angle_start(obj.as_ptr(), *start_angle as _);
                }
                if old_size != Some(*angle_size) {
                    rs_lvgl_sys::lv_arclabel_set_angle_size(obj.as_ptr(), *angle_size as _);
                }
            }
        }
    }
    Ok(())
}

/// Build image pointers for `sources`, generating RGB565 descriptors.
///
/// The descriptors are boxed so their addresses survive moving the vector
/// into [`Retained`] and while the vector grows.
#[allow(clippy::vec_box)]
#[cfg(any(feature = "widget-imagebutton", feature = "widget-animimage"))]
fn image_pointers(
    sources: impl Iterator<Item = ImageSource>,
) -> (
    Vec<Box<rs_lvgl_sys::lv_image_dsc_t>>,
    Vec<*const cty::c_void>,
) {
    let mut descriptors = Vec::new();
    let mut pointers = Vec::new();
    for source in sources {
        match source.descriptor() {
            Some(descriptor) => {
                pointers.push(source.ptr(Some(&descriptor)));
                descriptors.push(descriptor);
            }
            None => pointers.push(source.ptr(None)),
        }
    }
    (descriptors, pointers)
}

/// Add one message box footer button that reports its index.
#[cfg(feature = "widget-msgbox")]
fn add_footer_button(obj: NonNull<lv_obj_t>, index: usize, text: &str) -> Result<(), UiError> {
    let text = CString::new(text).map_err(|_| UiError::InvalidProperty)?;
    // SAFETY: `obj` is a live message box; LVGL copies the text. The
    // callback carries only the index and needs no cleanup.
    unsafe {
        let button = rs_lvgl_sys::lv_msgbox_add_footer_button(obj.as_ptr(), text.as_ptr());
        if button.is_null() {
            return Err(UiError::Allocation);
        }
        let dsc = rs_lvgl_sys::lv_obj_add_event_cb(
            button,
            Some(footer_button_clicked),
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED,
            (index + 1) as *mut cty::c_void,
        );
        if dsc.is_null() {
            return Err(UiError::Allocation);
        }
    }
    Ok(())
}

/// Forward a footer button click to its message box as a value change whose
/// parameter is the button index plus one.
///
/// # Safety
///
/// Called by LVGL with a live event whose target is a footer button
/// created by [`add_footer_button`]. The callback never unwinds.
#[cfg(feature = "widget-msgbox")]
unsafe extern "C" fn footer_button_clicked(event: *mut rs_lvgl_sys::lv_event_t) {
    // SAFETY: footer buttons are children of the footer, which is a child of
    // the message box; all are alive during the event.
    unsafe {
        let button = rs_lvgl_sys::lv_event_get_current_target_obj(event);
        let footer = rs_lvgl_sys::lv_obj_get_parent(button);
        let msgbox = rs_lvgl_sys::lv_obj_get_parent(footer);
        if msgbox.is_null() {
            return;
        }
        rs_lvgl_sys::lv_obj_send_event(
            msgbox,
            rs_lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            rs_lvgl_sys::lv_event_get_user_data(event),
        );
    }
}

#[cfg(feature = "widget-tileview")]
pub(crate) fn tileview_pages(
    obj: NonNull<lv_obj_t>,
    tiles: &[Tile],
    active: u32,
) -> Result<Vec<NonNull<lv_obj_t>>, UiError> {
    let mut pages = Vec::with_capacity(tiles.len());
    for tile in tiles {
        // SAFETY: `obj` is a live tileview.
        let page = unsafe {
            rs_lvgl_sys::lv_tileview_add_tile(
                obj.as_ptr(),
                tile.column,
                tile.row,
                tile.dir.to_raw(),
            )
        };
        let Some(page) = NonNull::new(page) else {
            return Err(UiError::Allocation);
        };
        pages.push(page);
    }
    set_tileview_active(obj, &pages, active);
    Ok(pages)
}

/// Show tile `active` without animation.
#[cfg(feature = "widget-tileview")]
pub(crate) fn set_tileview_active(
    obj: NonNull<lv_obj_t>,
    pages: &[NonNull<lv_obj_t>],
    active: u32,
) {
    if let Some(page) = pages.get(active as usize) {
        // SAFETY: `obj` is a live tileview and `page` one of its tiles.
        unsafe { rs_lvgl_sys::lv_tileview_set_tile(obj.as_ptr(), page.as_ptr(), false) };
    }
}

/// Whether tile `active` is the tileview's active tile.
#[cfg(feature = "widget-tileview")]
pub(crate) fn tileview_is_active(
    obj: NonNull<lv_obj_t>,
    pages: &[NonNull<lv_obj_t>],
    active: u32,
) -> bool {
    // SAFETY: `obj` is a live tileview.
    let current = unsafe { rs_lvgl_sys::lv_tileview_get_tile_active(obj.as_ptr()) };
    pages.get(active as usize).map(|page| page.as_ptr()) == Some(current)
}

#[cfg(feature = "widget-tabview")]
pub(crate) fn tabview_pages(
    obj: NonNull<lv_obj_t>,
    tabs: &[String],
    active: u32,
) -> Result<Vec<NonNull<lv_obj_t>>, UiError> {
    let mut pages = Vec::with_capacity(tabs.len());
    for title in tabs {
        let title = CString::new(title.as_str()).map_err(|_| UiError::InvalidProperty)?;
        // SAFETY: `obj` is a live tabview object. LVGL copies the title.
        let page = unsafe { rs_lvgl_sys::lv_tabview_add_tab(obj.as_ptr(), title.as_ptr()) };
        let Some(page) = NonNull::new(page) else {
            return Err(UiError::Allocation);
        };
        pages.push(page);
    }
    // SAFETY: `obj` is a live tabview and `active` was validated.
    unsafe { rs_lvgl_sys::lv_tabview_set_active(obj.as_ptr(), active, false) };
    Ok(pages)
}

#[cfg(feature = "widget-tabview")]
pub(crate) fn set_tabview_titles(obj: NonNull<lv_obj_t>, old: &[String], new: &[String]) {
    for (index, (old, new)) in old.iter().zip(new).enumerate() {
        if old == new {
            continue;
        }
        let Ok(title) = CString::new(new.as_str()) else {
            continue;
        };
        // SAFETY: `obj` is a live tabview and `index` addresses an existing tab.
        unsafe {
            rs_lvgl_sys::lv_tabview_set_tab_text(obj.as_ptr(), index as u32, title.as_ptr());
        }
    }
}

#[cfg(feature = "widget-tabview")]
pub(crate) fn set_tabview_active(obj: NonNull<lv_obj_t>, active: u32) {
    // SAFETY: `obj` is a live tabview and `active` was validated.
    unsafe { rs_lvgl_sys::lv_tabview_set_active(obj.as_ptr(), active, false) };
}

/// Build the LVGL template arrays for a grid and keep them alive in the
/// mounted node.
pub(crate) fn grid_descriptors(
    columns: &[GridTrack],
    rows: &[GridTrack],
) -> (Box<[i32]>, Box<[i32]>) {
    fn tracks(tracks: &[GridTrack]) -> Box<[i32]> {
        let mut values: Vec<i32> = tracks.iter().map(|track| track.to_raw()).collect();
        // LV_GRID_TEMPLATE_LAST is LV_COORD_MAX.
        values.push(0x1fff_ffff);
        values.into_boxed_slice()
    }
    (tracks(columns), tracks(rows))
}

pub(crate) fn set_grid_descriptors(obj: NonNull<lv_obj_t>, columns: &[i32], rows: &[i32]) {
    // SAFETY: `obj` is a live object and both arrays are terminated and kept
    // alive by the mounted node for as long as LVGL references them.
    unsafe {
        rs_lvgl_sys::lv_obj_set_grid_dsc_array(obj.as_ptr(), columns.as_ptr(), rows.as_ptr());
    }
}

pub(crate) fn apply_grid_cell(obj: NonNull<lv_obj_t>, cell: GridCell) {
    // SAFETY: `obj` is a live object. Out-of-range cells are clamped by LVGL.
    unsafe {
        rs_lvgl_sys::lv_obj_set_grid_cell(
            obj.as_ptr(),
            cell.column_align.to_raw(),
            cell.column as i32,
            cell.column_span as i32,
            cell.row_align.to_raw(),
            cell.row as i32,
            cell.row_span as i32,
        );
    }
}

fn set_label_text(obj: NonNull<lv_obj_t>, text: &str) {
    let Ok(text) = CString::new(text) else {
        return;
    };
    // SAFETY: `obj` is a live label and `text` is a valid NUL-terminated
    // string. LVGL copies the text into its own allocation.
    unsafe { rs_lvgl_sys::lv_label_set_text(obj.as_ptr(), text.as_ptr()) };
}

#[cfg(any(feature = "widget-switch", feature = "widget-checkbox"))]
fn is_checked(obj: NonNull<lv_obj_t>) -> bool {
    // SAFETY: `obj` is a live object; the state query only reads it.
    unsafe { rs_lvgl_sys::lv_obj_has_state(obj.as_ptr(), rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED) }
}

/// Whether the native textarea already shows `text`.
#[cfg(feature = "widget-textarea")]
fn textarea_text_is(obj: NonNull<lv_obj_t>, text: &str) -> bool {
    // SAFETY: `obj` is a live textarea. The returned buffer is owned by LVGL
    // and is only read before this function returns.
    unsafe {
        let current = rs_lvgl_sys::lv_textarea_get_text(obj.as_ptr());
        if current.is_null() {
            return text.is_empty();
        }
        cstr_core::CStr::from_ptr(current).to_bytes() == text.as_bytes()
    }
}

fn suppress<M: Clone + 'static>(slot: Option<&HandlerSlot<M>>, write: impl FnOnce()) {
    if let Some(slot) = slot {
        slot.suppress();
        write();
        slot.clear_suppress();
    } else {
        write();
    }
}

const REMOVE_PROPS: [u32; 29] = [
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_BG_COLOR,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_BG_OPA,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_BORDER_COLOR,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_BORDER_WIDTH,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_RADIUS,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_PAD_TOP,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_PAD_BOTTOM,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_PAD_LEFT,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_PAD_RIGHT,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_PAD_ROW,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_PAD_COLUMN,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_WIDTH,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_HEIGHT,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_MAX_WIDTH,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_MAX_HEIGHT,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_MIN_WIDTH,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_MIN_HEIGHT,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_TEXT_ALIGN,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_FLEX_GROW,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_ALIGN,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_OPA,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_TEXT_COLOR,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_TEXT_FONT,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_ANIM_DURATION,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_X,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_Y,
    // Removing the flex placement restores `FlexAlign::Start`, the value the
    // containers are created with.
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_FLEX_MAIN_PLACE,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_FLEX_CROSS_PLACE,
    rs_lvgl_sys::_lv_style_id_t_LV_STYLE_FLEX_TRACK_PLACE,
];

/// Apply a resolved style to the main part as local properties.
///
/// Local properties are removed first so that resetting a field restores the
/// widget class default (for example a slider's track) instead of overriding it
/// with a neutral value.
pub(crate) fn apply_style(obj: NonNull<lv_obj_t>, style: &Style, tag: KindTag) {
    apply_style_selector(obj, style, tag, 0);
}

pub(crate) fn apply_style_for_selector(
    obj: NonNull<lv_obj_t>,
    style: &Style,
    tag: KindTag,
    selector: Selector,
) {
    apply_style_selector(obj, style, tag, selector.to_raw());
}

pub(crate) fn clear_selector_style(obj: NonNull<lv_obj_t>, selector: Selector) {
    let part = selector.part;
    let selector = selector.to_raw();
    crate::style_properties::ExtendedStyle::clear(obj, selector);
    // SAFETY: `obj` is a live object; every property id belongs to LVGL's
    // built-in style property set.
    unsafe {
        for prop in REMOVE_PROPS {
            rs_lvgl_sys::lv_obj_remove_local_style_prop(
                obj.as_ptr(),
                prop as rs_lvgl_sys::lv_style_prop_t,
                selector,
            );
        }
        if part == Part::Indicator {
            rs_lvgl_sys::lv_obj_remove_local_style_prop(
                obj.as_ptr(),
                rs_lvgl_sys::_lv_style_id_t_LV_STYLE_ARC_COLOR as rs_lvgl_sys::lv_style_prop_t,
                selector,
            );
            rs_lvgl_sys::lv_obj_remove_local_style_prop(
                obj.as_ptr(),
                rs_lvgl_sys::_lv_style_id_t_LV_STYLE_ARC_WIDTH as rs_lvgl_sys::lv_style_prop_t,
                selector,
            );
        }
    }
}

fn apply_style_selector(
    obj: NonNull<lv_obj_t>,
    style: &Style,
    tag: KindTag,
    selector: rs_lvgl_sys::lv_style_selector_t,
) {
    crate::style_properties::ExtendedStyle::clear(obj, selector);
    style.extended.apply(obj, selector);
    // SAFETY: `obj` is a live object; every property id belongs to LVGL's
    // built-in style property set.
    unsafe {
        for prop in REMOVE_PROPS {
            rs_lvgl_sys::lv_obj_remove_local_style_prop(
                obj.as_ptr(),
                prop as rs_lvgl_sys::lv_style_prop_t,
                selector,
            );
        }

        if style.is_set(STYLE_BG) {
            rs_lvgl_sys::lv_obj_set_style_bg_color(obj.as_ptr(), style.bg.to_raw(), selector);
        }
        if style.is_set(STYLE_BG_OPA) {
            rs_lvgl_sys::lv_obj_set_style_bg_opa(obj.as_ptr(), style.bg_opa, selector);
        }
        if style.is_set(STYLE_FG) {
            rs_lvgl_sys::lv_obj_set_style_text_color(obj.as_ptr(), style.fg.to_raw(), selector);
        }
        if style.is_set(STYLE_FONT) {
            rs_lvgl_sys::lv_obj_set_style_text_font(obj.as_ptr(), style.font.0, selector);
        }
        if style.is_set(STYLE_BORDER) {
            rs_lvgl_sys::lv_obj_set_style_border_color(
                obj.as_ptr(),
                style.border.to_raw(),
                selector,
            );
            rs_lvgl_sys::lv_obj_set_style_border_width(
                obj.as_ptr(),
                style.border_width as i32,
                selector,
            );
        }
        if style.is_set(STYLE_RADIUS) {
            rs_lvgl_sys::lv_obj_set_style_radius(obj.as_ptr(), style.radius as i32, selector);
        }
        if style.is_set(STYLE_PAD_LEFT) {
            rs_lvgl_sys::lv_obj_set_style_pad_left(obj.as_ptr(), style.pad_left as i32, selector);
        }
        if style.is_set(STYLE_PAD_RIGHT) {
            rs_lvgl_sys::lv_obj_set_style_pad_right(obj.as_ptr(), style.pad_right as i32, selector);
        }
        if style.is_set(STYLE_PAD_TOP) {
            rs_lvgl_sys::lv_obj_set_style_pad_top(obj.as_ptr(), style.pad_top as i32, selector);
        }
        if style.is_set(STYLE_PAD_BOTTOM) {
            rs_lvgl_sys::lv_obj_set_style_pad_bottom(
                obj.as_ptr(),
                style.pad_bottom as i32,
                selector,
            );
        }
        if style.is_set(STYLE_WIDTH) {
            set_size(obj, style.width, true, selector);
        }
        if style.is_set(STYLE_HEIGHT) {
            set_size(obj, style.height, false, selector);
        }
        if style.is_set(STYLE_MAX_WIDTH) {
            set_bound(obj, style.max_width, true, true, selector);
        }
        if style.is_set(STYLE_MAX_HEIGHT) {
            set_bound(obj, style.max_height, false, true, selector);
        }
        if style.is_set(STYLE_MIN_WIDTH) {
            set_bound(obj, style.min_width, true, false, selector);
        }
        if style.is_set(STYLE_MIN_HEIGHT) {
            set_bound(obj, style.min_height, false, false, selector);
        }
        if style.is_set(STYLE_TEXT_ALIGN) {
            rs_lvgl_sys::lv_obj_set_style_text_align(
                obj.as_ptr(),
                style.text_align.to_raw(),
                selector,
            );
        }
        if style.is_set(STYLE_ANIM_DURATION) {
            rs_lvgl_sys::lv_obj_set_style_anim_duration(
                obj.as_ptr(),
                style.anim_duration,
                selector,
            );
        }
        if style.is_set(STYLE_GROW) {
            rs_lvgl_sys::lv_obj_set_style_flex_grow(obj.as_ptr(), style.flex_grow, selector);
        }
        if style.is_set(STYLE_ALIGN) {
            rs_lvgl_sys::lv_obj_set_style_align(obj.as_ptr(), style.align.to_raw(), selector);
        }
        if style.is_set(STYLE_OPACITY) {
            rs_lvgl_sys::lv_obj_set_style_opa(obj.as_ptr(), style.opacity, selector);
        }
        if style.is_set(STYLE_OFFSET) {
            rs_lvgl_sys::lv_obj_set_style_x(obj.as_ptr(), style.offset_x as i32, selector);
            rs_lvgl_sys::lv_obj_set_style_y(obj.as_ptr(), style.offset_y as i32, selector);
        }

        // Object-wide state and layout properties only make sense on the main
        // part; a part style must not change the whole widget's visibility or
        // container layout.
        if selector == 0 {
            // Mounted nodes are never floating by class default, so an unset
            // field clears the flag.
            rs_lvgl_sys::lv_obj_set_floating(
                obj.as_ptr(),
                style.is_set(STYLE_FLOATING) && style.floating,
            );
            if style.is_set(STYLE_HIDDEN) {
                rs_lvgl_sys::lv_obj_set_hidden(obj.as_ptr(), style.hidden);
            }
            if style.is_set(STYLE_DISABLED) {
                if style.disabled {
                    rs_lvgl_sys::lv_obj_add_state(
                        obj.as_ptr(),
                        rs_lvgl_sys::lv_state_t_LV_STATE_DISABLED,
                    );
                } else {
                    rs_lvgl_sys::lv_obj_remove_state(
                        obj.as_ptr(),
                        rs_lvgl_sys::lv_state_t_LV_STATE_DISABLED,
                    );
                }
            }
            #[cfg(feature = "widget-textarea")]
            if tag == KindTag::Textarea
                && !style.is_set(STYLE_HEIGHT)
                && rs_lvgl_sys::lv_textarea_get_one_line(obj.as_ptr())
            {
                rs_lvgl_sys::lv_obj_set_height(obj.as_ptr(), LV_SIZE_CONTENT);
            }
            if style.is_set(STYLE_FLEX_ALIGN) && matches!(tag, KindTag::Row | KindTag::Column) {
                rs_lvgl_sys::lv_obj_set_style_flex_main_place(
                    obj.as_ptr(),
                    style.flex_main.to_raw(),
                    0,
                );
                rs_lvgl_sys::lv_obj_set_style_flex_cross_place(
                    obj.as_ptr(),
                    style.flex_cross.to_raw(),
                    0,
                );
                // LVGL places items inside their line (cross) and lines inside
                // the container (track); a single-line container needs both
                // to move its items.
                rs_lvgl_sys::lv_obj_set_style_flex_track_place(
                    obj.as_ptr(),
                    style.flex_cross.to_raw(),
                    0,
                );
            }
            if style.is_set(STYLE_GAP) {
                match tag {
                    KindTag::Row => {
                        rs_lvgl_sys::lv_obj_set_style_pad_column(obj.as_ptr(), style.gap as i32, 0);
                    }
                    KindTag::Column => {
                        rs_lvgl_sys::lv_obj_set_style_pad_row(obj.as_ptr(), style.gap as i32, 0);
                    }
                    KindTag::Grid => {
                        rs_lvgl_sys::lv_obj_set_style_pad_row(obj.as_ptr(), style.gap as i32, 0);
                        rs_lvgl_sys::lv_obj_set_style_pad_column(obj.as_ptr(), style.gap as i32, 0);
                    }
                    _ => {}
                }
            }
        }

        #[cfg(feature = "widget-arc")]
        if tag == KindTag::Arc
            && (selector == 0 || selector == rs_lvgl_sys::lv_part_t_LV_PART_INDICATOR)
        {
            rs_lvgl_sys::lv_obj_remove_local_style_prop(
                obj.as_ptr(),
                rs_lvgl_sys::_lv_style_id_t_LV_STYLE_ARC_COLOR as rs_lvgl_sys::lv_style_prop_t,
                rs_lvgl_sys::lv_part_t_LV_PART_INDICATOR,
            );
            rs_lvgl_sys::lv_obj_remove_local_style_prop(
                obj.as_ptr(),
                rs_lvgl_sys::_lv_style_id_t_LV_STYLE_ARC_WIDTH as rs_lvgl_sys::lv_style_prop_t,
                rs_lvgl_sys::lv_part_t_LV_PART_INDICATOR,
            );
            if style.is_set(STYLE_ARC_COLOR) {
                rs_lvgl_sys::lv_obj_set_style_arc_color(
                    obj.as_ptr(),
                    style.arc_color.to_raw(),
                    rs_lvgl_sys::lv_part_t_LV_PART_INDICATOR,
                );
            }
            if style.is_set(STYLE_ARC_WIDTH) {
                rs_lvgl_sys::lv_obj_set_style_arc_width(
                    obj.as_ptr(),
                    style.arc_width as i32,
                    rs_lvgl_sys::lv_part_t_LV_PART_INDICATOR,
                );
            }
        }
    }
}

/// `LV_SIZE_CONTENT` is a C macro; LVGL encodes it as a special coordinate.
const LV_SIZE_CONTENT: i32 = 0x3fff_ffff;

fn size_value(size: Size) -> Option<i32> {
    match size {
        Size::Px(value) => Some(value as i32),
        Size::Percent(percent) => {
            // SAFETY: `lv_pct` is a pure coordinate conversion helper.
            Some(unsafe { rs_lvgl_sys::lv_pct(percent as i32) })
        }
        Size::Content => None,
    }
}

fn set_size(
    obj: NonNull<lv_obj_t>,
    size: Size,
    horizontal: bool,
    selector: rs_lvgl_sys::lv_style_selector_t,
) {
    let value = size_value(size).unwrap_or(LV_SIZE_CONTENT);
    // SAFETY: `obj` is a live object.
    unsafe {
        if horizontal {
            rs_lvgl_sys::lv_obj_set_style_width(obj.as_ptr(), value, selector);
        } else {
            rs_lvgl_sys::lv_obj_set_style_height(obj.as_ptr(), value, selector);
        }
    }
}

fn set_bound(
    obj: NonNull<lv_obj_t>,
    size: Size,
    horizontal: bool,
    maximum: bool,
    selector: rs_lvgl_sys::lv_style_selector_t,
) {
    let Some(value) = size_value(size) else {
        return;
    };
    // SAFETY: `obj` is a live object.
    unsafe {
        match (horizontal, maximum) {
            (true, true) => rs_lvgl_sys::lv_obj_set_style_max_width(obj.as_ptr(), value, selector),
            (false, true) => {
                rs_lvgl_sys::lv_obj_set_style_max_height(obj.as_ptr(), value, selector)
            }
            (true, false) => rs_lvgl_sys::lv_obj_set_style_min_width(obj.as_ptr(), value, selector),
            (false, false) => {
                rs_lvgl_sys::lv_obj_set_style_min_height(obj.as_ptr(), value, selector)
            }
        }
    }
}
