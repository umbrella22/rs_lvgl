//! Widget capability and native representation validation.

use super::{is_container, is_textarea};
use crate::node::{
    GridTrack, ImageSource, Node, WidgetKind, ANIMATED_IMAGE_MAX_FRAMES, SPINBOX_MAX_DIGITS,
    TABLE_MAX_CELLS_PER_AXIS,
};
use crate::UiError;
use alloc::vec::Vec;

#[cfg(any(
    feature = "widget-slider",
    feature = "widget-bar",
    feature = "widget-arc",
    feature = "widget-spinbox"
))]
fn validate_range(min: i32, max: i32, value: i32) -> Result<(), UiError> {
    if min > max || value < min || value > max {
        return Err(UiError::InvalidProperty);
    }
    Ok(())
}

pub(super) fn validate_kind<M>(node: &Node<M>) -> Result<(), UiError> {
    if node.handlers.text_changed.is_some() && !is_textarea(&node.kind) {
        return Err(UiError::InvalidProperty);
    }
    let container = is_container(&node.kind);
    if !container && !node.children.is_empty() {
        return Err(UiError::InvalidProperty);
    }

    if node.keyboard_id.is_some() && !is_textarea(&node.kind) {
        return Err(UiError::InvalidProperty);
    }
    if let Some(cell) = node.grid_cell {
        if cell.column_span == 0 || cell.row_span == 0 {
            return Err(UiError::InvalidProperty);
        }
    }

    match &node.kind {
        #[cfg(feature = "widget-ime-pinyin")]
        WidgetKind::PinyinKeyboard { font, .. } => {
            if font.0.is_null() {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-qrcode")]
        WidgetKind::QrCode { code } => code.validate()?,
        #[cfg(feature = "widget-barcode")]
        WidgetKind::Barcode { code } => code.validate()?,
        #[cfg(feature = "widget-gif")]
        WidgetKind::Gif { animation } => animation.validate()?,
        #[cfg(feature = "widget-lottie")]
        WidgetKind::Lottie { animation } => animation.validate()?,

        #[cfg(feature = "widget-canvas")]
        WidgetKind::Canvas { frame } => frame.validate()?,
        WidgetKind::Grid { columns, rows } => {
            if columns.is_empty() || rows.is_empty() {
                return Err(UiError::InvalidProperty);
            }
            for track in columns.iter().chain(rows) {
                match track {
                    GridTrack::Px(value) if *value < 0 => return Err(UiError::InvalidProperty),
                    GridTrack::Fr(value) if *value == 0 => return Err(UiError::InvalidProperty),
                    _ => {}
                }
            }
        }
        #[cfg(feature = "widget-tabview")]
        WidgetKind::Tabview { tabs, active } => {
            if tabs.is_empty()
                || *active as usize >= tabs.len()
                || node.children.len() != tabs.len()
            {
                return Err(UiError::InvalidProperty);
            }
            if tabs.iter().any(|tab| tab.as_bytes().contains(&0)) {
                return Err(UiError::InvalidProperty);
            }
        }
        WidgetKind::Label { text, .. } | WidgetKind::Button { text } => {
            if text.as_bytes().contains(&0) {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-checkbox")]
        WidgetKind::Checkbox { text, .. } => {
            if text.as_bytes().contains(&0) {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-textarea")]
        WidgetKind::Textarea {
            text,
            placeholder,
            input,
            ..
        } => {
            if let Some(options) = input {
                options.validate(text)?;
            }
            if text.as_bytes().contains(&0) || placeholder.as_bytes().contains(&0) {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-dropdown")]
        WidgetKind::Dropdown { options, selected } => {
            validate_options(options, *selected)?;
        }
        #[cfg(feature = "widget-arc")]
        WidgetKind::Arc { min, max, value } => {
            validate_range(*min, *max, *value)?;
        }
        #[cfg(feature = "widget-scale")]
        WidgetKind::Scale {
            min,
            max,
            total_ticks,
            major_tick_every,
        } => {
            if min > max
                || *total_ticks < 2
                || *major_tick_every == 0
                || *major_tick_every > *total_ticks
            {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-chart")]
        WidgetKind::Chart(spec) => spec.validate()?,
        #[cfg(feature = "widget-slider")]
        WidgetKind::RangeSlider { min, max, range } => range.validate_native(*min, *max)?,
        #[cfg(feature = "widget-slider")]
        WidgetKind::Slider { min, max, value } => validate_range(*min, *max, *value)?,
        #[cfg(feature = "widget-bar")]
        WidgetKind::ProgressBar { min, max, value } => validate_range(*min, *max, *value)?,
        #[cfg(feature = "widget-image")]
        WidgetKind::Image { source, scale } => {
            scale.validate_image()?;
            validate_image_source(source)?;
        }
        #[cfg(feature = "widget-roller")]
        WidgetKind::Roller {
            options,
            selected,
            visible_rows,
            mode: _,
        } => {
            if *visible_rows == 0 {
                return Err(UiError::InvalidProperty);
            }
            validate_options(options, *selected)?;
        }
        #[cfg(feature = "widget-spinbox")]
        WidgetKind::Spinbox {
            min,
            max,
            value,
            digit_count,
            decimal_pos,
            step,
        } => {
            validate_range(*min, *max, *value)?;
            if *digit_count == 0
                || *digit_count > SPINBOX_MAX_DIGITS
                || *decimal_pos >= *digit_count
            {
                return Err(UiError::InvalidProperty);
            }
            if *step == 0 {
                return Err(UiError::InvalidProperty);
            }
            // LVGL silently clamps the range to the representable digits.
            let limit = 10i64.pow(*digit_count) - 1;
            if i64::from(*max) > limit || i64::from(*min) < -limit {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-spinner")]
        WidgetKind::Spinner {
            duration_ms,
            arc_angle,
        } => {
            if *duration_ms == 0 || *arc_angle == 0 || *arc_angle > 360 {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-line")]
        WidgetKind::Line { points } => {
            if points.len() > u32::MAX as usize
                || points.iter().any(|&(x, y)| {
                    // lv_value_precise_t changes from i32 to f32 when a vector
                    // renderer is selected. Reject integers it cannot represent.
                    [x, y]
                        .into_iter()
                        .any(|v| (v as rs_lvgl_sys::lv_value_precise_t) as f64 != f64::from(v))
                })
            {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-buttonmatrix")]
        WidgetKind::ButtonMatrix { rows } => {
            if rows.is_empty() || rows.iter().any(Vec::is_empty) {
                return Err(UiError::InvalidProperty);
            }
            // An empty string terminates LVGL's map and "\n" separates rows.
            if rows
                .iter()
                .flatten()
                .any(|text| text.is_empty() || text == "\n" || text.as_bytes().contains(&0))
            {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-table")]
        WidgetKind::Table {
            cells,
            column_widths,
        } => {
            let columns = cells.first().map(Vec::len).unwrap_or(0);
            if cells.is_empty()
                || columns == 0
                || cells.len() > TABLE_MAX_CELLS_PER_AXIS
                || columns > TABLE_MAX_CELLS_PER_AXIS
                || cells.iter().any(|row| row.len() != columns)
                || column_widths.len() > columns
                || column_widths.iter().any(|width| *width < 0)
            {
                return Err(UiError::InvalidProperty);
            }
            if cells
                .iter()
                .flatten()
                .any(|text| text.as_bytes().contains(&0))
            {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-tileview")]
        WidgetKind::Tileview { tiles, active } => {
            if tiles.is_empty()
                || *active as usize >= tiles.len()
                || node.children.len() != tiles.len()
            {
                return Err(UiError::InvalidProperty);
            }
            for (index, tile) in tiles.iter().enumerate() {
                if tiles[..index]
                    .iter()
                    .any(|other| (other.column, other.row) == (tile.column, tile.row))
                {
                    return Err(UiError::InvalidProperty);
                }
            }
        }
        #[cfg(feature = "widget-msgbox")]
        WidgetKind::MessageBox {
            title,
            text,
            buttons,
        } => {
            if title
                .iter()
                .chain(core::iter::once(text))
                .chain(buttons)
                .any(|text| text.as_bytes().contains(&0))
            {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-calendar")]
        WidgetKind::Calendar {
            year: _,
            month,
            today,
            highlighted,
        } => {
            if !(1..=12).contains(month)
                || today.map(|date| !date.is_valid()).unwrap_or(false)
                || highlighted.iter().any(|date| !date.is_valid())
            {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-span")]
        WidgetKind::Spans { spans } => {
            if spans.iter().any(|span| span.text.as_bytes().contains(&0)) {
                return Err(UiError::InvalidProperty);
            }
        }
        #[cfg(feature = "widget-imagebutton")]
        WidgetKind::ImageButton { released, pressed } => {
            for source in core::iter::once(released).chain(pressed) {
                validate_image_source(source)?;
            }
        }
        #[cfg(feature = "widget-animimage")]
        WidgetKind::AnimatedImage {
            frames,
            duration_ms,
            repeat: _,
        } => {
            if frames.is_empty() || frames.len() > ANIMATED_IMAGE_MAX_FRAMES || *duration_ms == 0 {
                return Err(UiError::InvalidProperty);
            }
            for frame in frames {
                validate_image_source(frame)?;
            }
        }
        #[cfg(feature = "widget-arclabel")]
        WidgetKind::ArcLabel {
            text, angle_size, ..
        } => {
            if text.as_bytes().contains(&0) || !(0..=360).contains(angle_size) {
                return Err(UiError::InvalidProperty);
            }
        }
        WidgetKind::Container | WidgetKind::Row | WidgetKind::Column => {}
        #[cfg(feature = "widget-switch")]
        WidgetKind::Switch { .. } => {}
        #[cfg(feature = "widget-led")]
        WidgetKind::Led { .. } => {}
        #[cfg(feature = "widget-keyboard")]
        WidgetKind::Keyboard { .. } => {}
        WidgetKind::Custom(custom) => {
            custom.validate()?;
            // `crate::raw` owns its native callbacks; lifecycle widgets may use
            // the typed handler because the reconciler registers it on the
            // custom root object.
            if !custom.is_lifecycle() && !node.handlers.is_empty() {
                return Err(UiError::InvalidProperty);
            }
        }
    }
    Ok(())
}

fn validate_image_source(source: &ImageSource) -> Result<(), UiError> {
    match source {
        ImageSource::Raw(raw) if raw.as_ptr().is_null() => Err(UiError::InvalidProperty),
        source if source.is_pixels() => {
            if source.valid_pixels() { Ok(()) } else { Err(UiError::InvalidProperty) }
        }
        _ => Ok(()),
    }
}

#[cfg(any(feature = "widget-dropdown", feature = "widget-roller"))]
fn validate_options(options: &[alloc::string::String], selected: u32) -> Result<(), UiError> {
    if options.is_empty()
        || selected as usize >= options.len()
        || options
            .iter()
            .any(|option| option.as_bytes().contains(&0) || option.contains('\n'))
    {
        return Err(UiError::InvalidProperty);
    }
    Ok(())
}
