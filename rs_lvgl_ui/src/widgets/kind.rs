//! Widget identity and native event value sources.

use crate::events::ValueSource;
use crate::node::WidgetKind;

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
pub enum KindTag {
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
    Container,
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
    ProgressBar,
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
    MessageBox,
    #[cfg(feature = "widget-calendar")]
    Calendar,
    #[cfg(feature = "widget-span")]
    Spans,
    #[cfg(feature = "widget-imagebutton")]
    ImageButton,
    #[cfg(feature = "widget-animimage")]
    AnimatedImage,
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
        WidgetKind::Container => KindTag::Container,
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
        WidgetKind::ProgressBar { .. } => KindTag::ProgressBar,
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
        WidgetKind::MessageBox { .. } => KindTag::MessageBox,
        #[cfg(feature = "widget-calendar")]
        WidgetKind::Calendar { .. } => KindTag::Calendar,
        #[cfg(feature = "widget-span")]
        WidgetKind::Spans { .. } => KindTag::Spans,
        #[cfg(feature = "widget-imagebutton")]
        WidgetKind::ImageButton { .. } => KindTag::ImageButton,
        #[cfg(feature = "widget-animimage")]
        WidgetKind::AnimatedImage { .. } => KindTag::AnimatedImage,
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
        KindTag::ProgressBar => ValueSource::ProgressBar,
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
        KindTag::MessageBox => ValueSource::EventIndex,
        #[cfg(feature = "widget-calendar")]
        KindTag::Calendar => ValueSource::Calendar,
        _ => ValueSource::None,
    }
}
