//! Native-only samples, bounded by the mounted custom node's lifetime.
use super::*;
use rs_lvgl::NativeObject;

pub(super) fn view(app: &Studio, t: &Theme) -> View<Msg> {
    let mut samples = Vec::new();
    for (kind, title, caption) in [
        (0, "Native list", "Select a sample action"),
        (1, "Native menu", "Open Display, then use Back"),
        (2, "Native window", "Header / content / action"),
    ] {
        // SAFETY: each factory returns a direct child of the supplied parent.
        // All descendants and native menu callbacks are owned by that subtree;
        // strings are static or copied by LVGL. No pointers escape the factory.
        let sample = unsafe { raw(move |parent| {
            let obj = match kind {
                0 => rs_lvgl_sys::lv_list_create(parent.as_ptr()),
                1 => rs_lvgl_sys::lv_menu_create(parent.as_ptr()),
                _ => rs_lvgl_sys::lv_win_create(parent.as_ptr()),
            };
            let object = NonNull::new(obj)?;
            rs_lvgl_sys::lv_obj_set_size(obj, rs_lvgl_sys::lv_pct(100), 306);
            match kind {
                0 => {
                    rs_lvgl_sys::lv_list_add_text(obj, c"STUDIO COLLECTION".as_ptr().cast());
                    for name in [c"New workspace", c"Open a sample", c"Save a preset", c"Export a snapshot"] {
                        rs_lvgl_sys::lv_list_add_button(obj, core::ptr::null(), name.as_ptr().cast());
                    }
                }
                1 => {
                    let page = rs_lvgl_sys::lv_menu_page_create(obj, c"Device".as_ptr().cast());
                    let details = rs_lvgl_sys::lv_menu_page_create(obj, c"Display".as_ptr().cast());
                    let settings = rs_lvgl_sys::lv_menu_cont_create(page);
                    let label = rs_lvgl_sys::lv_label_create(settings);
                    rs_lvgl_sys::lv_label_set_text(label, c"Display  >".as_ptr().cast());
                    let label = rs_lvgl_sys::lv_label_create(details);
                    rs_lvgl_sys::lv_label_set_text(label, c"1280 x 860\nRGB color\nNative rendering".as_ptr().cast());
                    rs_lvgl_sys::lv_menu_set_load_page_event(obj, settings, details);
                    rs_lvgl_sys::lv_menu_set_page(obj, page);
                }
                _ => {
                    rs_lvgl_sys::lv_win_add_title(obj, c"Sample device".as_ptr().cast());
                    rs_lvgl_sys::lv_win_add_button(obj, c"\u{f067}".as_ptr().cast(), 42);
                    let label = rs_lvgl_sys::lv_label_create(rs_lvgl_sys::lv_win_get_content(obj));
                    rs_lvgl_sys::lv_label_set_text(label, c"A native window.\n\nComposed inside a Rust view.\n\nPress the header action.".as_ptr().cast());
                }
            }
            enable_bubbling(obj);
            Some(object)
        }) }.key(Key::Id(700 + kind + if app.dark { 10 } else { 0 }))
    ;
        samples.push(
            card(title, caption, t)
                .style(
                    Style::default()
                        .width(Size::Percent(32))
                        .height(Size::Px(430)),
                )
                .child(sample)
                .on_bubbled_event(|event| {
                    matches!(
                        event,
                        InputEvent::Other(Event::Clicked)
                            | InputEvent::Pointer(PointerEvent {
                                event: Event::Clicked,
                                ..
                            })
                    )
                    .then_some(Msg::Click)
                }),
        );
    }
    studio_column().child(studio_row().children(samples))
        .child(card("One native provider. Three ways to build.", "Typed views / imperative widgets / raw C bindings",t).style(Style::default().height(Size::Px(128)))
            .child(text(&format!("{} native actions received. Native samples keep their children and callbacks inside one mounted subtree.",app.clicks),t)
                .style(Style::default().width(Size::Percent(100)))))
}

#[derive(Default)]
pub(super) struct Pulse {
    elapsed: u32,
}
impl CustomWidget for Pulse {
    fn create(&mut self, parent: &mut rs_lvgl::Obj<'_>) -> Result<CustomObject, UiError> {
        // SAFETY: the reconciler supplies a live parent. The new direct child
        // is returned to the reconciler; this widget retains no raw pointer.
        unsafe {
            let raw = NonNull::new(rs_lvgl_sys::lv_obj_create(parent.raw().as_ptr()))
                .ok_or(UiError::Allocation)?;
            rs_lvgl_sys::lv_obj_set_size(raw.as_ptr(), 52, 52);
            rs_lvgl_sys::lv_obj_set_style_radius(raw.as_ptr(), 26, 0);
            rs_lvgl_sys::lv_obj_set_style_border_width(raw.as_ptr(), 0, 0);
            rs_lvgl_sys::lv_obj_set_style_bg_color(
                raw.as_ptr(),
                rs_lvgl_sys::lv_color_t {
                    red: 26,
                    green: 145,
                    blue: 128,
                },
                0,
            );
            CustomObject::new(parent, raw)
        }
    }
    fn update(&mut self, object: &mut CustomObject, elapsed_ms: u32) -> Result<(), UiError> {
        self.elapsed = self.elapsed.wrapping_add(elapsed_ms);
        let phase = (self.elapsed % 1600) as f32 / 1600.0 * std::f32::consts::TAU;
        let opacity = (185.0 + 70.0 * phase.sin()) as u8;
        // SAFETY: the handle is borrowed for this hook and stays on the UI thread.
        unsafe {
            rs_lvgl_sys::lv_obj_set_style_opa(object.raw().as_ptr(), opacity, 0);
        }
        Ok(())
    }
    fn delete(&mut self, _: &mut CustomObject) {
        self.elapsed = 0;
    }
}

/// Enable event forwarding within a newly created subtree.
///
/// # Safety
/// The caller must supply a live subtree on the LVGL thread, with no concurrent
/// deletion. No pointer escapes; recursion touches only current descendants.
unsafe fn enable_bubbling(root: *mut rs_lvgl_sys::lv_obj_t) {
    unsafe {
        rs_lvgl_sys::lv_obj_set_event_bubble(root, true);
        for i in 0..rs_lvgl_sys::lv_obj_get_child_count(root) {
            enable_bubbling(rs_lvgl_sys::lv_obj_get_child(root, i as i32));
        }
    }
}
