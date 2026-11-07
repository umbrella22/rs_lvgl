mod common;
use common::*;
use rs_lvgl_ui::prelude::*;
use rs_lvgl_ui::FontResource;
#[cfg(feature = "widget-image")]
use rs_lvgl_ui::ResourceError;
use std::{
    cell::Cell,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering::SeqCst},
};

#[cfg(feature = "widget-image")]
#[test]
fn loaded_images_share_replace_and_release_without_deleting_nodes() {
    use rs_lvgl_ui::ImageResource;
    let _guard = lock();
    let display = init();
    let pixels: Rc<[u8]> = Rc::from([0x00, 0xf8, 0xe0, 0x07]);
    let weak = Rc::downgrade(&pixels);
    assert_eq!(
        ImageResource::rgb565(0, 1, 2, pixels.clone()).unwrap_err(),
        ResourceError::InvalidLayout
    );
    assert_eq!(
        ImageResource::rgb565(2, 1, 2, pixels.clone()).unwrap_err(),
        ResourceError::InvalidLayout
    );
    let image = ImageResource::rgb565(2, 1, 4, pixels.clone()).unwrap();
    drop(pixels);
    struct Page(Option<ImageResource>);
    impl App for Page {
        type Action = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            column((0..2).map(|index| {
                let node = match &self.0 {
                    Some(image) => rs_lvgl_ui::image_resource(image.clone()),
                    None => rs_lvgl_ui::image(ImageSource::rgb565(1, 1, &[0, 0])),
                };
                node.key(Key::Id(index))
            }))
        }
    }
    let mut app = Page(Some(image));
    let mut ui = Ui::new(&display, theme()).unwrap();
    ui.mount(&app).unwrap();
    let first = child(app_root(), 0);
    let second = child(app_root(), 1);
    // SAFETY: both are live image objects mounted by this UI.
    unsafe {
        assert_eq!(
            rs_lvgl_sys::lv_image_get_src(first.as_ptr()),
            rs_lvgl_sys::lv_image_get_src(second.as_ptr())
        );
    }
    let snapshot = ui.snapshot().unwrap();
    assert!(snapshot
        .pixels
        .chunks_exact(4)
        .any(|pixel| pixel == [255, 0, 0, 255]));
    assert!(snapshot
        .pixels
        .chunks_exact(4)
        .any(|pixel| pixel == [0, 255, 0, 255]));
    app.0 = None;
    assert!(
        weak.upgrade().is_some(),
        "mounted nodes retain the loaded pixels"
    );
    ui.update(&app).unwrap();
    assert_eq!(first, child(app_root(), 0));
    assert!(
        weak.upgrade().is_none(),
        "replacement releases data with live reused objects"
    );
    ui.snapshot().unwrap();
    ui.unmount();
}

static FONT_DELETIONS: AtomicUsize = AtomicUsize::new(0);
unsafe extern "C" fn destroy_font(font: *mut rs_lvgl_sys::lv_font_t) {
    FONT_DELETIONS.fetch_add(1, SeqCst);
    // SAFETY: the test allocated only the outer descriptor using lv_malloc;
    // its glyph tables refer to static Montserrat data and are not freed.
    unsafe { rs_lvgl_sys::lv_free(font.cast()) };
}
struct Backing(Rc<Cell<usize>>);
impl Drop for Backing {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

fn loaded_font(dropped: Rc<Cell<usize>>) -> FontResource {
    // SAFETY: LVGL is initialized on this serial UI thread. The allocation has
    // native alignment; copied glyph tables are immutable static data. The
    // supplied destructor matches this outer descriptor's allocator.
    unsafe {
        let raw = rs_lvgl_sys::lv_malloc(core::mem::size_of::<rs_lvgl_sys::lv_font_t>())
            .cast::<rs_lvgl_sys::lv_font_t>();
        let raw = core::ptr::NonNull::new(raw).unwrap();
        raw.as_ptr()
            .write(core::ptr::addr_of!(rs_lvgl_sys::lv_font_montserrat_14).read());
        FontResource::from_raw_owned(raw, Backing(dropped), destroy_font)
    }
}

#[test]
fn loaded_font_is_retained_for_inheritance_and_replacement() {
    let _guard = lock();
    let display = init();
    FONT_DELETIONS.store(0, SeqCst);
    let dropped = Rc::new(Cell::new(0));
    struct Page(Option<FontResource>);
    impl App for Page {
        type Action = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            let root = column([label("Loaded glyphs"), label("Shared font")]);
            match &self.0 {
                Some(font) => root.font_resource(font.clone()),
                None => root,
            }
        }
    }
    let mut app = Page(Some(loaded_font(dropped.clone())));
    let raw = app.0.as_ref().unwrap().as_ptr();
    let mut ui = Ui::new(&display, theme()).unwrap();
    ui.mount(&app).unwrap();
    let root = app_root();
    // SAFETY: the label is live and inherits its parent's installed font.
    unsafe {
        assert_eq!(
            rs_lvgl_sys::lv_obj_get_style_text_font(child(root, 0).as_ptr(), 0),
            raw
        );
    }
    let loaded_snapshot = ui.snapshot().unwrap();
    app.0 = None;
    assert_eq!(dropped.get(), 0);
    ui.update(&app).unwrap();
    assert_eq!(root, app_root());
    assert_eq!(FONT_DELETIONS.load(SeqCst), 1);
    assert_eq!(dropped.get(), 1);
    assert_eq!(
        ui.snapshot().unwrap().pixels,
        loaded_snapshot.pixels,
        "a loaded copy of Montserrat renders identically to the restored static font"
    );
    app.0 = Some(loaded_font(dropped.clone()));
    ui.update(&app).unwrap();
    app.0 = None;
    ui.unmount();
    assert_eq!(FONT_DELETIONS.load(SeqCst), 2);
    assert_eq!(dropped.get(), 2);
}

#[cfg(feature = "widget-image")]
#[test]
fn partial_creation_and_update_failures_release_loaded_resources() {
    use rs_lvgl_ui::{CustomObject, ImageResource};
    let _guard = lock();
    let display = init();
    struct Fails;
    impl CustomWidget for Fails {
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
    struct Page {
        image: ImageResource,
        font: FontResource,
        fail: bool,
    }
    impl App for Page {
        type Action = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            column([
                image_resource(self.image.clone()).key(Key::Id(1)),
                if self.fail {
                    custom(Fails, ()).key(Key::Id(2))
                } else {
                    label("valid").key(Key::Id(2))
                },
            ])
            .font_resource(self.font.clone())
        }
    }
    let mut ui = Ui::new(&display, theme()).unwrap();
    let drops = Rc::new(Cell::new(0));
    for fail_on_update in [false, true] {
        let pixels: Rc<[u8]> = Rc::from([0u8, 0]);
        let weak = Rc::downgrade(&pixels);
        let mut app = Page {
            image: ImageResource::rgb565(1, 1, 2, pixels).unwrap(),
            font: loaded_font(drops.clone()),
            fail: !fail_on_update,
        };
        if fail_on_update {
            ui.mount(&app).unwrap();
            // Change the source on a reused image, then fail a later sibling.
            let replacement: Rc<[u8]> = Rc::from([0xe0u8, 0x07]);
            app.image = ImageResource::rgb565(1, 1, 2, replacement).unwrap();
            app.fail = true;
            assert_eq!(ui.update(&app), Err(UiError::Allocation));
        } else {
            assert_eq!(ui.mount(&app), Err(UiError::Allocation));
        }
        drop(app);
        assert!(weak.upgrade().is_none());
        assert_eq!(ui.stats().mounted_nodes, 0);
        ui.unmount();
    }
    assert_eq!(drops.get(), 2);
}

#[cfg(feature = "widget-image")]
#[test]
fn rejected_native_image_source_reports_failure_and_allows_retry() {
    let _guard = lock();
    let display = init();
    struct Page(Option<rs_lvgl_ui::ImageResource>);
    impl App for Page {
        type Action = ();
        fn update(&mut self, _: ()) {}
        fn view(&self, _: &Theme) -> View<()> {
            match &self.0 {
                Some(source) => image_resource(source.clone()),
                // No driver is registered for this deliberately absent path.
                None => rs_lvgl_ui::image(ImageSource::Text(
                    cstr_core::CStr::from_bytes_with_nul(b"Z:/absent-resource.rgb565\0").unwrap(),
                )),
            }
        }
    }
    let pixels: Rc<[u8]> = Rc::from([0u8, 0]);
    let weak = Rc::downgrade(&pixels);
    let source = rs_lvgl_ui::ImageResource::rgb565(1, 1, 2, pixels).unwrap();
    let mut page = Page(Some(source));
    let mut ui = Ui::new(&display, theme()).unwrap();
    ui.mount(&page).unwrap();
    page.0 = None;
    assert!(weak.upgrade().is_some());
    assert_eq!(ui.update(&page), Err(UiError::InvalidProperty));
    assert!(weak.upgrade().is_none());
    assert_eq!(ui.stats().mounted_nodes, 0);
    assert_eq!(ui.mount(&page), Err(UiError::InvalidProperty));
    page.0 = Some(rs_lvgl_ui::ImageResource::rgb565(1, 1, 2, Rc::from([0u8, 0])).unwrap());
    ui.mount(&page).unwrap();
    ui.snapshot().unwrap();
    ui.unmount();
}
