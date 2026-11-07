//! Native copied-storage, streaming order and cursor lifetime oracles.
mod common;

use core::ptr::NonNull;
use rs_lvgl_sys as sys;

struct Chart(NonNull<sys::lv_obj_t>);
impl Chart {
    fn new(points: u32, kind: sys::lv_chart_type_t) -> Self {
        // SAFETY: callers serialize LVGL and retain their memory display.
        let raw = NonNull::new(unsafe { sys::lv_chart_create(common::screen().as_ptr()) }).unwrap();
        // SAFETY: configure the new chart before any series allocations.
        unsafe {
            sys::lv_chart_set_type(raw.as_ptr(), kind);
            sys::lv_chart_set_point_count(raw.as_ptr(), points);
        }
        Self(raw)
    }

    fn series(&self, axis: sys::lv_chart_axis_t) -> NonNull<sys::lv_chart_series_t> {
        // SAFETY: the chart owns the returned series until explicit removal or
        // chart deletion. The copied arrays never refer to Rust storage.
        NonNull::new(unsafe {
            sys::lv_chart_add_series(
                self.0.as_ptr(),
                sys::lv_color_t {
                    red: 200,
                    green: 70,
                    blue: 30,
                },
                axis,
            )
        })
        .unwrap()
    }

    fn y(&self, series: NonNull<sys::lv_chart_series_t>) -> Vec<i32> {
        // SAFETY: the live series has the chart's current point count; copy the
        // values before any call that could reallocate/remove its native array.
        unsafe {
            let len = sys::lv_chart_get_point_count(self.0.as_ptr()) as usize;
            std::slice::from_raw_parts(
                sys::lv_chart_get_series_y_array(self.0.as_ptr(), series.as_ptr()),
                len,
            )
            .to_vec()
        }
    }
}

impl Drop for Chart {
    fn drop(&mut self) {
        // SAFETY: the display outlives the chart and native drawing is idle.
        // LVGL owns and releases all remaining series/cursors and point arrays.
        unsafe { sys::lv_obj_delete(self.0.as_ptr()) };
    }
}

#[test]
fn native_series_copy_arrays_and_keep_explicit_shift_or_circular_order() {
    let _guard = common::lock();
    let _display = common::init();
    let chart = Chart::new(3, sys::lv_chart_type_t_LV_CHART_TYPE_LINE);
    let series = chart.series(sys::lv_chart_axis_t_LV_CHART_AXIS_PRIMARY_Y);
    let mut values = vec![10, 20, 30];
    // SAFETY: native setters copy values into the chart-owned array.
    unsafe {
        sys::lv_chart_set_series_values(
            chart.0.as_ptr(),
            series.as_ptr(),
            values.as_ptr(),
            values.len(),
        )
    };
    values.fill(-1);
    drop(values);
    assert_eq!(chart.y(series), [10, 20, 30]);
    for mode in [
        sys::lv_chart_update_mode_t_LV_CHART_UPDATE_MODE_SHIFT,
        sys::lv_chart_update_mode_t_LV_CHART_UPDATE_MODE_CIRCULAR,
    ] {
        let mut reference = [10, 20, 30];
        let mut next = 0;
        // SAFETY: reset an existing series and its write position.
        unsafe {
            sys::lv_chart_set_update_mode(chart.0.as_ptr(), mode);
            sys::lv_chart_set_x_start_point(chart.0.as_ptr(), series.as_ptr(), 0);
            sys::lv_chart_set_series_values(
                chart.0.as_ptr(),
                series.as_ptr(),
                reference.as_ptr(),
                reference.len(),
            );
        }
        for value in [40, 50, 60, 70, 80] {
            reference[next] = value;
            next = (next + 1) % 3;
            // SAFETY: appending writes one copied sample to a valid series.
            unsafe { sys::lv_chart_set_next_value(chart.0.as_ptr(), series.as_ptr(), value) };
            assert_eq!(chart.y(series), reference);
            // SAFETY: synchronous getter, valid series.
            assert_eq!(
                unsafe { sys::lv_chart_get_x_start_point(chart.0.as_ptr(), series.as_ptr()) },
                next as u32
            );
            if mode == sys::lv_chart_update_mode_t_LV_CHART_UPDATE_MODE_SHIFT {
                let raw = chart.y(series);
                let ordered: Vec<_> = (0..3).map(|i| raw[(next + i) % 3]).collect();
                assert_eq!(ordered.last(), Some(&value));
            }
        }
    }
}

#[test]
fn native_scatter_copies_xy_and_cursor_coordinates_and_retires_cursor_first() {
    let _guard = common::lock();
    let _display = common::init();
    let chart = Chart::new(3, sys::lv_chart_type_t_LV_CHART_TYPE_SCATTER);
    let series = chart.series(
        sys::lv_chart_axis_t_LV_CHART_AXIS_SECONDARY_X
            | sys::lv_chart_axis_t_LV_CHART_AXIS_SECONDARY_Y,
    );
    let (x, y) = (vec![1, 3, 7], vec![11, 33, 77]);
    // SAFETY: chart has scatter X/Y storage of length three; copy both arrays.
    unsafe {
        sys::lv_chart_set_series_values2(
            chart.0.as_ptr(),
            series.as_ptr(),
            x.as_ptr(),
            y.as_ptr(),
            x.len(),
        )
    };
    drop((x, y));
    assert_eq!(chart.y(series), [11, 33, 77]);
    // SAFETY: live scatter series has three native X points.
    assert_eq!(
        unsafe {
            std::slice::from_raw_parts(
                sys::lv_chart_get_series_x_array(chart.0.as_ptr(), series.as_ptr()),
                3,
            )
        },
        [1, 3, 7]
    );
    // SAFETY: the chart owns the cursor and copies coordinate values.
    unsafe {
        let cursor = sys::lv_chart_add_cursor(
            chart.0.as_ptr(),
            sys::lv_color_t {
                red: 30,
                green: 160,
                blue: 80,
            },
            sys::lv_dir_t_LV_DIR_ALL,
        );
        assert!(!cursor.is_null());
        let mut pos = sys::lv_point_t { x: 12, y: 23 };
        sys::lv_chart_set_cursor_pos(chart.0.as_ptr(), cursor, &mut pos);
        pos.x = -1;
        assert_eq!(
            sys::lv_chart_get_cursor_point(chart.0.as_ptr(), cursor).x,
            12
        );
        assert_eq!(
            sys::lv_chart_get_cursor_point(chart.0.as_ptr(), cursor).y,
            23
        );
        sys::lv_chart_set_cursor_point(chart.0.as_ptr(), cursor, series.as_ptr(), 1);
        // A point cursor retains the series pointer; retire it before its series.
        sys::lv_chart_remove_cursor(chart.0.as_ptr(), cursor);
        sys::lv_chart_remove_series(chart.0.as_ptr(), series.as_ptr());
        assert!(sys::lv_chart_get_series_next(chart.0.as_ptr(), core::ptr::null()).is_null());
    }
    let replacement = chart.series(sys::lv_chart_axis_t_LV_CHART_AXIS_PRIMARY_Y);
    assert_eq!(
        chart.y(replacement),
        [i32::MAX; 3],
        "native missing-point sentinel"
    );
}

#[test]
fn native_resize_requires_full_snapshot_restore_after_growing() {
    let _guard = common::lock();
    let _display = common::init();
    let chart = Chart::new(3, sys::lv_chart_type_t_LV_CHART_TYPE_CURVE);
    let series = chart.series(sys::lv_chart_axis_t_LV_CHART_AXIS_PRIMARY_Y);
    let samples = [1, i32::MAX, 3];
    // SAFETY: three copied samples, including LVGL's explicit native sentinel.
    unsafe {
        sys::lv_chart_set_series_values(
            chart.0.as_ptr(),
            series.as_ptr(),
            samples.as_ptr(),
            samples.len(),
        )
    };
    assert_eq!(chart.y(series), samples);
    // SAFETY: resize while no cursor or borrowed slice refers to the old array.
    unsafe { sys::lv_chart_set_point_count(chart.0.as_ptr(), 5) };
    // The pinned provider's zero-start growth path initializes from
    // old_count - 1, so growth overwrites the last old value. This is a native
    // characterization, not an assumption that resizing preserves a snapshot.
    assert_eq!(chart.y(series), [1, i32::MAX, i32::MAX, i32::MAX, i32::MAX]);
    let snapshot = [1, i32::MAX, 3, 4, 5];
    // SAFETY: copied full synchronization restores every canonical sample;
    // native resizing alone must never be the declarative update oracle.
    unsafe {
        sys::lv_chart_set_x_start_point(chart.0.as_ptr(), series.as_ptr(), 0);
        sys::lv_chart_set_series_values(
            chart.0.as_ptr(),
            series.as_ptr(),
            snapshot.as_ptr(),
            snapshot.len(),
        );
    }
    assert_eq!(chart.y(series), snapshot);
    // SAFETY: getter only; the new curve native type is already selected.
    assert_eq!(
        unsafe { sys::lv_chart_get_type(chart.0.as_ptr()) },
        sys::lv_chart_type_t_LV_CHART_TYPE_CURVE
    );
}

use rs_lvgl_ui::prelude::*;

#[derive(Clone)]
struct ChartPage {
    spec: ChartSpec,
    size: (i16, i16),
    padding: i16,
}
impl App for ChartPage {
    type Action = ();
    fn update(&mut self, _: ()) {}
    fn view(&self, _: &Theme) -> View<()> {
        chart(self.spec.clone())
            .key(Key::Named("chart"))
            .width(px(self.size.0))
            .height(px(self.size.1))
            .padding(self.padding)
            .border(Color::rgb(0, 0, 0), 0)
            .radius(0)
            .background(Color::rgb(0, 0, 0))
    }
}
fn spec(kind: ChartType, values: Vec<ChartSeries>) -> ChartSpec {
    ChartSpec::new(
        kind,
        ChartAxes::new(ChartRange::new(0, 100), ChartRange::new(0, 100)),
        values,
    )
}
fn series(id: u64, values: &[i32]) -> ChartSeries {
    ChartSeries::new(
        id,
        Color::rgb((id * 31) as u8, 180, 70),
        ChartData::values(values.iter().copied()),
    )
}
fn native_series(chart: NonNull<sys::lv_obj_t>) -> Vec<NonNull<sys::lv_chart_series_t>> {
    let mut out = Vec::new();
    // SAFETY: live mounted chart; pointers are copied and never used after an
    // update that removes/resizes the corresponding series.
    unsafe {
        let mut p = sys::lv_chart_get_series_next(chart.as_ptr(), core::ptr::null());
        while let Some(pointer) = NonNull::new(p) {
            out.push(pointer);
            p = sys::lv_chart_get_series_next(chart.as_ptr(), p);
        }
    }
    out
}
fn native_samples(
    chart: NonNull<sys::lv_obj_t>,
    pointer: NonNull<sys::lv_chart_series_t>,
    x: bool,
) -> Vec<i32> {
    // SAFETY: caller supplies a live series in this chart. Copy before mutation.
    unsafe {
        let len = sys::lv_chart_get_point_count(chart.as_ptr()) as usize;
        let array = if x {
            sys::lv_chart_get_series_x_array(chart.as_ptr(), pointer.as_ptr())
        } else {
            sys::lv_chart_get_series_y_array(chart.as_ptr(), pointer.as_ptr())
        };
        std::slice::from_raw_parts(array, len).to_vec()
    }
}

#[test]
fn descriptors_patch_compatible_series_and_rebuild_ordered_membership() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = ChartPage {
        spec: spec(
            ChartType::Line,
            vec![series(1, &[1, 2, 3]), series(2, &[4, 5, 6])],
        ),
        size: (60, 40),
        padding: 0,
    };
    ui.mount(&app).unwrap();
    let chart = common::app_root();
    let pointers = native_series(chart);
    app.spec.series[0].data = ChartData::values([3, 2, 1]);
    app.spec.series[0].color = Color::rgb(201, 33, 77);
    ui.update(&app).unwrap();
    assert_eq!(native_series(chart), pointers);
    assert_eq!(native_samples(chart, pointers[0], false), [3, 2, 1]);
    // Independent order oracle: each series has distinct values and color.
    app.spec.series.swap(0, 1);
    ui.update(&app).unwrap();
    let reordered = native_series(chart);
    assert_eq!(native_samples(chart, reordered[0], false), [4, 5, 6]);
    assert_eq!(native_samples(chart, reordered[1], false), [3, 2, 1]);
    app.spec.series.truncate(1);
    ui.update(&app).unwrap();
    assert_eq!(native_series(chart), vec![reordered[0]]);
    app.spec.series.push(series(9, &[11, 12, 13]));
    ui.update(&app).unwrap();
    assert_eq!(native_series(chart)[0], reordered[0]);
    app.spec.series.clear();
    ui.update(&app).unwrap();
    assert!(native_series(chart).is_empty());
    assert_eq!(common::app_root(), chart);
    ui.unmount();
    ui.mount(&app).unwrap();
}

#[test]
fn scatter_axes_and_type_count_changes_use_owned_reconstructible_arrays() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = ChartPage {
        spec: spec(
            ChartType::Scatter,
            vec![ChartSeries::new(
                1,
                Color::rgb(200, 80, 30),
                ChartData::xy([(100, 1000), (150, 1500), (200, 2000)]),
            )
            .axes(ChartAxis::Secondary, ChartAxis::Secondary)],
        ),
        size: (60, 40),
        padding: 0,
    };
    app.spec.axes.secondary_x = ChartRange::new(100, 200);
    app.spec.axes.secondary_y = ChartRange::new(1000, 2000);
    ui.mount(&app).unwrap();
    let chart = common::app_root();
    let pointer = native_series(chart)[0];
    assert_eq!(native_samples(chart, pointer, true), [100, 150, 200]);
    assert_eq!(native_samples(chart, pointer, false), [1000, 1500, 2000]);
    // Native coordinate lookup independently proves both secondary assignments.
    unsafe {
        sys::lv_obj_update_layout(chart.as_ptr());
        let mut p = sys::lv_point_t { x: 0, y: 0 };
        sys::lv_chart_get_point_pos_by_id(chart.as_ptr(), pointer.as_ptr(), 1, &mut p);
        assert_eq!((p.x, p.y), (30, 20));
    }
    app.spec = spec(ChartType::Bar, vec![series(1, &[-10, 20, 110, 40])]);
    ui.update(&app).unwrap();
    assert_eq!(
        native_samples(chart, native_series(chart)[0], false),
        [-10, 20, 110, 40],
        "out-of-range representable samples stay canonical"
    );
    app.spec = spec(
        ChartType::Line,
        vec![ChartSeries::new(
            1,
            Color::rgb(200, 80, 30),
            ChartData::Y(vec![
                ChartValue::Value(-200),
                ChartValue::Missing,
                ChartValue::Value(300),
            ]),
        )],
    );
    ui.update(&app).unwrap();
    assert_eq!(
        native_samples(chart, native_series(chart)[0], false),
        [-200, i32::MAX, 300]
    );
    drop(ui.snapshot().unwrap());
}

#[test]
fn invalid_chart_shapes_identifiers_and_native_encodings_preserve_mounted_tree() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let good = spec(ChartType::Line, vec![series(1, &[1, 2, 3])]);
    let mut app = ChartPage {
        spec: good.clone(),
        size: (60, 40),
        padding: 0,
    };
    ui.mount(&app).unwrap();
    let chart = common::app_root();
    let original = native_series(chart)[0];
    let mut invalid = Vec::new();
    let mut s = good.clone();
    s.axes.primary_y = ChartRange::new(10, 10);
    invalid.push(s);
    let mut s = good.clone();
    s.axes.primary_y = ChartRange::new(i32::MIN, i32::MAX);
    invalid.push(s);
    let mut s = good.clone();
    s.series.push(s.series[0].clone());
    invalid.push(s);
    invalid.push(spec(
        ChartType::Line,
        vec![series(1, &[1, 2]), series(2, &[1])],
    ));
    invalid.push(spec(ChartType::Line, vec![series(1, &[i32::MAX])]));
    invalid.push(spec(
        ChartType::Scatter,
        vec![ChartSeries::new(
            1,
            Color::rgb(0, 0, 0),
            ChartData::XY {
                x: vec![ChartValue::Value(1)],
                y: vec![],
            },
        )],
    ));
    invalid.push(spec(ChartType::Scatter, vec![series(1, &[1])]));
    invalid.push(spec(ChartType::Stacked, vec![series(1, &[-1])]));
    invalid.push(spec(
        ChartType::Stacked,
        vec![series(1, &[i32::MAX - 1]), series(2, &[3])],
    ));
    invalid.push(spec(
        ChartType::Bar,
        vec![ChartSeries::new(
            1,
            Color::rgb(0, 0, 0),
            ChartData::Y(vec![ChartValue::Missing]),
        )],
    ));
    let mut s = good.clone();
    s.cursors.push(ChartCursor::new(
        1,
        Color::rgb(1, 1, 1),
        ChartCursorDirection::Both,
        ChartCursorTarget::Point {
            series: 9,
            index: 0,
        },
    ));
    invalid.push(s);
    let mut s = good.clone();
    let c = ChartCursor::new(
        1,
        Color::rgb(1, 1, 1),
        ChartCursorDirection::Both,
        ChartCursorTarget::Point {
            series: 1,
            index: 0,
        },
    );
    s.cursors = vec![c.clone(), c];
    invalid.push(s);
    for bad in invalid {
        app.spec = bad;
        assert_eq!(ui.update(&app), Err(UiError::InvalidProperty));
        assert_eq!(common::app_root(), chart);
        assert_eq!(native_series(chart), vec![original]);
        assert_eq!(native_samples(chart, original, false), [1, 2, 3]);
    }
    app.spec = good;
    ui.update(&app).unwrap();
}

#[test]
fn all_supported_presentations_and_cursors_render_real_native_pixels() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    for kind in [
        ChartType::Line,
        ChartType::Bar,
        ChartType::Stacked,
        ChartType::Curve,
        ChartType::Scatter,
    ] {
        let data = if kind == ChartType::Scatter {
            ChartData::xy([(10, 20), (50, 80), (90, 30)])
        } else {
            ChartData::values([20, 80, 30])
        };
        let mut app = ChartPage {
            spec: spec(
                kind,
                vec![ChartSeries::new(1, Color::rgb(240, 80, 40), data)],
            ),
            size: (60, 40),
            padding: 2,
        };
        app.spec.cursors.push(ChartCursor::new(
            1,
            Color::rgb(30, 240, 80),
            ChartCursorDirection::Both,
            ChartCursorTarget::Point {
                series: 1,
                index: 1,
            },
        ));
        if kind == ChartType::Curve && sys::LV_USE_VECTOR_GRAPHIC == 0 {
            assert_eq!(ui.mount(&app), Err(UiError::InvalidProperty));
            continue;
        }
        ui.mount(&app).unwrap();
        let image = ui.snapshot().unwrap();
        let series_pixels = image
            .pixels
            .chunks_exact(4)
            .filter(|p| p[0] > 180 && p[1] < 130 && p[2] < 100)
            .count();
        let cursor_pixels = image
            .pixels
            .chunks_exact(4)
            .filter(|p| p[1] > 180 && p[0] < 100 && p[2] < 130)
            .count();
        assert!(series_pixels > 4, "{kind:?}: visible data");
        assert!(cursor_pixels > 20, "{kind:?}: visible cursor");
        app.size = (50, 30);
        app.padding = 3;
        ui.update(&app).unwrap();
        let image = ui.snapshot().unwrap();
        assert_eq!((image.width, image.height), (50, 30));
        assert!(
            image
                .pixels
                .chunks_exact(4)
                .filter(|p| p[1] > 180 && p[0] < 100 && p[2] < 130)
                .count()
                > 15,
            "cursor follows final layout"
        );
        app.spec.cursors.clear();
        app.spec.series.clear();
        ui.update(&app).unwrap();
        drop(ui.snapshot().unwrap());
        ui.unmount();
    }
}

use core::num::NonZeroU32;
fn capacity(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
}

#[test]
fn history_is_bounded_immutable_and_checks_kind_epoch_and_sequence_without_mutation() {
    let mut h = ChartHistory::new(9, capacity(3), ChartSampleKind::Y);
    for value in 0..5 {
        assert_eq!(h.push_y(value), Ok(value as u64));
    }
    assert_eq!(h.len(), 3);
    let snapshot = h.snapshot();
    assert_eq!(
        (snapshot.first_sequence(), snapshot.next_sequence()),
        (2, 5)
    );
    assert_eq!(
        snapshot.samples(),
        [
            ChartSample::Y(ChartValue::Value(2)),
            ChartSample::Y(ChartValue::Value(3)),
            ChartSample::Y(ChartValue::Value(4))
        ]
    );
    assert_eq!(h.push_xy(1, 2), Err(ChartHistoryError::WrongSampleKind));
    assert_eq!(h.push_y(i32::MAX), Err(ChartHistoryError::ReservedValue));
    assert_eq!(h.clear(9), Err(ChartHistoryError::EpochUnchanged));
    assert_eq!(h.snapshot(), snapshot);
    h.push_y(5).unwrap();
    assert_eq!(snapshot.first_sequence(), 2, "copied immutable snapshot");
    h.set_capacity(capacity(1));
    assert_eq!(h.len(), 1);
    assert_eq!(h.snapshot().first_sequence(), 5);
    h.clear(10).unwrap();
    assert!(h.is_empty());
    assert_eq!(h.next_sequence(), 0);
    let mut last = ChartHistory::starting_at(99, capacity(2), ChartSampleKind::Y, u64::MAX - 1);
    assert_eq!(last.push_y(7), Ok(u64::MAX - 1));
    let snapshot = last.snapshot();
    assert_eq!(last.push_y(8), Err(ChartHistoryError::SequenceExhausted));
    assert_eq!(last.snapshot(), snapshot);
}

fn assert_history_native(chart: NonNull<sys::lv_obj_t>, snapshot: &ChartHistorySnapshot) {
    let pointer = native_series(chart)[0];
    let capacity = snapshot.capacity().get() as usize;
    let mut expected_y = vec![i32::MAX; capacity];
    let mut expected_x = vec![i32::MAX; capacity];
    // This oracle uses canonical sequence modulo capacity directly, independent
    // of the adapter's display/continuity helpers and native append operations.
    for (offset, sample) in snapshot.samples().iter().enumerate() {
        let slot = ((snapshot.first_sequence() + offset as u64) % capacity as u64) as usize;
        let raw = |value: ChartValue| match value {
            ChartValue::Value(v) => v,
            ChartValue::Missing => i32::MAX,
            _ => unreachable!(),
        };
        match *sample {
            ChartSample::Y(y) => expected_y[slot] = raw(y),
            ChartSample::XY { x, y } => {
                expected_x[slot] = raw(x);
                expected_y[slot] = raw(y);
            }
            _ => unreachable!(),
        }
    }
    assert_eq!(native_samples(chart, pointer, false), expected_y);
    if snapshot.kind() == ChartSampleKind::XY {
        assert_eq!(native_samples(chart, pointer, true), expected_x);
    }
    // SAFETY: live chart/series; no update occurs while reading ring position.
    assert_eq!(
        unsafe { sys::lv_chart_get_x_start_point(chart.as_ptr(), pointer.as_ptr()) },
        (snapshot.next_sequence() % capacity as u64) as u32
    );
}

#[test]
fn history_append_repeat_skip_wrap_capacity_mode_clear_and_remount_match_full_reference() {
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    for mode in [ChartUpdateMode::Shift, ChartUpdateMode::Circular] {
        for kind in [ChartSampleKind::Y, ChartSampleKind::XY] {
            for count in [1, 4, 7] {
                let mut history = ChartHistory::starting_at(11, capacity(count), kind, 100);
                let make_spec = |h: &ChartHistory| {
                    let mut s = spec(
                        if kind == ChartSampleKind::XY {
                            ChartType::Scatter
                        } else {
                            ChartType::Line
                        },
                        vec![ChartSeries::new(
                            1,
                            Color::rgb(230, 90, 30),
                            ChartData::History(h.snapshot()),
                        )],
                    );
                    s.update_mode = mode;
                    s
                };
                let mut app = ChartPage {
                    spec: make_spec(&history),
                    size: (60, 40),
                    padding: 2,
                };
                ui.mount(&app).unwrap();
                let chart = common::app_root();
                for batch in [1, 2, 2, 4, 9] {
                    for _ in 0..batch {
                        let value = (history.next_sequence() % 100) as i32;
                        if kind == ChartSampleKind::XY {
                            history.push_xy(value, value * 2).unwrap();
                        } else {
                            history.push_y(value).unwrap();
                        }
                    }
                    app.spec = make_spec(&history);
                    ui.update(&app).unwrap();
                    assert_history_native(chart, &history.snapshot());
                    let pointer = native_series(chart)[0];
                    ui.update(&app).unwrap();
                    assert_history_native(chart, &history.snapshot());
                    assert_eq!(native_series(chart)[0], pointer);
                    drop(ui.snapshot().unwrap());
                }
                history.set_capacity(capacity(count + 2));
                app.spec = make_spec(&history);
                ui.update(&app).unwrap();
                assert_history_native(chart, &history.snapshot());
                app.spec.update_mode = if mode == ChartUpdateMode::Shift {
                    ChartUpdateMode::Circular
                } else {
                    ChartUpdateMode::Shift
                };
                ui.update(&app).unwrap();
                assert_history_native(chart, &history.snapshot());
                history.clear(12).unwrap();
                app.spec = make_spec(&history);
                ui.update(&app).unwrap();
                assert_history_native(chart, &history.snapshot());
                ui.unmount();
                ui.mount(&app).unwrap();
                assert_history_native(common::app_root(), &history.snapshot());
                ui.unmount();
            }
        }
    }
}

#[cfg(feature = "update-metrics")]
#[test]
fn native_write_metrics_distinguish_unseen_append_full_recovery_and_repeat() {
    use rs_lvgl_ui::update_metrics as metrics;
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut history = ChartHistory::new(1, capacity(8), ChartSampleKind::Y);
    let make_spec = |h: &ChartHistory| {
        spec(
            ChartType::Line,
            vec![ChartSeries::new(
                1,
                Color::rgb(230, 90, 30),
                ChartData::History(h.snapshot()),
            )],
        )
    };
    let mut app = ChartPage {
        spec: make_spec(&history),
        size: (60, 40),
        padding: 0,
    };
    let before = metrics();
    ui.mount(&app).unwrap();
    let mounted = metrics();
    assert_eq!(
        mounted.native_chart_full_sample_writes - before.native_chart_full_sample_writes,
        8
    );
    history.push_y(10).unwrap();
    app.spec = make_spec(&history);
    ui.update(&app).unwrap();
    let appended = metrics();
    assert_eq!(
        appended.native_chart_sample_appends - mounted.native_chart_sample_appends,
        1
    );
    assert_eq!(
        appended.native_chart_full_syncs,
        mounted.native_chart_full_syncs
    );
    ui.update(&app).unwrap();
    assert_eq!(metrics(), appended);
    for value in 11..15 {
        history.push_y(value).unwrap();
    }
    app.spec = make_spec(&history);
    ui.update(&app).unwrap();
    let batch = metrics();
    assert_eq!(
        batch.native_chart_sample_appends - appended.native_chart_sample_appends,
        4
    );
    // An empty history at sequence 100 cannot append a replacement that also
    // introduces earlier samples; the missing prefix requires full sync.
    let empty = ChartHistory::starting_at(9, capacity(8), ChartSampleKind::Y, 100);
    app.spec = make_spec(&empty);
    ui.update(&app).unwrap();
    let before_prefix = metrics();
    let mut prefix = ChartHistory::starting_at(9, capacity(8), ChartSampleKind::Y, 98);
    for value in 0..4 {
        prefix.push_y(value).unwrap();
    }
    app.spec = make_spec(&prefix);
    ui.update(&app).unwrap();
    assert_eq!(
        metrics().native_chart_full_syncs - before_prefix.native_chart_full_syncs,
        1
    );
    assert_history_native(common::app_root(), &prefix.snapshot());
    // Restore the original source before the changed-overlap control.
    app.spec = make_spec(&history);
    ui.update(&app).unwrap();
    let batch = metrics();
    // A same-epoch replacement with changed overlap must full-sync.
    let mut branch = ChartHistory::new(1, capacity(8), ChartSampleKind::Y);
    for value in 30..35 {
        branch.push_y(value).unwrap();
    }
    app.spec = make_spec(&branch);
    ui.update(&app).unwrap();
    let replaced = metrics();
    assert_eq!(
        replaced.native_chart_full_syncs - batch.native_chart_full_syncs,
        1
    );
    for value in 40..60 {
        branch.push_y(value).unwrap();
    }
    app.spec = make_spec(&branch);
    ui.update(&app).unwrap();
    let skipped = metrics();
    assert_eq!(
        skipped.native_chart_full_syncs - replaced.native_chart_full_syncs,
        1
    );
    assert_eq!(
        skipped.native_chart_sample_appends,
        replaced.native_chart_sample_appends
    );
    assert_history_native(common::app_root(), &branch.snapshot());
}

#[test]
fn history_later_sibling_failure_retires_progress_and_retry_reconstructs_canonical_state() {
    struct Page {
        history: ChartHistory,
        fail: bool,
    }
    impl App for Page {
        type Action = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            column([
                chart(spec(
                    ChartType::Line,
                    vec![ChartSeries::new(
                        1,
                        Color::rgb(200, 80, 30),
                        ChartData::History(self.history.snapshot()),
                    )],
                ))
                .key(Key::Id(1)),
                if self.fail {
                    // SAFETY: deliberate native allocation failure creates no
                    // objects, retains no pointers and has no callbacks.
                    unsafe { raw(|_| None) }
                } else {
                    label("valid")
                }
                .key(Key::Id(2)),
            ])
        }
    }
    let _guard = common::lock();
    let display = common::init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let mut app = Page {
        history: ChartHistory::new(1, capacity(4), ChartSampleKind::Y),
        fail: false,
    };
    app.history.push_y(10).unwrap();
    ui.mount(&app).unwrap();
    app.history.push_y(20).unwrap();
    app.fail = true;
    assert_eq!(ui.update(&app), Err(UiError::Allocation));
    assert_eq!(ui.stats().mounted_nodes, 0);
    app.fail = false;
    ui.mount(&app).unwrap();
    assert_history_native(
        common::child(common::app_root(), 0),
        &app.history.snapshot(),
    );
}
