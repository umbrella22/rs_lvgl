//! The native SDL backend may delete its display inside the timer handler.
use rs_lvgl_ui::prelude::*;

pub fn frame<A: App>(
    ui: &mut Ui<A::Msg>,
    app: &mut A,
    budget: usize,
) -> Result<Option<DrainStats>, UiError> {
    ui.timer_handler();
    // SDL can delete the sole native display while dispatching window events.
    // Check before draining: a queued message would reconcile stale objects.
    if unsafe { rs_lvgl_sys::lv_display_get_default() }.is_null() {
        return Ok(None);
    }
    ui.drain(app, budget).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rs_lvgl_ui::{Theme, View};

    #[test]
    fn closing_during_dispatch_does_not_drain_a_queued_update() {
        // This example test binary runs serially, like every native UI test.
        unsafe { rs_lvgl::deinit() };
        rs_lvgl::init();
        let display = rs_lvgl::Display::register::<_, 64>(8, 8, |_| {}).unwrap();
        struct AppState(usize);
        impl App for AppState {
            type Msg = ();
            fn update(&mut self, _: ()) {
                self.0 += 1;
            }
            fn view(&self, _: &Theme) -> View<()> {
                // A red oracle stops before rendering touches freed pointers.
                assert!(
                    !unsafe { rs_lvgl_sys::lv_display_get_default() }.is_null(),
                    "queued update rendered after native display deletion"
                );
                label("close regression")
            }
        }
        let mut app = AppState(0);
        let mut ui = Ui::new(&display, Theme::dark()).unwrap();
        ui.mount(&app).unwrap();
        ui.post(()).unwrap();
        unsafe extern "C" fn close(timer: *mut rs_lvgl_sys::lv_timer_t) {
            // SAFETY: single-threaded native dispatch; delete the sole display
            // exactly as SDL's close callback does, then remove this timer.
            unsafe {
                rs_lvgl_sys::lv_display_delete(rs_lvgl_sys::lv_display_get_default());
                rs_lvgl_sys::lv_timer_delete(timer);
            }
        }
        unsafe {
            let timer = rs_lvgl_sys::lv_timer_create(Some(close), 1, core::ptr::null_mut());
            rs_lvgl_sys::lv_timer_ready(timer);
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            frame(&mut ui, &mut app, 16)
        }));
        // External native deletion violates Ui's normal teardown ordering.
        // The examples' emergency exit intentionally must not drop stale roots.
        core::mem::forget(ui);
        assert!(unsafe { rs_lvgl_sys::lv_display_get_default() }.is_null());
        assert!(
            result.is_ok(),
            "frame must stop before the queued view update"
        );
        assert!(result.unwrap().unwrap().is_none());
        assert_eq!(app.0, 0);
    }
}
