use rs_lvgl::{ColorFormat, DisplayBuffers, DisplayBuilder, PixelBuffer, RenderMode};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

fn reset() {
    unsafe { rs_lvgl::deinit() };
    rs_lvgl::init();
}

fn buffer(bytes: usize) -> PixelBuffer {
    #[cfg(feature = "alloc")]
    {
        PixelBuffer::Owned(vec![0; bytes].into_boxed_slice())
    }
    #[cfg(not(feature = "alloc"))]
    {
        PixelBuffer::Static(Box::leak(vec![0; bytes].into_boxed_slice()))
    }
}

unsafe fn paint(display: &rs_lvgl::Display, r: u8, g: u8, b: u8) {
    let raw = display.raw().as_ptr();
    let screen = rs_lvgl::sys::lv_display_get_screen_active(raw);
    rs_lvgl::sys::lv_obj_set_style_bg_color(screen, rs_lvgl::sys::lv_color_make(r, g, b), 0);
    rs_lvgl::sys::lv_obj_set_style_bg_opa(screen, 255, 0);
    rs_lvgl::sys::lv_refr_now(raw);
}

unsafe fn black_rectangle(
    display: &rs_lvgl::Display,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> *mut rs_lvgl::sys::lv_obj_t {
    let screen = rs_lvgl::sys::lv_display_get_screen_active(display.raw().as_ptr());
    let object = rs_lvgl::sys::lv_obj_create(screen);
    rs_lvgl::sys::lv_obj_remove_style_all(object);
    rs_lvgl::sys::lv_obj_set_pos(object, x, y);
    rs_lvgl::sys::lv_obj_set_size(object, width, height);
    rs_lvgl::sys::lv_obj_set_style_bg_color(object, rs_lvgl::sys::lv_color_make(0, 0, 0), 0);
    rs_lvgl::sys::lv_obj_set_style_bg_opa(object, 255, 0);
    object
}

#[test]
fn rgb888_display_uses_explicit_padded_rows() {
    // This integration binary runs native tests serially.
    unsafe { rs_lvgl::deinit() };
    rs_lvgl::init();
    let buffer = Box::leak(vec![0; 32 * 4].into_boxed_slice());
    let display = DisplayBuilder::new(7, 4)
        .color_format(ColorFormat::Rgb888)
        .render_mode(RenderMode::Direct)
        .stride(32)
        .buffers(DisplayBuffers::Single(PixelBuffer::Static(buffer)))
        .build(|info| {
            assert_eq!(info.stride(), 32);
            assert_eq!(info.format(), ColorFormat::Rgb888);
            assert_eq!(info.origin(), (0, 0));
        })
        .unwrap();
    unsafe {
        let raw = rs_lvgl::sys::lv_display_get_default();
        assert_eq!(
            rs_lvgl::sys::lv_display_get_color_format(raw),
            rs_lvgl::sys::lv_color_format_t_LV_COLOR_FORMAT_RGB888
        );
        rs_lvgl::sys::lv_refr_now(raw);
        display.delete();
    }
}

#[test]
fn native_formats_and_all_modes_preserve_pixel_bytes() {
    for double_buffered in [false, true] {
        for mode in [RenderMode::Partial, RenderMode::Direct, RenderMode::Full] {
            for (format, pixel) in [
                (ColorFormat::Rgb888, &[0, 0, 255][..]),
                (ColorFormat::Rgb565, &[0, 248][..]),
                (ColorFormat::Rgb565Swapped, &[248, 0][..]),
            ] {
                reset();
                let count = Rc::new(Cell::new(0));
                let calls = count.clone();
                let buffers = if double_buffered {
                    DisplayBuffers::Double {
                        front: buffer(32 * 5),
                        back: buffer(32 * 5 + 7),
                    }
                } else {
                    DisplayBuffers::Single(buffer(32 * 5))
                };
                let display = DisplayBuilder::new(7, 5)
                    .color_format(format)
                    .render_mode(mode)
                    .stride(32)
                    .buffers(buffers)
                    .build(move |info| {
                        assert_eq!(info.format(), format);
                        // No native reentry/transfer can occur while these rows are borrowed.
                        for row in unsafe { info.rows() } {
                            assert!(row.chunks_exact(pixel.len()).all(|p| p == pixel));
                            calls.set(calls.get() + row.len() / pixel.len());
                        }
                        if mode == RenderMode::Partial {
                            assert_eq!(info.origin(), (info.area.x1, info.area.y1));
                        } else {
                            assert_eq!(info.stride(), 32);
                        }
                    })
                    .unwrap();
                unsafe {
                    paint(&display, 255, 0, 0);
                }
                assert_eq!(count.get(), 35, "format={format:?}, mode={mode:?}");
                unsafe {
                    display.delete();
                }
            }
        }
    }
}

#[test]
fn direct_dirty_rectangle_uses_full_buffer_origin_and_padding() {
    reset();
    let observed = Rc::new(RefCell::new(Vec::new()));
    let capture = observed.clone();
    let display = DisplayBuilder::new(9, 7)
        .render_mode(RenderMode::Direct)
        .stride(32)
        .buffers(DisplayBuffers::Single(buffer(32 * 7)))
        .build(move |info| {
            let rows: Vec<Vec<u8>> = unsafe { info.rows() }.map(<[u8]>::to_vec).collect();
            capture
                .borrow_mut()
                .push((info.area, info.origin(), info.stride(), rows));
        })
        .unwrap();
    unsafe {
        paint(&display, 255, 0, 0);
        observed.borrow_mut().clear();
        let area = rs_lvgl::sys::lv_area_t {
            x1: 2,
            y1: 3,
            x2: 4,
            y2: 4,
        };
        rs_lvgl::sys::lv_obj_invalidate_area(
            rs_lvgl::sys::lv_display_get_screen_active(display.raw().as_ptr()),
            &area,
        );
        rs_lvgl::sys::lv_refr_now(display.raw().as_ptr());
    }
    let samples = observed.borrow();
    assert_eq!(samples.len(), 1);
    let (area, origin, stride, rows) = &samples[0];
    assert_eq!((area.x1, area.y1, area.x2, area.y2), (2, 3, 4, 4));
    assert_eq!(*origin, (0, 0));
    assert_eq!(*stride, 32);
    assert_eq!(rows, &vec![vec![0, 0, 255, 0, 0, 255, 0, 0, 255]; 2]);
    drop(samples);
    unsafe {
        display.delete();
    }
}

#[test]
fn partial_dirty_regions_use_their_actual_stride_when_copied_to_a_shadow() {
    reset();
    let observed = Rc::new(RefCell::new(Vec::new()));
    let capture = observed.clone();
    let shadow = Rc::new(RefCell::new(vec![0xA5; 40 * 9]));
    let destination = shadow.clone();
    let display = DisplayBuilder::new(11, 9)
        .stride(40)
        .buffers(DisplayBuffers::Single(buffer(120)))
        .build(move |info| {
            // Native reads finish in this synchronous callback. The shadow is
            // a separate allocation; no row borrow survives flush-ready.
            let rows: Vec<Vec<u8>> = unsafe { info.rows() }.map(<[u8]>::to_vec).collect();
            for (row, bytes) in rows.iter().enumerate() {
                let start = (info.area.y1 as usize + row) * 40 + info.area.x1 as usize * 3;
                destination.borrow_mut()[start..start + bytes.len()].copy_from_slice(bytes);
            }
            capture.borrow_mut().push((
                info.area,
                info.origin(),
                info.stride(),
                info.is_last(),
                rows,
            ));
        })
        .unwrap();
    unsafe {
        let left = black_rectangle(&display, 1, 1, 3, 2);
        let right = black_rectangle(&display, 8, 6, 1, 2);
        paint(&display, 0, 0, 0);
        observed.borrow_mut().clear();
        rs_lvgl::sys::lv_obj_set_style_bg_color(left, rs_lvgl::sys::lv_color_make(255, 0, 0), 0);
        rs_lvgl::sys::lv_obj_set_style_bg_color(right, rs_lvgl::sys::lv_color_make(0, 255, 0), 0);
        rs_lvgl::sys::lv_refr_now(display.raw().as_ptr());
    }
    let samples = observed.borrow();
    assert_eq!(samples.len(), 2);
    assert!(!samples[0].3);
    assert!(samples[1].3);
    for (area, origin, stride, _, rows) in samples.iter() {
        let (expected_area, pixel) = if area.x1 == 1 {
            ((1, 1, 3, 2), [0, 0, 255])
        } else {
            ((8, 6, 8, 7), [0, 255, 0])
        };
        assert_eq!((area.x1, area.y1, area.x2, area.y2), expected_area);
        assert_eq!(*origin, (area.x1, area.y1));
        let width = (area.x2 - area.x1 + 1) as u32;
        let native_stride = unsafe {
            rs_lvgl::sys::lv_draw_buf_width_to_stride(width, ColorFormat::Rgb888.to_raw())
        };
        assert_eq!(*stride, native_stride as usize);
        assert_ne!(*stride, 40);
        assert_eq!(rows.len(), 2);
        for row in rows {
            assert_eq!(row.len(), width as usize * 3);
            assert!(row.chunks_exact(3).all(|bytes| bytes == pixel));
        }
    }
    let pixels = shadow.borrow();
    for y in 0..9 {
        for x in 0..11 {
            let expected = if (1..=3).contains(&x) && (1..=2).contains(&y) {
                [0, 0, 255]
            } else if x == 8 && (6..=7).contains(&y) {
                [0, 255, 0]
            } else {
                [0, 0, 0]
            };
            assert_eq!(&pixels[y * 40 + x * 3..y * 40 + x * 3 + 3], &expected);
        }
        assert_eq!(&pixels[y * 40 + 33..(y + 1) * 40], &[0xA5; 7]);
    }
    drop((pixels, samples));
    unsafe { display.delete() };
}

#[test]
fn direct_double_buffers_sync_pixels_and_lease_only_the_pending_region() {
    struct PendingRegion {
        data: usize,
        rows: Vec<(usize, Vec<u8>)>,
        full: Vec<u8>,
        last: bool,
    }
    unsafe fn complete(lease: PendingRegion) -> bool {
        for (offset, expected) in &lease.rows {
            let row =
                core::slice::from_raw_parts((lease.data + offset) as *const u8, expected.len());
            assert_eq!(row, expected);
        }
        let full = core::slice::from_raw_parts(lease.data as *const u8, lease.full.len());
        if lease.last {
            assert_eq!(full, lease.full);
        }
        full != lease.full
    }

    reset();
    let pending = Rc::new(RefCell::new(None::<PendingRegion>));
    let submit = pending.clone();
    let retire = pending.clone();
    let observed = Rc::new(RefCell::new(Vec::new()));
    let capture = observed.clone();
    let frames = Rc::new(RefCell::new(Vec::new()));
    let final_regions = frames.clone();
    let waits = Rc::new(Cell::new(0));
    let completed = waits.clone();
    let other_regions_changed = Rc::new(Cell::new(0));
    let changes = other_regions_changed.clone();
    // The simulated transfer retains raw addresses and owned byte snapshots,
    // never a Rust borrow while native rendering resumes. Only submitted rows
    // are leased before the last area; DIRECT can draw another area meanwhile.
    let mut display = unsafe {
        DisplayBuilder::new(11, 9)
            .render_mode(RenderMode::Direct)
            .invalidation_alignment(4, 3)
            .stride(40)
            .buffers(DisplayBuffers::Double {
                front: buffer(360),
                back: buffer(360),
            })
            .build_async(
                move |info| {
                    assert!(submit.borrow().is_none());
                    assert_eq!(info.origin(), (0, 0));
                    assert_eq!(info.stride(), 40);
                    let x = info.area.x1 as usize;
                    let y = info.area.y1 as usize;
                    let rows = info
                        .rows()
                        .enumerate()
                        .map(|(row, bytes)| ((y + row) * 40 + x * 3, bytes.to_vec()))
                        .collect();
                    let full = info.buffer().as_bytes().to_vec();
                    let data = info.data().as_ptr() as usize;
                    capture.borrow_mut().push((data, info.area, info.is_last()));
                    if info.is_last() {
                        final_regions.borrow_mut().push(full.clone());
                    }
                    *submit.borrow_mut() = Some(PendingRegion {
                        data,
                        rows,
                        full,
                        last: info.is_last(),
                    });
                },
                move || {
                    let lease = retire.borrow_mut().take().expect("one submitted region");
                    if complete(lease) {
                        changes.set(changes.get() + 1);
                    }
                    completed.set(completed.get() + 1);
                },
            )
            .unwrap()
    };
    unsafe {
        let left = black_rectangle(&display, 1, 1, 3, 2);
        let right = black_rectangle(&display, 8, 6, 1, 2);
        paint(&display, 0, 0, 0);
        rs_lvgl::sys::lv_obj_set_style_bg_color(left, rs_lvgl::sys::lv_color_make(255, 0, 0), 0);
        rs_lvgl::sys::lv_refr_now(display.raw().as_ptr());
        rs_lvgl::sys::lv_obj_set_style_bg_color(right, rs_lvgl::sys::lv_color_make(0, 0, 255), 0);
        rs_lvgl::sys::lv_refr_now(display.raw().as_ptr());
        rs_lvgl::sys::lv_obj_set_style_bg_color(left, rs_lvgl::sys::lv_color_make(0, 255, 0), 0);
        rs_lvgl::sys::lv_obj_set_style_bg_color(right, rs_lvgl::sys::lv_color_make(255, 0, 0), 0);
        rs_lvgl::sys::lv_refr_now(display.raw().as_ptr());
    }
    let samples = observed.borrow();
    assert_eq!(samples.len(), 5);
    let addresses: Vec<_> = samples
        .iter()
        .filter(|sample| sample.2)
        .map(|sample| sample.0)
        .collect();
    assert_eq!(addresses.len(), 4);
    assert_eq!(addresses[0], addresses[2]);
    assert_eq!(addresses[1], addresses[3]);
    assert_ne!(addresses[0], addresses[1]);
    assert_eq!(samples[3].0, samples[4].0);
    assert!(!samples[3].2);
    assert!(samples[4].2);
    assert_eq!(
        samples[1].1,
        rs_lvgl::Area {
            x1: 0,
            y1: 0,
            x2: 3,
            y2: 2
        }
    );
    assert_eq!(
        samples[2].1,
        rs_lvgl::Area {
            x1: 8,
            y1: 6,
            x2: 10,
            y2: 8
        }
    );
    assert_eq!(other_regions_changed.get(), 1);
    assert_eq!(waits.get(), 4);
    drop(samples);

    let frames = frames.borrow();
    assert_eq!(frames.len(), 4);
    for (index, pixels) in frames.iter().enumerate() {
        for y in 0..9 {
            for x in 0..11 {
                let expected = if (1..=3).contains(&x) && (1..=2).contains(&y) {
                    match index {
                        0 => [0, 0, 0],
                        1 | 2 => [0, 0, 255],
                        _ => [0, 255, 0],
                    }
                } else if x == 8 && (6..=7).contains(&y) {
                    match index {
                        0 | 1 => [0, 0, 0],
                        2 => [255, 0, 0],
                        _ => [0, 0, 255],
                    }
                } else {
                    [0, 0, 0]
                };
                assert_eq!(
                    &pixels[y * 40 + x * 3..y * 40 + x * 3 + 3],
                    &expected,
                    "frame {index}, ({x},{y})"
                );
            }
        }
    }
    // Retire the final transfer explicitly on the UI thread. A subsequent
    // no-damage refresh may synchronize buffers, but must not wait twice.
    let last = pending.borrow_mut().take().unwrap();
    assert!(last.last);
    unsafe {
        assert!(!complete(last));
        display.flush_ready();
        rs_lvgl::sys::lv_refr_now(display.raw().as_ptr());
        assert_eq!(
            display.active_buffer().unwrap().as_bytes(),
            frames.last().unwrap()
        );
    }
    assert_eq!(waits.get(), 4);
    drop(frames);
    unsafe { display.delete() };
}

#[test]
fn invalidation_alignment_preserves_padded_pixels_and_clips_native_regions() {
    // Expected logical rectangles are independent of the implementation helper.
    let cases = [
        ((1, 1), (2, 3, 4, 4), (2, 3, 4, 4)),
        ((4, 1), (2, 3, 4, 4), (0, 3, 7, 4)),
        ((3, 2), (-5, -2, 2, 2), (0, 0, 2, 3)),
        ((4, 4), (9, 8, 20, 15), (8, 8, 10, 8)),
        ((5, 3), (7, 4, 7, 4), (5, 3, 9, 5)),
        ((16, 16), (8, 7, 8, 7), (0, 0, 10, 8)),
        ((u32::MAX, u32::MAX), (3, 4, 3, 4), (0, 0, 10, 8)),
        ((2, 2), (4, 2, 7, 5), (4, 2, 7, 5)),
        ((4, 4), (1, 1, 3, 2), (0, 0, 3, 3)),
    ];
    for mode in [RenderMode::Partial, RenderMode::Direct] {
        for double in [false, true] {
            for (granularity, input, expected) in cases {
                reset();
                let observed = Rc::new(RefCell::new(Vec::new()));
                let capture = observed.clone();
                let rows = if mode == RenderMode::Partial {
                    granularity.1.clamp(5, 9)
                } else {
                    9
                };
                let storage = if double {
                    DisplayBuffers::Double {
                        front: buffer(40 * rows as usize),
                        back: buffer(40 * rows as usize + 7),
                    }
                } else {
                    DisplayBuffers::Single(buffer(40 * rows as usize))
                };
                let display = DisplayBuilder::new(11, 9)
                    .render_mode(mode)
                    .stride(40)
                    .invalidation_alignment(granularity.0, granularity.1)
                    .buffers(storage)
                    .build(move |info| {
                        let row_bytes = (info.area.x2 - info.area.x1 + 1) as usize * 3;
                        for row in unsafe { info.rows() } {
                            assert_eq!(row.len(), row_bytes);
                            assert!(row.chunks_exact(3).all(|pixel| pixel == [0, 0, 255]));
                        }
                        if mode == RenderMode::Direct {
                            assert_eq!(info.origin(), (0, 0));
                            assert_eq!(info.stride(), 40);
                        } else {
                            assert_eq!(info.origin(), (info.area.x1, info.area.y1));
                        }
                        capture.borrow_mut().push(info.area);
                    })
                    .unwrap();
                unsafe {
                    paint(&display, 255, 0, 0);
                    observed.borrow_mut().clear();
                    let area = rs_lvgl::sys::lv_area_t {
                        x1: input.0,
                        y1: input.1,
                        x2: input.2,
                        y2: input.3,
                    };
                    rs_lvgl::sys::lv_obj_invalidate_area(
                        rs_lvgl::sys::lv_display_get_screen_active(display.raw().as_ptr()),
                        &area,
                    );
                    rs_lvgl::sys::lv_refr_now(display.raw().as_ptr());
                }
                let samples = observed.borrow();
                assert!(
                    !samples.is_empty(),
                    "{mode:?}, double={double}, {granularity:?}"
                );
                let mut next_row = expected.1;
                for area in samples.iter() {
                    assert_eq!((area.x1, area.x2), (expected.0, expected.2));
                    assert_eq!(area.y1, next_row);
                    assert!(area.y2 >= area.y1 && area.y2 <= expected.3);
                    next_row = area.y2 + 1;
                }
                assert_eq!(next_row, expected.3 + 1);
                drop(samples);
                unsafe { display.delete() };
            }
        }
    }
}

#[test]
fn native_direct_sync_uses_the_aligned_invalidation_grid() {
    unsafe extern "C" fn sync_ready(
        display: *mut rs_lvgl::sys::lv_display_t,
        _: *const rs_lvgl::sys::lv_area_t,
    ) {
        // This metadata oracle makes native synchronization observable; actual
        // automatic copies are exercised by the separate delayed pixel oracle.
        rs_lvgl::sys::lv_display_sync_ready(display);
    }
    unsafe extern "C" fn observe(event: *mut rs_lvgl::sys::lv_event_t) {
        let areas = &*rs_lvgl::sys::lv_event_get_user_data(event)
            .cast::<RefCell<Vec<(i32, i32, i32, i32)>>>();
        let area = &*rs_lvgl::sys::lv_event_get_param(event).cast::<rs_lvgl::sys::lv_area_t>();
        areas
            .borrow_mut()
            .push((area.x1, area.y1, area.x2, area.y2));
    }
    reset();
    let display = DisplayBuilder::new(12, 8)
        .render_mode(RenderMode::Direct)
        .stride(40)
        .invalidation_alignment(4, 2)
        .buffers(DisplayBuffers::Double {
            front: buffer(320),
            back: buffer(320),
        })
        .build(|_| {})
        .unwrap();
    let areas = Box::new(RefCell::new(Vec::<(i32, i32, i32, i32)>::new()));
    unsafe {
        let raw = display.raw().as_ptr();
        rs_lvgl::sys::lv_display_add_event_cb(
            raw,
            Some(observe),
            rs_lvgl::sys::lv_event_code_t_LV_EVENT_SYNC_START,
            (&*areas as *const RefCell<Vec<_>>).cast_mut().cast(),
        );
        rs_lvgl::sys::lv_display_set_sync_cb(raw, Some(sync_ready));
        paint(&display, 255, 0, 0);
        let area = rs_lvgl::sys::lv_area_t {
            x1: 5,
            y1: 3,
            x2: 5,
            y2: 3,
        };
        rs_lvgl::sys::lv_obj_invalidate_area(
            rs_lvgl::sys::lv_display_get_screen_active(raw),
            &area,
        );
        rs_lvgl::sys::lv_refr_now(raw);
        let mut actual = areas.borrow().clone();
        actual.sort();
        let mut expected = vec![(0, 0, 11, 1), (0, 4, 11, 7), (0, 2, 3, 3), (8, 2, 11, 3)];
        expected.sort();
        assert_eq!(actual, expected);
        areas.borrow_mut().clear();
        // With no new damage the remaining prior dirty area is synchronized.
        rs_lvgl::sys::lv_refr_now(raw);
        assert_eq!(&*areas.borrow(), &[(4, 2, 7, 3)]);
        display.delete();
    }
    // Event storage remains valid through display deletion.
    drop(areas);
}

#[test]
fn invalidation_alignment_survives_handle_drop_and_reinitialization() {
    for delete in [true, false] {
        for _ in 0..3 {
            reset();
            let retained = Rc::new(RefCell::new(Vec::new()));
            let captured = retained.clone();
            let display = DisplayBuilder::new(11, 9)
                .render_mode(RenderMode::Direct)
                .stride(40)
                .invalidation_alignment(4, 3)
                .buffers(DisplayBuffers::Single(buffer(360)))
                .build(move |info| captured.borrow_mut().push(info.area))
                .unwrap();
            let alias = rs_lvgl::Display::default();
            drop(display);
            assert_eq!(Rc::strong_count(&retained), 2);
            unsafe {
                paint(&alias, 0, 255, 0);
                retained.borrow_mut().clear();
                let area = rs_lvgl::sys::lv_area_t {
                    x1: 9,
                    y1: 7,
                    x2: 9,
                    y2: 7,
                };
                rs_lvgl::sys::lv_obj_invalidate_area(
                    rs_lvgl::sys::lv_display_get_screen_active(alias.raw().as_ptr()),
                    &area,
                );
                rs_lvgl::sys::lv_refr_now(alias.raw().as_ptr());
                assert_eq!(
                    &*retained.borrow(),
                    &[rs_lvgl::Area {
                        x1: 8,
                        y1: 6,
                        x2: 10,
                        y2: 8
                    }]
                );
                if delete {
                    alias.delete();
                } else {
                    rs_lvgl::deinit();
                }
            }
            assert_eq!(Rc::strong_count(&retained), 1);
        }
    }
}

#[test]
fn invalid_alignment_and_insufficient_partial_row_groups_reject_before_registration() {
    use rs_lvgl::DisplayConfigError;
    reset();
    let retained = Rc::new(());
    for (columns, rows) in [(0, 1), (1, 0), (0, 0)] {
        let capture = retained.clone();
        let result = DisplayBuilder::new(11, 9)
            .invalidation_alignment(columns, rows)
            .buffers(DisplayBuffers::Single(buffer(360)))
            .build(move |_| {
                let _ = &capture;
            });
        assert!(matches!(
            result,
            Err(DisplayConfigError::InvalidInvalidationAlignment)
        ));
        assert_eq!(Rc::strong_count(&retained), 1);
        assert!(unsafe { rs_lvgl::sys::lv_display_get_default() }.is_null());
    }
    for double in [false, true] {
        let storage = if double {
            DisplayBuffers::Double {
                front: buffer(80),
                back: buffer(360),
            }
        } else {
            DisplayBuffers::Single(buffer(80))
        };
        let result = DisplayBuilder::new(11, 9)
            .stride(40)
            .invalidation_alignment(4, 4)
            .buffers(storage)
            .build(|_| {});
        assert!(matches!(result, Err(DisplayConfigError::BufferTooSmall)));
        assert!(unsafe { rs_lvgl::sys::lv_display_get_default() }.is_null());
    }
}

#[test]
fn invalidation_granularity_is_per_display_and_full_mode_preserves_full_refresh() {
    reset();
    let first = Rc::new(RefCell::new(Vec::new()));
    let second = Rc::new(RefCell::new(Vec::new()));
    let a = first.clone();
    let b = second.clone();
    let aligned = DisplayBuilder::new(11, 9)
        .render_mode(RenderMode::Direct)
        .stride(40)
        .invalidation_alignment(3, 2)
        .buffers(DisplayBuffers::Single(buffer(360)))
        .build(move |info| a.borrow_mut().push(info.area))
        .unwrap();
    let full = DisplayBuilder::new(7, 5)
        .render_mode(RenderMode::Full)
        .stride(32)
        .invalidation_alignment(4, 3)
        .buffers(DisplayBuffers::Double {
            front: buffer(160),
            back: buffer(160),
        })
        .build(move |info| b.borrow_mut().push(info.area))
        .unwrap();
    unsafe {
        paint(&aligned, 255, 0, 0);
        paint(&full, 0, 255, 0);
        first.borrow_mut().clear();
        second.borrow_mut().clear();
        let area = rs_lvgl::sys::lv_area_t {
            x1: 4,
            y1: 3,
            x2: 4,
            y2: 3,
        };
        for display in [&aligned, &full] {
            rs_lvgl::sys::lv_obj_invalidate_area(
                rs_lvgl::sys::lv_display_get_screen_active(display.raw().as_ptr()),
                &area,
            );
            rs_lvgl::sys::lv_refr_now(display.raw().as_ptr());
        }
        assert_eq!(
            &*first.borrow(),
            &[rs_lvgl::Area {
                x1: 3,
                y1: 2,
                x2: 5,
                y2: 3
            }]
        );
        assert_eq!(
            &*second.borrow(),
            &[rs_lvgl::Area {
                x1: 0,
                y1: 0,
                x2: 6,
                y2: 4
            }]
        );
        aligned.delete();
        // Deleting one display must not remove the other's event state.
        second.borrow_mut().clear();
        paint(&full, 0, 0, 255);
        assert_eq!(second.borrow().len(), 1);
        full.delete();
    }
}

#[test]
fn invalid_configuration_never_registers_a_native_display() {
    use rs_lvgl::DisplayConfigError as Error;
    reset();
    let cases = [
        (0, 3, 32, 128, Error::InvalidDimensions),
        (65536, 3, 32, 128, Error::InvalidDimensions),
        (7, 3, 20, 128, Error::InvalidStride),
        (7, 3, 65536, 128, Error::InvalidStride),
        (7, 3, 32, 95, Error::BufferTooSmall),
    ];
    for (width, height, stride, bytes, expected) in cases {
        let result = DisplayBuilder::new(width, height)
            .render_mode(RenderMode::Full)
            .stride(stride)
            .buffers(DisplayBuffers::Single(buffer(bytes)))
            .build(|_| {});
        assert!(matches!(result, Err(e) if e == expected));
        assert!(unsafe { rs_lvgl::sys::lv_display_get_default() }.is_null());
    }
    let bytes = Box::leak(vec![0; 128].into_boxed_slice());
    let unaligned = &mut bytes[1..];
    assert!(matches!(
        DisplayBuilder::new(7, 3)
            .buffers(DisplayBuffers::Single(PixelBuffer::Static(unaligned)))
            .build(|_| {}),
        Err(Error::UnalignedBuffer)
    ));
    assert!(matches!(
        DisplayBuilder::new(7, 3).build(|_| {}),
        Err(Error::MissingBuffer)
    ));
}

#[test]
fn async_wait_finishes_prior_buffer_before_reuse() {
    reset();
    let busy = Rc::new(Cell::new(None::<(usize, Vec<u8>)>));
    let submitted = Rc::new(RefCell::new(Vec::new()));
    let waits = Rc::new(Cell::new(0));
    let busy_flush = busy.clone();
    let busy_wait = busy.clone();
    let addresses = submitted.clone();
    let completed = waits.clone();
    // The simulated DMA retains a raw pointer only. wait verifies the data did
    // not change in flight, then retires the transfer before LVGL can reuse it.
    let mut display = unsafe {
        DisplayBuilder::new(7, 8)
            .buffers(DisplayBuffers::Double {
                front: buffer(32),
                back: buffer(48),
            })
            .build_async(
                move |info| {
                    assert!(busy_flush.take().is_none());
                    let ptr = info.data().as_ptr() as usize;
                    let count = info.buffer().len();
                    let bytes = core::slice::from_raw_parts(ptr as *const u8, count).to_vec();
                    busy_flush.set(Some((ptr, bytes)));
                    addresses.borrow_mut().push(ptr);
                },
                move || {
                    let (ptr, bytes) = busy_wait.take().expect("one pending transfer");
                    assert_eq!(
                        core::slice::from_raw_parts(ptr as *const u8, bytes.len()),
                        bytes
                    );
                    completed.set(completed.get() + 1);
                },
            )
            .unwrap()
    };
    unsafe {
        paint(&display, 255, 0, 0);
        paint(&display, 0, 0, 255);
    }
    // Native wait occurs before reuse, and the last transfer can remain in flight.
    assert!(waits.get() > 1);
    let addresses = submitted.borrow();
    assert!(addresses.len() > 2);
    assert!(addresses.windows(2).all(|p| p[0] != p[1]));
    drop(addresses);
    busy.take(); // complete the final simulated transfer before deletion.
    unsafe {
        display.flush_ready();
        display.delete();
    }
}

#[test]
fn callback_state_survives_handle_drop_and_releases_on_native_delete() {
    reset();
    let retained = Rc::new(());
    let copy = retained.clone();
    let display = DisplayBuilder::new(7, 3)
        .buffers(DisplayBuffers::Single(buffer(96)))
        .build(move |_| {
            let _ = &copy;
        })
        .unwrap();
    let alias = rs_lvgl::Display::default();
    drop(display);
    assert_eq!(Rc::strong_count(&retained), 2);
    unsafe {
        paint(&alias, 0, 255, 0);
        alias.delete();
    }
    assert_eq!(Rc::strong_count(&retained), 1);
    let rejected = retained.clone();
    assert!(DisplayBuilder::new(0, 3)
        .build(move |_| {
            let _ = &rejected;
        })
        .is_err());
    assert_eq!(Rc::strong_count(&retained), 1);
}

#[test]
fn configured_display_releases_backend_across_native_reinitialization() {
    for _ in 0..3 {
        reset();
        let retained = Rc::new(());
        let captured = retained.clone();
        let display = DisplayBuilder::new(7, 3)
            .buffers(DisplayBuffers::Single(buffer(96)))
            .build(move |_| {
                let _ = &captured;
            })
            .unwrap();
        unsafe {
            paint(&display, 0, 0, 255);
        }
        drop(display);
        assert_eq!(Rc::strong_count(&retained), 2);
        unsafe {
            rs_lvgl::deinit();
        }
        assert_eq!(Rc::strong_count(&retained), 1);
        rs_lvgl::init();
    }
}

#[test]
fn display_and_input_periods_are_independent_and_checked() {
    use rs_lvgl::input_device::{
        Encoder, EncoderInputData, InputDriver, Pointer, PointerInputData,
    };
    use rs_lvgl::PeriodError;
    reset();
    let mut display = DisplayBuilder::new(7, 3)
        .buffers(DisplayBuffers::Single(buffer(96)))
        .build(|_| {})
        .unwrap();
    let mut pointer =
        Pointer::register(|| PointerInputData::Key(0).released().once(), &display).unwrap();
    let mut encoder =
        Encoder::register(|| EncoderInputData::Press.released().once(), &display).unwrap();
    for millis in [1, 16, 33, 1000, u32::MAX as u64] {
        display
            .set_refresh_period(Duration::from_millis(millis))
            .unwrap();
        assert_eq!(
            display.refresh_period().unwrap(),
            Duration::from_millis(millis)
        );
    }
    pointer.set_read_period(Duration::from_millis(8)).unwrap();
    encoder.set_read_period(Duration::from_millis(20)).unwrap();
    display
        .set_refresh_period(Duration::from_millis(1000))
        .unwrap();
    for period in [
        Duration::ZERO,
        Duration::from_nanos(500_000),
        Duration::from_micros(16_001),
        Duration::from_millis(u32::MAX as u64 + 1),
    ] {
        assert_eq!(
            display.set_refresh_period(period),
            Err(PeriodError::InvalidPeriod)
        );
        assert_eq!(
            pointer.set_read_period(period),
            Err(PeriodError::InvalidPeriod)
        );
    }
    assert_eq!(
        display.refresh_period().unwrap(),
        Duration::from_millis(1000)
    );
    assert_eq!(pointer.read_period().unwrap(), Duration::from_millis(8));
    assert_eq!(encoder.read_period().unwrap(), Duration::from_millis(20));
    #[cfg(feature = "input-keypad")]
    {
        use rs_lvgl::input_device::{Keypad, KeypadInputData};
        let mut keypad = Keypad::register(|| KeypadInputData::released(0), &display).unwrap();
        keypad.set_read_period(Duration::from_millis(12)).unwrap();
        assert_eq!(keypad.read_period().unwrap(), Duration::from_millis(12));
        unsafe {
            rs_lvgl::sys::lv_indev_delete(keypad.raw().unwrap().as_ptr());
        }
        assert_eq!(keypad.read_period(), Err(PeriodError::TimerUnavailable));
    }
    drop((pointer, encoder));
    unsafe {
        rs_lvgl::sys::lv_display_delete_refr_timer(display.raw().as_ptr());
    }
    assert_eq!(
        display.set_refresh_period(Duration::from_millis(16)),
        Err(PeriodError::TimerUnavailable)
    );
    assert_eq!(display.refresh_period(), Err(PeriodError::TimerUnavailable));
    unsafe {
        display.delete();
    }
}
