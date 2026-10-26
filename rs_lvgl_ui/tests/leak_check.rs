//! Strict lifecycle accounting. Run through scripts/check-ui-leaks.py.
//! Each workload executes in a fresh process, with one declared priming cycle.

use rs_lvgl_ui::prelude::*;
use rs_lvgl_ui::{components::dialog::Dialog, *};
use std::alloc::{GlobalAlloc, Layout, System};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

use rs_lvgl::input_device::{key, Keypad, KeypadInputData};

struct CountingAllocator;
static RUST_BYTES: AtomicUsize = AtomicUsize::new(0);
static RUST_BLOCKS: AtomicUsize = AtomicUsize::new(0);

// SAFETY: all pointers/layouts are forwarded unchanged to System. Only successful
// allocations change the counters; instrumentation performs no allocation.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            RUST_BYTES.fetch_add(layout.size(), SeqCst);
            RUST_BLOCKS.fetch_add(1, SeqCst);
        }
        pointer
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            RUST_BYTES.fetch_add(layout.size(), SeqCst);
            RUST_BLOCKS.fetch_add(1, SeqCst);
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        RUST_BYTES.fetch_sub(layout.size(), SeqCst);
        RUST_BLOCKS.fetch_sub(1, SeqCst);
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let replacement = unsafe { System.realloc(pointer, layout, size) };
        if !replacement.is_null() {
            RUST_BYTES.fetch_add(size, SeqCst);
            RUST_BYTES.fetch_sub(layout.size(), SeqCst);
        }
        replacement
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Usage {
    rust_bytes: usize,
    rust_blocks: usize,
    native_bytes: usize,
    native_blocks: usize,
}

unsafe extern "C" {
    // Internal to the pinned LVGL 9.6 software renderer; used only by this test.
    fn lv_draw_sw_mask_cleanup();
}

fn clear_render_cache() {
    // The software renderer retains up to four radius masks across displays.
    // Their replacement can change live bytes at a constant block count. Clear
    // only this known global cache, after exercising the real cached renderer.
    // SAFETY: every display has been deleted after synchronous rendering;
    // no draw task retains an acquired radius mask.
    unsafe { lv_draw_sw_mask_cleanup() };
}

fn observable_allocator() {
    assert_eq!(
        rs_lvgl_sys::LV_USE_STDLIB_MALLOC,
        rs_lvgl_sys::LV_STDLIB_BUILTIN,
        "LEAK_UNOBSERVABLE_NATIVE: run scripts/check-ui-leaks.py; CLIB monitor results are not measurements"
    );
}

fn usage() -> Usage {
    let mut native = rs_lvgl_sys::lv_mem_monitor_t::default();
    // SAFETY: the child test has initialized LVGL; this stack monitor is valid
    // for the synchronous call, which neither retains it nor allocates.
    unsafe { rs_lvgl_sys::lv_mem_monitor(&mut native) };
    assert!(native.total_size > 0 && native.free_size <= native.total_size);
    Usage {
        rust_bytes: RUST_BYTES.load(SeqCst),
        rust_blocks: RUST_BLOCKS.load(SeqCst),
        native_bytes: native.total_size - native.free_size,
        native_blocks: native.used_cnt,
    }
}

fn assert_balance(before: Usage, after: Usage, case: &str, iteration: usize) {
    assert_eq!(
        (after.rust_bytes, after.rust_blocks),
        (before.rust_bytes, before.rust_blocks),
        "LEAK_RUST: {case} iteration {iteration}; {before:?} -> {after:?}"
    );
    assert_eq!(
        (after.native_bytes, after.native_blocks),
        (before.native_bytes, before.native_blocks),
        "LEAK_NATIVE: {case} iteration {iteration}; {before:?} -> {after:?}"
    );
}

fn display_cycle(work: impl FnOnce(&rs_lvgl::Display)) {
    let retained = vec![7u8; 71];
    let display = rs_lvgl::Display::register::<_, { 64 * 64 }>(64, 64, move |_| {
        std::hint::black_box(&retained);
    })
    .unwrap();
    work(&display);
    // SAFETY: the closure dropped/unmounted every UI, input and snapshot. Flush
    // completion is synchronous; no alias or pending transfer survives it.
    unsafe { display.delete() };
}

struct TestApp {
    kind: usize,
    overlay: bool,
    changed: bool,
}

impl App for TestApp {
    type Msg = String;
    fn update(&mut self, _: String) {
        self.changed = !self.changed;
    }
    fn view(&self, _: &Theme) -> View<String> {
        match self.kind {
            0 => column([]).children((0..12).map(|index| {
                let index = if self.changed { 11 - index } else { index };
                button(format!("item {index}"))
                    .key(Key::Id(index))
                    .on_click(format!("message {index}"))
            })),
            1 => button("Styled")
                .key(Key::Id(0))
                .on_click("pressed".into())
                .style(
                    Style::default()
                        .outline_width(0)
                        .transition(StyleTransition::new(
                            &[
                                TransitionProperty::OutlineWidth,
                                TransitionProperty::ShadowWidth,
                            ],
                            30,
                        )),
                )
                .state_style(
                    State::PRESSED,
                    Style::default().outline_width(3).shadow_width(5),
                )
                .animate(Animation::new(AnimationProperty::Opacity, 100, 255, 40).reverse(30)),
            2 => canvas(
                Canvas::new(if self.changed { 19 } else { 17 }, 13)
                    .background(Color::rgb(30, 50, 80), 255)
                    .draw(DrawCommand::Text {
                        x: 0,
                        y: 0,
                        width: 17,
                        height: 13,
                        text: "Canvas".into(),
                        font: Font::default(),
                        color: Color::rgb(255, 255, 255),
                        opacity: 255,
                    }),
            ),
            3 => qrcode(QrCode::new(b"bytes\0retained".to_vec(), 58)),
            4 => barcode(Barcode::new("A B 123", 20)),
            5 => gif(Gif::new(&include_bytes!("assets/two-colors.gif")[..])),
            6 => lottie(Lottie::new(
                include_str!("assets/moving-square.json"),
                32,
                32,
            )),
            7 => column([])
                .child(textarea("text").keyboard_id(1))
                .child(pinyin_keyboard(1, Font::cjk_16())),
            8 => lottie(Lottie::new("{}", 32, 32)),
            _ => obj().style(Style::default().width(Size::Px(-1))),
        }
    }
    fn overlay(&self, theme: &Theme) -> Option<View<String>> {
        self.overlay.then(|| {
            Dialog::new("Confirm")
                .text("Retained dialog")
                .primary("OK", "accept".into())
                .view(theme)
        })
    }
}

fn advance() {
    // SAFETY: called only with a live display on the child UI thread. Synchronous
    // callbacks enqueue messages or finish drawing; none unwinds across C.
    unsafe {
        rs_lvgl_sys::lv_tick_inc(100);
        rs_lvgl_sys::lv_anim_refr_now();
        rs_lvgl_sys::lv_timer_handler();
    }
}

fn workload(case: &str) {
    if case == "display" {
        display_cycle(|_| advance());
        return;
    }
    display_cycle(|display| {
        let mut app = TestApp {
            kind: 0,
            overlay: false,
            changed: false,
        };
        if case == "screens" {
            let mut screens = Screens::new(display, Theme::dark()).unwrap();
            screens.show(&app, ScreenTransition::immediate()).unwrap();
            for _ in 0..3 {
                app.overlay = !app.overlay;
                screens
                    .show(&app, ScreenTransition::new(ScreenEffect::MoveLeft, 40))
                    .unwrap();
                rs_lvgl::tick_inc(core::time::Duration::from_millis(10));
                screens.timer_handler();
            }
            advance();
            screens.timer_handler();
            screens
                .show(&app, ScreenTransition::new(ScreenEffect::FadeIn, 40))
                .unwrap();
            screens.unmount();
            return;
        }
        let mut ui = Ui::new(display, Theme::dark()).unwrap();
        match case {
            "callbacks" => {
                ui.mount(&app).unwrap();
                for _ in 0..3 {
                    // SAFETY: all three children belong to this mounted view.
                    unsafe {
                        let screen = display.get_scr_act().unwrap().raw().as_ptr();
                        let root = rs_lvgl_sys::lv_obj_get_child(screen, 0);
                        let column = rs_lvgl_sys::lv_obj_get_child(root, 0);
                        let button = rs_lvgl_sys::lv_obj_get_child(column, 0);
                        rs_lvgl_sys::lv_obj_send_event(
                            button,
                            rs_lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED,
                            core::ptr::null_mut(),
                        );
                    }
                    assert_eq!(ui.drain(&mut app, 8).unwrap().processed, 1);
                }
            }
            "styles" => {
                app.kind = 1;
                ui.mount(&app).unwrap();
                advance();
                // SAFETY: this styled button is the mounted app root. Start a
                // native state transition, then remove it before it completes.
                unsafe {
                    let screen = display.get_scr_act().unwrap().raw().as_ptr();
                    let root = rs_lvgl_sys::lv_obj_get_child(screen, 0);
                    let button = rs_lvgl_sys::lv_obj_get_child(root, 0);
                    rs_lvgl_sys::lv_obj_add_state(button, rs_lvgl_sys::lv_state_t_LV_STATE_PRESSED);
                    rs_lvgl_sys::lv_tick_inc(10);
                    rs_lvgl_sys::lv_anim_refr_now();
                }
                ui.update(&app).unwrap();
                app.kind = 0;
                ui.update(&app).unwrap();
            }
            "focus" => {
                let reader_data = vec![1u8; 83];
                let keypad = Keypad::register(
                    move || {
                        std::hint::black_box(&reader_data);
                        KeypadInputData::released(key::ENTER)
                    },
                    display,
                )
                .unwrap();
                ui.bind_keypad(&keypad).unwrap();
                ui.mount(&app).unwrap();
                assert!(ui.focus(Key::Id(0)).unwrap());
                app.overlay = true;
                ui.update(&app).unwrap();
                app.overlay = false;
                ui.update(&app).unwrap();
                ui.unmount();
                drop(keypad);
            }
            "canvas" => {
                app.kind = 2;
                ui.mount(&app).unwrap();
                let first = ui.snapshot().unwrap();
                app.changed = true;
                ui.update(&app).unwrap();
                let resized = ui.snapshot().unwrap();
                assert_ne!(first.width, resized.width);
                ui.unmount();
                drop((first, resized));
            }
            "media" | "qrcode" | "barcode" | "gif" | "lottie" | "pinyin" => {
                let kinds = match case {
                    "qrcode" => 3..4,
                    "barcode" => 4..5,
                    "gif" => 5..6,
                    "lottie" => 6..7,
                    "pinyin" => 7..8,
                    _ => 3..8,
                };
                for kind in kinds {
                    app.kind = kind;
                    ui.mount(&app).unwrap();
                    advance();
                    drop(ui.snapshot().unwrap());
                }
            }
            "rollback" => {
                ui.mount(&app).unwrap();
                app.kind = 9;
                assert_eq!(ui.update(&app), Err(UiError::InvalidProperty));
                app.kind = 8;
                assert_eq!(ui.update(&app), Err(UiError::InvalidProperty));
                app.kind = 0;
                ui.mount(&app).unwrap();
            }
            _ => panic!("unknown leak workload"),
        }
        ui.unmount();
    });
}

fn child(case: &str, expected_failure: Option<&str>) {
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "leak_subprocess",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("LVGL_LEAK_CASE", case)
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if let Some(marker) = expected_failure {
        assert!(
            !output.status.success() && text.contains(marker),
            "wrong negative-control failure:\n{text}"
        );
        for line in text.lines().filter(|line| line.contains(marker)) {
            println!("LEAK_CONTROL {case}: {line}");
        }
    } else {
        assert!(
            output.status.success() && text.contains("LEAK_BALANCED"),
            "{case}:\n{text}"
        );
        for line in text.lines().filter(|line| line.contains("LEAK_BALANCED")) {
            println!("{line}");
        }
    }
}

#[test]
fn allocator_is_observable() {
    observable_allocator();
}

#[test]
fn both_heaps_balance_across_resource_lifecycles() {
    observable_allocator();
    for case in [
        "display",
        "callbacks",
        "styles",
        "focus",
        "canvas",
        "media",
        "qrcode",
        "barcode",
        "gif",
        "lottie",
        "pinyin",
        "screens",
        "rollback",
    ] {
        child(case, None);
    }
}

#[test]
fn detects_one_byte_rust_leak() {
    observable_allocator();
    child("rust-negative", Some("LEAK_RUST:"));
}

#[test]
fn detects_minimum_native_leak() {
    observable_allocator();
    child("native-negative", Some("LEAK_NATIVE:"));
}

#[test]
#[ignore = "invoked in an isolated subprocess by the parent tests"]
fn leak_subprocess() {
    observable_allocator();
    let case = std::env::var("LVGL_LEAK_CASE").expect("parent supplies the workload");
    // SAFETY: this fresh process has no application objects or pending rendering.
    unsafe { rs_lvgl::deinit() };
    rs_lvgl::init();
    let negative = case.ends_with("-negative");
    if !negative {
        workload(&case);
        clear_render_cache();
    }
    let before = usage();
    for iteration in 0..64 {
        match case.as_str() {
            "rust-negative" => {
                std::hint::black_box(Box::leak(Box::new(1u8)));
            }
            "native-negative" => {
                // SAFETY: the deliberately retained native allocation is confined
                // to this negative-control child, which exits after detection.
                assert!(!unsafe { rs_lvgl_sys::lv_malloc(1) }.is_null());
            }
            _ => workload(&case),
        }
        if !negative {
            clear_render_cache();
        }
        assert_balance(before, usage(), &case, iteration);
    }
    // SAFETY: every measured lifecycle returned without live objects or callbacks.
    unsafe { rs_lvgl::deinit() };
    println!("LEAK_BALANCED {case}: 64 cycles; baseline {before:?}");
}
