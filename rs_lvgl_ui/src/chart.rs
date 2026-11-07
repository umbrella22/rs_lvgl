//! Owned descriptions for the advanced chart path.

use crate::{Color, UiError};
use alloc::vec::Vec;

/// Presentation for [`crate::chart`].
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartType {
    Line,
    Bar,
    Stacked,
    /// Requires the effective provider's `LV_USE_VECTOR_GRAPHIC` capability.
    Curve,
    Scatter,
}

impl ChartType {
    pub(crate) fn raw(self) -> rs_lvgl_sys::lv_chart_type_t {
        use rs_lvgl_sys::*;
        match self {
            Self::Line => lv_chart_type_t_LV_CHART_TYPE_LINE,
            Self::Bar => lv_chart_type_t_LV_CHART_TYPE_BAR,
            Self::Stacked => lv_chart_type_t_LV_CHART_TYPE_STACKED,
            Self::Curve => lv_chart_type_t_LV_CHART_TYPE_CURVE,
            Self::Scatter => lv_chart_type_t_LV_CHART_TYPE_SCATTER,
        }
    }
}

/// Axis selection for a series. X selection affects scatter charts.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ChartAxis {
    #[default]
    Primary,
    Secondary,
}

/// An inclusive display range, validated when the view is rendered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChartRange {
    pub min: i32,
    pub max: i32,
}

impl ChartRange {
    pub const fn new(min: i32, max: i32) -> Self {
        Self { min, max }
    }

    fn validate(self) -> Result<(), UiError> {
        // The pinned native mapper subtracts endpoints in signed int32_t.
        if self.min >= self.max || self.max.checked_sub(self.min).is_none() {
            return Err(UiError::InvalidProperty);
        }
        Ok(())
    }
}

/// Independent primary/secondary X/Y display ranges.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChartAxes {
    pub primary_x: ChartRange,
    pub primary_y: ChartRange,
    pub secondary_x: ChartRange,
    pub secondary_y: ChartRange,
}

impl ChartAxes {
    /// Initialize both axis pairs to the same ranges.
    pub const fn new(x: ChartRange, y: ChartRange) -> Self {
        Self {
            primary_x: x,
            primary_y: y,
            secondary_x: x,
            secondary_y: y,
        }
    }

    pub(crate) fn x(self, axis: ChartAxis) -> ChartRange {
        match axis {
            ChartAxis::Primary => self.primary_x,
            ChartAxis::Secondary => self.secondary_x,
        }
    }

    pub(crate) fn y(self, axis: ChartAxis) -> ChartRange {
        match axis {
            ChartAxis::Primary => self.primary_y,
            ChartAxis::Secondary => self.secondary_y,
        }
    }
}

/// A sample value or an explicit gap. `Value(i32::MAX)` is rejected because
/// LVGL reserves that representation for a gap.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartValue {
    Value(i32),
    Missing,
}

impl From<i32> for ChartValue {
    fn from(value: i32) -> Self {
        Self::Value(value)
    }
}

impl ChartValue {
    pub(crate) fn raw(self) -> i32 {
        match self {
            Self::Value(value) => value,
            Self::Missing => i32::MAX,
        }
    }

    pub(crate) fn validate(self) -> Result<(), UiError> {
        if self == Self::Value(i32::MAX) {
            Err(UiError::InvalidProperty)
        } else {
            Ok(())
        }
    }
}

/// Owned samples. XY is required for scatter and forbidden for other forms.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChartData {
    History(crate::ChartHistorySnapshot),
    Y(Vec<ChartValue>),
    XY {
        x: Vec<ChartValue>,
        y: Vec<ChartValue>,
    },
}

impl ChartData {
    pub fn values(values: impl IntoIterator<Item = i32>) -> Self {
        Self::Y(values.into_iter().map(ChartValue::Value).collect())
    }

    pub fn xy(values: impl IntoIterator<Item = (i32, i32)>) -> Self {
        let (x, y) = values
            .into_iter()
            .map(|(x, y)| (ChartValue::Value(x), ChartValue::Value(y)))
            .unzip();
        Self::XY { x, y }
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Y(y) | Self::XY { y, .. } => y.len(),
            Self::History(h) => h.capacity().get() as usize,
        }
    }

    pub(crate) fn xy_kind(&self) -> bool {
        matches!(self, Self::XY { .. })
            || matches!(self,Self::History(h) if h.kind() == crate::ChartSampleKind::XY)
    }

    pub(crate) fn sample(&self, index: usize, mode: ChartUpdateMode) -> (ChartValue, ChartValue) {
        match self {
            Self::History(h) => h.displayed(index, mode),
            Self::Y(y) => (ChartValue::Missing, y[index]),
            Self::XY { x, y } => (x[index], y[index]),
        }
    }
    pub(crate) fn stored_len(&self) -> usize {
        match self {
            Self::History(h) => h.samples().len(),
            _ => self.len(),
        }
    }

    pub(crate) fn stored_sample(&self, index: usize) -> (ChartValue, ChartValue) {
        match self {
            Self::History(h) => h.samples()[index].pair(),
            _ => self.sample(index, ChartUpdateMode::Shift),
        }
    }

    pub(crate) fn display_index(&self, stored_index: usize, mode: ChartUpdateMode) -> usize {
        match self {
            Self::History(h) => match mode {
                ChartUpdateMode::Shift => self.len() - h.samples().len() + stored_index,
                ChartUpdateMode::Circular => {
                    ((h.first_sequence() + stored_index as u64) % self.len() as u64) as usize
                }
            },
            _ => stored_index,
        }
    }
}

/// A stable series identity, color, axes and owned samples.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChartSeries {
    pub id: u64,
    pub color: Color,
    pub x_axis: ChartAxis,
    pub y_axis: ChartAxis,
    pub data: ChartData,
}

impl ChartSeries {
    pub fn new(id: u64, color: Color, data: ChartData) -> Self {
        Self {
            id,
            color,
            data,
            x_axis: ChartAxis::Primary,
            y_axis: ChartAxis::Primary,
        }
    }

    pub fn axes(mut self, x: ChartAxis, y: ChartAxis) -> Self {
        self.x_axis = x;
        self.y_axis = y;
        self
    }
}

/// Native history display order; static vectors retain their supplied order.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ChartUpdateMode {
    #[default]
    Shift,
    Circular,
}

/// Cursor lines through a position.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartCursorDirection {
    Horizontal,
    Vertical,
    Both,
}

/// Cursor coordinates are chart-local pixels, or a displayed sample slot
/// counted left to right. A missing sample hides its point cursor.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartCursorTarget {
    Position { x: i32, y: i32 },
    Point { series: u64, index: u32 },
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChartCursor {
    pub id: u64,
    pub color: Color,
    pub direction: ChartCursorDirection,
    pub target: ChartCursorTarget,
}

impl ChartCursor {
    pub fn new(
        id: u64,
        color: Color,
        direction: ChartCursorDirection,
        target: ChartCursorTarget,
    ) -> Self {
        Self {
            id,
            color,
            direction,
            target,
        }
    }
}

/// Advanced owned chart descriptor. Invalid descriptions fail before either
/// mounted tree changes. Empty series membership is valid; nonempty series
/// need a common nonzero point count.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChartSpec {
    pub presentation: ChartType,
    pub axes: ChartAxes,
    pub update_mode: ChartUpdateMode,
    pub series: Vec<ChartSeries>,
    pub cursors: Vec<ChartCursor>,
}

impl ChartSpec {
    pub fn new(presentation: ChartType, axes: ChartAxes, series: Vec<ChartSeries>) -> Self {
        Self {
            presentation,
            axes,
            series,
            update_mode: ChartUpdateMode::Shift,
            cursors: Vec::new(),
        }
    }

    pub(crate) fn points(&self) -> usize {
        self.series.first().map(|s| s.data.len()).unwrap_or(1)
    }

    pub(crate) fn validate(&self) -> Result<(), UiError> {
        for range in [
            self.axes.primary_x,
            self.axes.primary_y,
            self.axes.secondary_x,
            self.axes.secondary_y,
        ] {
            range.validate()?;
        }
        if self.presentation == ChartType::Curve && rs_lvgl_sys::LV_USE_VECTOR_GRAPHIC == 0 {
            return Err(UiError::InvalidProperty);
        }
        let points = self.points();
        // Native indexes, allocations and draw-loop intermediates use u32/i32.
        if points == 0
            || points > i32::MAX as usize
            || points
                .checked_mul(core::mem::size_of::<i32>())
                .filter(|n| *n <= isize::MAX as usize)
                .is_none()
        {
            return Err(UiError::InvalidProperty);
        }
        for (i, series) in self.series.iter().enumerate() {
            if self.series[..i].iter().any(|s| s.id == series.id)
                || series.data.len() != points
                || series.data.xy_kind() != (self.presentation == ChartType::Scatter)
            {
                return Err(UiError::InvalidProperty);
            }
            if let ChartData::XY { x, y } = &series.data {
                if x.len() != y.len() {
                    return Err(UiError::InvalidProperty);
                }
            }
            // Stacking uses one common cumulative Y scale in the native renderer.
            if self.presentation == ChartType::Stacked && series.y_axis != self.series[0].y_axis {
                return Err(UiError::InvalidProperty);
            }
            if self.presentation == ChartType::Bar && series.data.stored_len() != points {
                return Err(UiError::InvalidProperty);
            }
            for index in 0..series.data.stored_len() {
                let (x, y) = series.data.stored_sample(index);
                x.validate()?;
                y.validate()?;
                if self.presentation == ChartType::Bar {
                    let ChartValue::Value(y) = y else {
                        return Err(UiError::InvalidProperty);
                    };
                    if y.checked_sub(self.axes.y(series.y_axis).min).is_none() {
                        return Err(UiError::InvalidProperty);
                    }
                }
                if self.presentation == ChartType::Stacked
                    && matches!(y, ChartValue::Value(v) if v < 0)
                {
                    return Err(UiError::InvalidProperty);
                }
            }
        }
        if self.presentation == ChartType::Stacked {
            // Only occupied slots can contribute. Iterating stored samples also
            // keeps empty large-capacity histories independent of capacity.
            for source in &self.series {
                for stored in 0..source.data.stored_len() {
                    let index = source.data.display_index(stored, self.update_mode);
                    let mut sum = 0i32;
                    for series in &self.series {
                        if let ChartValue::Value(value) =
                            series.data.sample(index, self.update_mode).1
                        {
                            sum = sum.checked_add(value).ok_or(UiError::InvalidProperty)?;
                        }
                    }
                }
            }
        }
        for (i, cursor) in self.cursors.iter().enumerate() {
            if self.cursors[..i].iter().any(|c| c.id == cursor.id) {
                return Err(UiError::InvalidProperty);
            }
            match cursor.target {
                ChartCursorTarget::Point { series, index } => {
                    if index as usize >= points || !self.series.iter().any(|s| s.id == series) {
                        return Err(UiError::InvalidProperty);
                    }
                }
                ChartCursorTarget::Position { x, y } => {
                    // LVGL adds the widget origin and draws cursor marker bounds.
                    if x.unsigned_abs() > rs_lvgl_sys::LV_COORD_MAX as u32
                        || y.unsigned_abs() > rs_lvgl_sys::LV_COORD_MAX as u32
                    {
                        return Err(UiError::InvalidProperty);
                    }
                }
            }
        }
        Ok(())
    }
}
