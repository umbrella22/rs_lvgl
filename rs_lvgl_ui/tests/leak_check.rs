//! Strict lifecycle accounting. Run through scripts/check-ui-leaks.py.
//! Each workload executes in a fresh process, with one declared priming cycle.

use rs_lvgl_ui::prelude::*;
use rs_lvgl_ui::{components::dialog::Dialog, *};
use std::alloc::{GlobalAlloc, Layout, System};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::{cell::Cell, ptr::NonNull, rc::Rc};

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
    type Action = String;
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
            _ => container().style(Style::default().width(Size::Px(-1))),
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

struct FontBacking {
    destroyed: Rc<Cell<bool>>,
    dropped: Rc<Cell<bool>>,
}
impl Drop for FontBacking {
    fn drop(&mut self) {
        assert!(
            self.destroyed.get(),
            "native font destructor precedes backing drop"
        );
        assert!(!self.dropped.replace(true), "backing drops once");
    }
}
unsafe extern "C" fn release_loaded_font(font: *mut rs_lvgl_sys::lv_font_t) {
    // SAFETY: loaded_font allocated the descriptor with lv_malloc, and its
    // user_data points to a Cell retained by FontBacking until after this call.
    unsafe {
        (*((*font).user_data.cast::<Cell<bool>>())).set(true);
        rs_lvgl_sys::lv_free(font.cast());
    }
}
fn loaded_font(dropped: Rc<Cell<bool>>) -> FontResource {
    let destroyed = Rc::new(Cell::new(false));
    // SAFETY: the serial child has initialized LVGL. Only the descriptor uses
    // native storage; all glyph tables/callbacks are static Montserrat data.
    unsafe {
        let font = NonNull::new(
            rs_lvgl_sys::lv_malloc(core::mem::size_of::<rs_lvgl_sys::lv_font_t>())
                .cast::<rs_lvgl_sys::lv_font_t>(),
        )
        .unwrap();
        font.as_ptr()
            .write(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_14).read());
        (*font.as_ptr()).user_data = Rc::as_ptr(&destroyed).cast_mut().cast();
        FontResource::from_raw_owned(
            font,
            FontBacking { destroyed, dropped },
            release_loaded_font,
        )
    }
}

struct FailingResourceWidget;
impl CustomWidget for FailingResourceWidget {
    type Props = ();
    fn apply_props(&mut self, _: &mut CustomObject, _: &()) -> Result<(), UiError> {
        Ok(())
    }
    fn create(&mut self, _: &mut Obj<'_>, _: &Self::Props) -> Result<CustomObject, UiError> {
        Err(UiError::Allocation)
    }
    fn tick(&mut self, _: &mut CustomObject, _: u32) -> Result<(), UiError> {
        Ok(())
    }
    fn delete(&mut self, _: &mut CustomObject) {
        panic!("failed create has no delete hook")
    }
}
struct ResourcePage {
    image: Option<ImageResource>,
    font: Option<FontResource>,
    fail: bool,
}
impl App for ResourcePage {
    type Action = String;
    fn update(&mut self, _: String) {}
    fn view(&self, _: &Theme) -> View<String> {
        let image = || match &self.image {
            Some(source) => image_resource(source.clone()),
            None => rs_lvgl_ui::image(ImageSource::rgb565(1, 1, &[0, 0])),
        };
        let root = column([
            image().key(Key::Id(1)),
            image().key(Key::Id(2)),
            if self.fail {
                custom(FailingResourceWidget, ())
            } else {
                label_static("valid")
            }
            .key(Key::Id(3)),
        ]);
        match &self.font {
            Some(font) => root.font_resource(font.clone()),
            None => root,
        }
    }
}

fn resource_workload(ui: &mut Ui<String>) {
    let dropped = Rc::new(Cell::new(false));
    let pixels: Rc<[u8]> = Rc::from([0xe0u8, 0x07]);
    let weak = Rc::downgrade(&pixels);
    let mut page = ResourcePage {
        image: Some(ImageResource::rgb565(1, 1, 2, pixels).unwrap()),
        font: Some(loaded_font(dropped.clone())),
        fail: false,
    };
    ui.mount(&page).unwrap();
    drop(ui.snapshot().unwrap());
    ui.update(&page).unwrap(); // sharing and stable-order update
    page.image = None;
    page.font = None;
    assert!(weak.upgrade().is_some());
    assert!(!dropped.get());
    ui.update(&page).unwrap(); // install static/inherited resources before release
    assert!(weak.upgrade().is_none());
    assert!(dropped.get());
    drop(ui.snapshot().unwrap());
    for on_update in [false, true] {
        let dropped = Rc::new(Cell::new(false));
        let pixels: Rc<[u8]> = Rc::from([0u8, 0]);
        let weak = Rc::downgrade(&pixels);
        let mut page = ResourcePage {
            image: Some(ImageResource::rgb565(1, 1, 2, pixels).unwrap()),
            font: Some(loaded_font(dropped.clone())),
            fail: !on_update,
        };
        if on_update {
            ui.mount(&page).unwrap();
            page.fail = true;
            assert_eq!(ui.update(&page), Err(UiError::Allocation));
        } else {
            assert_eq!(ui.mount(&page), Err(UiError::Allocation));
        }
        drop(page);
        assert!(weak.upgrade().is_none());
        assert!(dropped.get());
        assert_eq!(ui.stats().mounted_nodes, 0);
    }
    ui.mount(&page).unwrap(); // retry after a later-sibling failure
    ui.unmount();
}

struct CompositionPage {
    show: bool,
    reversed: bool,
}
impl App for CompositionPage {
    type Action = String;
    fn update(&mut self, _: String) {}
    fn view(&self, theme: &Theme) -> View<String> {
        let form = typed::column([])
            .child(typed::slider(
                0,
                100,
                binding(50, |value: i32| value.to_string()),
            ))
            .child(
                typed::textarea(binding("model", |text: &str| text.to_owned())).placeholder("Name"),
            )
            .child_if(self.show, || {
                let retained = vec![19u8; 73];
                typed::switch(binding(false, move |value: bool| {
                    std::hint::black_box(&retained);
                    value.to_string()
                }))
                .key(Key::Named("optional"))
            })
            .children((0..4).map(|position| {
                let id = if self.reversed {
                    3 - position
                } else {
                    position
                };
                typed::button("item")
                    .key(Key::Id(id))
                    .on_click(id.to_string())
                    .into_view()
            }))
            .map(|action| action)
            .into_view();
        let row = components::list::ListItem::new("")
            .leading(typed::button("lead").on_click("lead".into()))
            .content(typed::label("Custom content"))
            .trailing(typed::switch(binding(false, |value: bool| {
                value.to_string()
            })))
            .view(theme);
        components::panel::Panel::new("Form")
            .header_action(typed::button("Save").on_click("save".into()))
            .content(form)
            .child(row)
            .view(theme)
    }
    fn overlay(&self, theme: &Theme) -> Option<View<String>> {
        self.show.then(|| {
            Dialog::new("Details")
                .content(typed::label("Body"))
                .children([label("Extra")])
                .primary("OK", "close".into())
                .view(theme)
        })
    }
}

fn composition_workload(ui: &mut Ui<String>) {
    let mut page = CompositionPage {
        show: true,
        reversed: false,
    };
    ui.mount(&page).unwrap();
    for _ in 0..4 {
        page.show = !page.show;
        page.reversed = !page.reversed;
        ui.update(&page).unwrap();
        ui.post("input".into()).unwrap();
        assert_eq!(ui.drain(&mut page, 8).unwrap().processed, 1);
        drop(ui.snapshot().unwrap());
    }
    ui.unmount();
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
            "display-adaptation" => {
                use rs_lvgl::{CanvasBuffer, ColorFormat, DisplayBuffers, DisplayBuilder, PixelBuffer, RenderMode};
                let captured = vec![0u8; 79];
                let configured = DisplayBuilder::new(16, 16)
                    .render_mode(RenderMode::Direct).stride(64)
                    .invalidation_alignment(4, 2)
                    .buffers(DisplayBuffers::Double {
                        front: PixelBuffer::Owned(vec![0; 1024].into_boxed_slice()),
                        back: PixelBuffer::Owned(vec![0; 1024].into_boxed_slice()),
                    }).build(move |_| { std::hint::black_box(&captured); }).unwrap();
                for format in [ColorFormat::Rgb888, ColorFormat::Rgb565, ColorFormat::Rgb888] {
                    let mut canvas = rs_lvgl::widgets::Canvas::new().unwrap();
                    for _ in 0..3 {
                        canvas.attach_buffer(CanvasBuffer::new(7, 5, format, Some(32),
                            PixelBuffer::Owned(vec![0; 160].into_boxed_slice())).unwrap()).unwrap();
                    }
                    unsafe { rs_lvgl_sys::lv_obj_delete(canvas.raw().as_ptr()); }
                }
                unsafe { rs_lvgl_sys::lv_refr_now(configured.raw().as_ptr()); configured.delete(); }
            }
            "rgb888-images" => {
                struct Picture;
                impl App for Picture {
                    type Action = String;
                    fn update(&mut self, _: String) {}
                    fn view(&self, _: &Theme) -> View<String> {
                        image(ImageSource::rgb888(1, 1, &[0, 0, 255]))
                    }
                }
                ui.mount(&Picture).unwrap();
                drop(ui.snapshot().unwrap());
                ui.unmount();
                let pixels: Rc<[u8]> = Rc::from(&[0u8, 255, 0, 0][..]);
                let loaded = ImageResource::rgb888(1, 1, 4, pixels).unwrap();
                let app = ResourcePage { image: Some(loaded), font: None, fail: false };
                ui.mount(&app).unwrap();
                drop(ui.snapshot().unwrap());
                ui.unmount();
            }
            "editing-depth" => editing_workload(&mut ui),
            "chart-depth" => chart_workload(&mut ui),
            "custom-props" => custom_props_workload(&mut ui),
            "loaded-resources" => resource_workload(&mut ui),
            "composition" => composition_workload(&mut ui),
            "callbacks" => {
                #[cfg(feature = "diagnostics")]
                let diagnostic_weak = {
                    let retained = Rc::new(vec![0u8; 93]);
                    let weak = Rc::downgrade(&retained);
                    ui.set_diagnostic_sink(Some(Rc::new(move |_| {
                        std::hint::black_box(&retained);
                    })));
                    weak
                };
                let observed = Rc::new(Cell::new(0));
                let owned = vec![0u8; 91];
                let weak = Rc::downgrade(&observed);
                let captured = observed.clone();
                ui.set_event_queue_observer(Some(Rc::new(move |_| {
                    std::hint::black_box(&owned);
                    captured.set(captured.get() + 1);
                })));
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
                assert_eq!(observed.get(), 3);
                drop(observed);
                ui.set_event_queue_observer(None);
                assert!(
                    weak.upgrade().is_none(),
                    "observer replacement releases captures"
                );
                #[cfg(feature = "diagnostics")]
                {
                    ui.set_diagnostic_sink(None);
                    assert!(
                        diagnostic_weak.upgrade().is_none(),
                        "sink replacement releases captures"
                    );
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
        "display-adaptation",
        "rgb888-images",
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
        "loaded-resources",
        "composition",
        "editing-depth",
        "chart-depth",
        "custom-props",
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

struct EditingPage {
    pass: usize,
    fail: bool,
}
impl App for EditingPage {
    type Action = String;
    fn update(&mut self, _: String) {}
    fn view(&self, _: &Theme) -> View<String> {
        let mut field = typed::textarea(binding("A中é", |s: &str| s.to_owned()))
            .password(self.pass % 2 == 0)
            .cursor_position(Some(2));
        if self.pass % 3 != 0 {
            field = field.accepted_chars(format!("0123{}", self.pass));
        }
        column([
            field.key(Key::Id(1)).into_view(),
            range_slider(
                -100,
                100,
                SliderRange::new(-(self.pass as i32), self.pass as i32),
            )
            .on_range_changed(|v| format!("{}..{}", v.lower, v.upper))
            .key(Key::Id(2)),
            if self.fail {
                custom(FailingResourceWidget, ())
            } else {
                label_static("valid")
            }
            .key(Key::Id(3)),
        ])
    }
}
fn editing_workload(ui: &mut Ui<String>) {
    let mut app = EditingPage {
        pass: 1,
        fail: false,
    };
    ui.mount(&app).unwrap();
    for pass in 2..10 {
        app.pass = pass;
        ui.update(&app).unwrap();
        drop(ui.snapshot().unwrap());
    }
    app.fail = true;
    assert_eq!(ui.update(&app), Err(UiError::Allocation));
    assert_eq!(
        ui.stats().mounted_nodes,
        0,
        "later-sibling failure retires retained input buffers"
    );
    app.fail = false;
    ui.mount(&app).unwrap();
    ui.unmount();
}

struct HistoryPage {
    history: ChartHistory,
    mode: ChartUpdateMode,
    fail: bool,
}
impl App for HistoryPage {
    type Action = String;
    fn update(&mut self, _: String) {}
    fn view(&self, _: &Theme) -> View<String> {
        let mut spec = ChartSpec::new(
            ChartType::Line,
            ChartAxes::new(ChartRange::new(0, 100), ChartRange::new(0, 100)),
            vec![ChartSeries::new(
                1,
                Color::rgb(200, 80, 30),
                ChartData::History(self.history.snapshot()),
            )],
        );
        spec.update_mode = self.mode;
        spec.cursors.push(ChartCursor::new(
            1,
            Color::rgb(30, 200, 80),
            ChartCursorDirection::Both,
            ChartCursorTarget::Point {
                series: 1,
                index: 0,
            },
        ));
        column([
            chart(spec).key(Key::Id(1)),
            if self.fail {
                unsafe { raw(|_| None) }
            } else {
                label("valid")
            }
            .key(Key::Id(2)),
        ])
    }
}
fn chart_workload(ui: &mut Ui<String>) {
    use core::num::NonZeroU32;
    let mut app = HistoryPage {
        history: ChartHistory::new(1, NonZeroU32::new(8).unwrap(), ChartSampleKind::Y),
        mode: ChartUpdateMode::Shift,
        fail: false,
    };
    ui.mount(&app).unwrap();
    for batch in [1, 4, 16] {
        for value in 0..batch {
            app.history.push_y(value).unwrap();
        }
        ui.update(&app).unwrap();
        drop(ui.snapshot().unwrap());
    }
    app.mode = ChartUpdateMode::Circular;
    app.history.set_capacity(NonZeroU32::new(4).unwrap());
    ui.update(&app).unwrap();
    app.fail = true;
    assert_eq!(ui.update(&app), Err(UiError::Allocation));
    assert_eq!(ui.stats().mounted_nodes, 0);
    app.fail = false;
    ui.mount(&app).unwrap();
    ui.unmount();
    // The dedicated BUILTIN heap cannot satisfy this native allocation. The
    // empty canonical snapshot itself allocates no capacity-sized Rust buffer.
    app.history = ChartHistory::new(
        2,
        NonZeroU32::new(1_000_000_000).unwrap(),
        ChartSampleKind::Y,
    );
    assert_eq!(ui.mount(&app), Err(UiError::Allocation));
    assert_eq!(ui.stats().mounted_nodes, 0);
    app.history = ChartHistory::new(3, NonZeroU32::new(4).unwrap(), ChartSampleKind::Y);
    app.history.push_y(20).unwrap();
    ui.mount(&app).unwrap();
    ui.unmount();
}

struct HeapCustom;
struct HeapProps {
    text: std::ffi::CString,
    fail: bool,
}
impl CustomWidget for HeapCustom {
    type Props = HeapProps;
    fn create(&mut self, parent: &mut Obj<'_>, _: &HeapProps) -> Result<CustomObject, UiError> {
        // SAFETY: return a new native label owned by the mounted subtree.
        unsafe {
            CustomObject::new(
                parent,
                NonNull::new(rs_lvgl_sys::lv_label_create(parent.raw().as_ptr()))
                    .ok_or(UiError::Allocation)?,
            )
        }
    }
    fn apply_props(&mut self, object: &mut CustomObject, props: &HeapProps) -> Result<(), UiError> {
        // SAFETY: boxed props are retained through replacement and native deletion.
        unsafe {
            rs_lvgl_sys::lv_label_set_text_static(
                object.raw().as_ptr(),
                props.text.as_ptr().cast(),
            );
        }
        if props.fail {
            Err(UiError::InvalidProperty)
        } else {
            Ok(())
        }
    }
    fn delete(&mut self, _: &mut CustomObject) {}
}
struct HeapPage {
    value: u32,
    fail: bool,
    sibling: bool,
}
impl App for HeapPage {
    type Action = String;
    fn update(&mut self, _: String) {}
    fn view(&self, _: &Theme) -> View<String> {
        column([
            custom(
                HeapCustom,
                HeapProps {
                    text: std::ffi::CString::new(format!("native {}", self.value)).unwrap(),
                    fail: self.fail,
                },
            )
            .key(Key::Id(1)),
            if self.sibling {
                custom(FailingResourceWidget, ())
            } else {
                label("ready")
            }
            .key(Key::Id(2)),
        ])
    }
}
fn custom_props_workload(ui: &mut Ui<String>) {
    let mut app = HeapPage {
        value: 0,
        fail: false,
        sibling: false,
    };
    ui.mount(&app).unwrap();
    for value in 1..9 {
        app.value = value;
        ui.update(&app).unwrap();
        ui.update(&app).unwrap();
        ui.tick(16).unwrap();
    }
    app.fail = true;
    assert_eq!(ui.update(&app), Err(UiError::InvalidProperty));
    app.fail = false;
    ui.update(&app).unwrap();
    app.sibling = true;
    assert_eq!(ui.update(&app), Err(UiError::Allocation));
    app.sibling = false;
    ui.update(&app).unwrap();
    ui.unmount();
    ui.mount(&app).unwrap();
    ui.unmount();
    app.fail = true;
    assert_eq!(ui.mount(&app), Err(UiError::InvalidProperty));
    ui.unmount();
}
