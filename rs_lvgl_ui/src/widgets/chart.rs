//! Native advanced-chart resources. All arrays are owned and freed by LVGL.

use super::Retained;
use crate::chart::*;
use crate::UiError;
use alloc::{boxed::Box, vec::Vec};
use core::ptr::NonNull;
use rs_lvgl_sys as sys;

pub(crate) struct ChartState {
    series: Vec<(u64, NonNull<sys::lv_chart_series_t>)>,
    cursors: Vec<(u64, NonNull<sys::lv_chart_cursor_t>)>,
    // A separate allocation keeps callback data stable when the mounted state
    // moves. It is retained through native deletion, including failed updates.
    draw: Box<DrawContext>,
}

struct DrawContext {
    bindings: Vec<CursorBinding>,
    presentation: ChartType,
    points: usize,
}
struct CursorBinding {
    cursor: NonNull<sys::lv_chart_cursor_t>,
    position: CursorPosition,
}
enum CursorPosition {
    Hidden,
    Pixels(i32, i32),
    Sample {
        index: usize,
        series_index: usize,
        series_count: usize,
        x: ChartValue,
        y: i32,
        x_range: ChartRange,
        y_range: ChartRange,
    },
}

pub(crate) fn apply(
    obj: NonNull<sys::lv_obj_t>,
    spec: &ChartSpec,
    old: Option<&ChartSpec>,
    retained: &mut Retained,
) -> Result<(), UiError> {
    if !matches!(retained, Retained::Chart(_)) {
        let mut state = Box::new(ChartState {
            series: Vec::new(),
            cursors: Vec::new(),
            draw: Box::new(DrawContext {
                bindings: Vec::new(),
                presentation: spec.presentation,
                points: spec.points(),
            }),
        });
        // SAFETY: live chart; stable callback allocation moves into `retained`
        // before any later fallible native operation. No borrowed samples enter C.
        let registration = unsafe {
            sys::lv_obj_add_event_cb(
                obj.as_ptr(),
                Some(resolve_cursors),
                sys::lv_event_code_t_LV_EVENT_ALL,
                (&mut *state.draw as *mut DrawContext).cast(),
            )
        };
        if registration.is_null() {
            return Err(UiError::Allocation);
        }
        *retained = Retained::Chart(state);
    }
    let Retained::Chart(state) = retained else {
        unreachable!()
    };
    let reset = old
        .map(|s| s.points() != spec.points() || s.presentation != spec.presentation)
        .unwrap_or(true);
    // Empty all cursor references before removing or reordering series. Keeping
    // the callback allocation itself alive also covers early-error teardown.
    state.draw.bindings.clear();
    // SAFETY: this mounted state owns the chart and its private resources. Native
    // setters invalidate only; drawing cannot run concurrently on the UI thread.
    unsafe {
        // Cursor membership is small and native cursor fields have no public
        // color/direction setters. Recreate only cursors, never compatible series.
        for (_, cursor) in state.cursors.drain(..) {
            sys::lv_chart_remove_cursor(obj.as_ptr(), cursor.as_ptr());
        }
        if reset {
            for (_, series) in state.series.drain(..) {
                sys::lv_chart_remove_series(obj.as_ptr(), series.as_ptr());
            }
            // These void setters cannot fail with an empty series list.
            sys::lv_chart_set_type(obj.as_ptr(), spec.presentation.raw());
            sys::lv_chart_set_point_count(obj.as_ptr(), spec.points() as u32);
        } else {
            // The public native API can append and remove series, but cannot
            // reorder or reassign their axes. Preserve the compatible prefix;
            // retire/recreate only the incompatible suffix in descriptor order.
            let keep = state
                .series
                .iter()
                .zip(&spec.series)
                .take_while(|((id, _), new)| {
                    *id == new.id
                        && old
                            .and_then(|s| s.series.iter().find(|s| s.id == *id))
                            .map(|s| (s.x_axis, s.y_axis))
                            == Some((new.x_axis, new.y_axis))
                })
                .count();
            for (_, series) in state.series.drain(keep..) {
                sys::lv_chart_remove_series(obj.as_ptr(), series.as_ptr());
            }
        }
        for (axis, range) in [
            (
                sys::lv_chart_axis_t_LV_CHART_AXIS_PRIMARY_X,
                spec.axes.primary_x,
            ),
            (
                sys::lv_chart_axis_t_LV_CHART_AXIS_PRIMARY_Y,
                spec.axes.primary_y,
            ),
            (
                sys::lv_chart_axis_t_LV_CHART_AXIS_SECONDARY_X,
                spec.axes.secondary_x,
            ),
            (
                sys::lv_chart_axis_t_LV_CHART_AXIS_SECONDARY_Y,
                spec.axes.secondary_y,
            ),
        ] {
            sys::lv_chart_set_axis_range(obj.as_ptr(), axis, range.min, range.max);
        }
        sys::lv_chart_set_update_mode(
            obj.as_ptr(),
            match spec.update_mode {
                ChartUpdateMode::Shift => sys::lv_chart_update_mode_t_LV_CHART_UPDATE_MODE_SHIFT,
                ChartUpdateMode::Circular => {
                    sys::lv_chart_update_mode_t_LV_CHART_UPDATE_MODE_CIRCULAR
                }
            },
        );
        for series in &spec.series {
            let existing = state
                .series
                .iter()
                .find(|(id, _)| *id == series.id)
                .map(|(_, p)| *p);
            let pointer = match existing {
                Some(p) => p,
                None => {
                    let axis = if series.x_axis == ChartAxis::Secondary {
                        sys::lv_chart_axis_t_LV_CHART_AXIS_SECONDARY_X
                    } else {
                        sys::lv_chart_axis_t_LV_CHART_AXIS_PRIMARY_X
                    } | if series.y_axis == ChartAxis::Secondary {
                        sys::lv_chart_axis_t_LV_CHART_AXIS_SECONDARY_Y
                    } else {
                        sys::lv_chart_axis_t_LV_CHART_AXIS_PRIMARY_Y
                    };
                    let p = NonNull::new(sys::lv_chart_add_series(
                        obj.as_ptr(),
                        series.color.to_raw(),
                        axis,
                    ))
                    .ok_or(UiError::Allocation)?;
                    state.series.push((series.id, p));
                    p
                }
            };
            sys::lv_chart_set_series_color(obj.as_ptr(), pointer.as_ptr(), series.color.to_raw());
            let previous = old.and_then(|s| s.series.iter().find(|s| s.id == series.id));
            let same_mode = old.map(|s| s.update_mode) == Some(spec.update_mode);
            let append = if !reset && existing.is_some() && same_mode {
                match (previous.map(|s| &s.data), &series.data) {
                    (Some(ChartData::History(old)), ChartData::History(new)) => {
                        new.appended_since(old)
                    }
                    _ => None,
                }
            } else {
                None
            };
            if let Some(samples) = append {
                for sample in samples {
                    let (x, y) = sample.pair();
                    if series.data.xy_kind() {
                        sys::lv_chart_set_next_value2(
                            obj.as_ptr(),
                            pointer.as_ptr(),
                            x.raw(),
                            y.raw(),
                        );
                    } else {
                        sys::lv_chart_set_next_value(obj.as_ptr(), pointer.as_ptr(), y.raw());
                    }
                }
                #[cfg(feature = "update-metrics")]
                crate::metrics::chart_append(samples.len());
            } else if reset
                || existing.is_none()
                || !same_mode
                || previous.map(|s| &s.data) != Some(&series.data)
            {
                write_full(obj, pointer, &series.data);
            }
        }
        state.draw.presentation = spec.presentation;
        state.draw.points = spec.points();
        for cursor in &spec.cursors {
            let dir = match cursor.direction {
                ChartCursorDirection::Horizontal => sys::lv_dir_t_LV_DIR_HOR,
                ChartCursorDirection::Vertical => sys::lv_dir_t_LV_DIR_VER,
                ChartCursorDirection::Both => sys::lv_dir_t_LV_DIR_ALL,
            };
            let pointer = NonNull::new(sys::lv_chart_add_cursor(
                obj.as_ptr(),
                cursor.color.to_raw(),
                dir as sys::lv_dir_t,
            ))
            .ok_or(UiError::Allocation)?;
            state.cursors.push((cursor.id, pointer));
            let position = match cursor.target {
                ChartCursorTarget::Position { x, y } => CursorPosition::Pixels(x, y),
                ChartCursorTarget::Point { series, index } => {
                    let series_index = spec
                        .series
                        .iter()
                        .position(|s| s.id == series)
                        .ok_or(UiError::InvalidProperty)?;
                    let series = &spec.series[series_index];
                    let (x, y) = series.data.sample(index as usize, spec.update_mode);
                    match y {
                        ChartValue::Missing => CursorPosition::Hidden,
                        ChartValue::Value(mut y) => {
                            if spec.presentation == ChartType::Stacked {
                                y = 0;
                                for s in &spec.series[..=series_index] {
                                    if let ChartValue::Value(v) =
                                        s.data.sample(index as usize, spec.update_mode).1
                                    {
                                        y += v;
                                    }
                                }
                            }
                            if spec.presentation == ChartType::Scatter && x == ChartValue::Missing {
                                CursorPosition::Hidden
                            } else {
                                CursorPosition::Sample {
                                    index: index as usize,
                                    series_index,
                                    series_count: spec.series.len(),
                                    x,
                                    y,
                                    x_range: spec.axes.x(series.x_axis),
                                    y_range: spec.axes.y(series.y_axis),
                                }
                            }
                        }
                    }
                }
            };
            state.draw.bindings.push(CursorBinding {
                cursor: pointer,
                position,
            });
        }
        resolve_positions(obj.as_ptr(), &state.draw);
        sys::lv_chart_refresh(obj.as_ptr());
    }
    Ok(())
}

/// Copy into native-owned arrays. No borrowed Rust storage is retained by C.
unsafe fn write_full(
    obj: NonNull<sys::lv_obj_t>,
    pointer: NonNull<sys::lv_chart_series_t>,
    data: &ChartData,
) {
    // SAFETY: caller owns a live series allocated for exactly `data.len()`
    // samples. Scatter creation checked both native arrays; static data has
    // already passed shape/sentinel validation. There are no active array borrows.
    unsafe {
        let ys = sys::lv_chart_get_series_y_array(obj.as_ptr(), pointer.as_ptr());
        let xs = if data.xy_kind() {
            sys::lv_chart_get_series_x_array(obj.as_ptr(), pointer.as_ptr())
        } else {
            core::ptr::null_mut()
        };
        for index in 0..data.len() {
            // Circular display order is exactly the physical array order;
            // Shift uses the same array and its separately restored ring start.
            let (x, y) = data.sample(index, ChartUpdateMode::Circular);
            ys.add(index).write(y.raw());
            if !xs.is_null() {
                xs.add(index).write(x.raw());
            }
        }
        let start = match data {
            ChartData::History(h) => (h.next_sequence() % data.len() as u64) as u32,
            _ => 0,
        };
        sys::lv_chart_set_x_start_point(obj.as_ptr(), pointer.as_ptr(), start);
        #[cfg(feature = "update-metrics")]
        crate::metrics::chart_full(data.len());
    }
}

fn map_value(value: i32, range: ChartRange, extent: i64) -> i64 {
    // Same endpoint clipping as LVGL, using i64 for the adapter's arithmetic.
    let value = value.clamp(range.min, range.max);
    (i64::from(value) - i64::from(range.min)) * extent
        / (i64::from(range.max) - i64::from(range.min))
}

unsafe extern "C" fn resolve_cursors(event: *mut sys::lv_event_t) {
    // SAFETY: registered on the live chart, with stable data retained through
    // native deletion. Only layout/style/scroll events update cursor positions;
    // native setters invalidate, so they must never run from a draw event.
    unsafe {
        let code = sys::lv_event_get_code(event);
        if code != sys::lv_event_code_t_LV_EVENT_SIZE_CHANGED
            && code != sys::lv_event_code_t_LV_EVENT_STYLE_CHANGED
            && code != sys::lv_event_code_t_LV_EVENT_SCROLL
        {
            return;
        }
        let ctx = sys::lv_event_get_user_data(event).cast::<DrawContext>();
        let obj = sys::lv_event_get_current_target(event).cast::<sys::lv_obj_t>();
        if let Some(ctx) = ctx.as_ref() {
            if !obj.is_null() {
                resolve_positions(obj, ctx);
            }
        }
    }
}

unsafe fn resolve_positions(obj: *mut sys::lv_obj_t, ctx: &DrawContext) {
    // SAFETY: caller guarantees a live chart/cursors and exclusive UI-thread
    // access. No references to native arrays are held. No user code, allocation,
    // checked-index panic or drawing occurs in this callback path.
    unsafe {
        let w = i64::from(sys::lv_obj_get_content_width(obj));
        let h = i64::from(sys::lv_obj_get_content_height(obj));
        let border = i64::from(sys::lv_obj_get_style_border_width(
            obj,
            sys::lv_part_t_LV_PART_MAIN,
        ));
        let x_offset = i64::from(sys::lv_obj_get_style_pad_left(
            obj,
            sys::lv_part_t_LV_PART_MAIN,
        )) + border
            - i64::from(sys::lv_obj_get_scroll_left(obj));
        let y_offset = i64::from(sys::lv_obj_get_style_pad_top(
            obj,
            sys::lv_part_t_LV_PART_MAIN,
        )) + border
            - i64::from(sys::lv_obj_get_scroll_top(obj));
        for binding in &ctx.bindings {
            let cursor = binding.cursor.as_ptr();
            let (x, y) = match binding.position {
                CursorPosition::Hidden => {
                    sys::lv_chart_set_cursor_point(
                        obj,
                        cursor,
                        core::ptr::null_mut(),
                        i32::MAX as u32,
                    );
                    continue;
                }
                CursorPosition::Pixels(x, y) => (i64::from(x), i64::from(y)),
                CursorPosition::Sample {
                    index,
                    series_index,
                    series_count,
                    x,
                    y,
                    x_range,
                    y_range,
                } => {
                    let x = match ctx.presentation {
                        ChartType::Scatter => match x {
                            ChartValue::Value(x) => map_value(x, x_range, w),
                            ChartValue::Missing => continue,
                        },
                        ChartType::Bar | ChartType::Stacked => {
                            let gap = i64::from(sys::lv_obj_get_style_pad_column(
                                obj,
                                sys::lv_part_t_LV_PART_MAIN,
                            ));
                            let points = ctx.points as i64;
                            let block = (w - (points - 1) * gap) / points;
                            let base = if points > 1 {
                                (w - block) * index as i64 / (points - 1)
                            } else {
                                0
                            };
                            if ctx.presentation == ChartType::Bar {
                                let series_gap = i64::from(sys::lv_obj_get_style_pad_column(
                                    obj,
                                    sys::lv_part_t_LV_PART_ITEMS,
                                ));
                                let column = ((block - (series_count as i64 - 1) * series_gap)
                                    / series_count as i64)
                                    .max(1);
                                base + series_index as i64 * (column + series_gap) + column / 2
                            } else {
                                base + block / 2
                            }
                        }
                        _ => {
                            if ctx.points > 1 {
                                w * index as i64 / (ctx.points as i64 - 1)
                            } else {
                                0
                            }
                        }
                    };
                    (x + x_offset, h - map_value(y, y_range, h) + y_offset)
                }
            };
            let mut position = sys::lv_point_t {
                x: x as i32,
                y: y as i32,
            };
            sys::lv_chart_set_cursor_pos(obj, cursor, &mut position);
        }
    }
}
