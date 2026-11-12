use super::DisplayConfigError;
use crate::sys;

/// Validated logical pixel granularity, independent of buffer byte alignment.
pub(super) struct InvalidationAlignment {
    columns: u32,
    rows: u32,
    width: u32,
    height: u32,
}

impl InvalidationAlignment {
    pub fn new(
        columns: u32,
        rows: u32,
        width: u32,
        height: u32,
    ) -> Result<Self, DisplayConfigError> {
        if columns == 0 || rows == 0 {
            return Err(DisplayConfigError::InvalidInvalidationAlignment);
        }
        Ok(Self {
            columns,
            rows,
            width,
            height,
        })
    }

    pub fn minimum_buffer_rows(&self) -> u32 {
        self.rows.min(self.height)
    }

    pub fn expand(&self, area: &mut sys::lv_area_t) {
        fn axis(start: &mut i32, end: &mut i32, granularity: u32, extent: u32) {
            // The native invalidation path supplies screen-clipped coordinates.
            // i64 also covers rounding by any nonzero u32 granularity.
            let step = i64::from(granularity);
            let lower = i64::from(*start).div_euclid(step) * step;
            let upper = (i64::from(*end).div_euclid(step) + 1) * step - 1;
            *start = lower.max(0) as i32;
            *end = upper.min(i64::from(extent) - 1) as i32;
        }
        axis(&mut area.x1, &mut area.x2, self.columns, self.width);
        axis(&mut area.y1, &mut area.y2, self.rows, self.height);
    }

    pub fn fit_partial_probe(&self, area: &mut sys::lv_area_t) {
        // LVGL 9.6 get_max_row sends a synthetic (0,0)-(0,max_rows-1)
        // during RENDER_START..RENDER_READY. Fit complete row groups downward.
        // A shorter height is the clipped bottom group (or the entire display
        // when granularity exceeds its height); it must remain nonempty.
        let height = area.y2 as u32 + 1;
        if height >= self.rows {
            area.y2 = (height / self.rows * self.rows - 1) as i32;
        }
    }
}
