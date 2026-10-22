//! Native screen transitions with retained declarative pages.
use crate::{App, Theme, Ui, UiError};
use alloc::vec::Vec;
use core::ptr::NonNull;
use rs_lvgl::{Display, NativeObject};

/// Direction and composition of a native screen transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ScreenEffect {
    None = rs_lvgl_sys::lv_screen_load_anim_t_LV_SCREEN_LOAD_ANIM_NONE,
    FadeIn = rs_lvgl_sys::lv_screen_load_anim_t_LV_SCREEN_LOAD_ANIM_FADE_IN,
    FadeOut = rs_lvgl_sys::lv_screen_load_anim_t_LV_SCREEN_LOAD_ANIM_FADE_OUT,
    MoveLeft = rs_lvgl_sys::lv_screen_load_anim_t_LV_SCREEN_LOAD_ANIM_MOVE_LEFT,
    MoveRight = rs_lvgl_sys::lv_screen_load_anim_t_LV_SCREEN_LOAD_ANIM_MOVE_RIGHT,
    MoveUp = rs_lvgl_sys::lv_screen_load_anim_t_LV_SCREEN_LOAD_ANIM_MOVE_TOP,
    MoveDown = rs_lvgl_sys::lv_screen_load_anim_t_LV_SCREEN_LOAD_ANIM_MOVE_BOTTOM,
    OverLeft = rs_lvgl_sys::lv_screen_load_anim_t_LV_SCREEN_LOAD_ANIM_OVER_LEFT,
    OverRight = rs_lvgl_sys::lv_screen_load_anim_t_LV_SCREEN_LOAD_ANIM_OVER_RIGHT,
    OverUp = rs_lvgl_sys::lv_screen_load_anim_t_LV_SCREEN_LOAD_ANIM_OVER_TOP,
    OverDown = rs_lvgl_sys::lv_screen_load_anim_t_LV_SCREEN_LOAD_ANIM_OVER_BOTTOM,
    OutLeft = rs_lvgl_sys::lv_screen_load_anim_t_LV_SCREEN_LOAD_ANIM_OUT_LEFT,
    OutRight = rs_lvgl_sys::lv_screen_load_anim_t_LV_SCREEN_LOAD_ANIM_OUT_RIGHT,
    OutUp = rs_lvgl_sys::lv_screen_load_anim_t_LV_SCREEN_LOAD_ANIM_OUT_TOP,
    OutDown = rs_lvgl_sys::lv_screen_load_anim_t_LV_SCREEN_LOAD_ANIM_OUT_BOTTOM,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScreenTransition {
    pub effect: ScreenEffect,
    pub duration_ms: u32,
}
impl ScreenTransition {
    pub const fn new(effect: ScreenEffect, duration_ms: u32) -> Self {
        Self {
            effect,
            duration_ms,
        }
    }
    pub const fn immediate() -> Self {
        Self::new(ScreenEffect::None, 0)
    }
}

struct Page<M: Clone + 'static> {
    ui: Ui<M>,
    screen: NonNull<rs_lvgl_sys::lv_obj_t>,
}

// One native registration per display prevents two safe controllers from
// retaining/deleting each other's original screen. The marker has static
// storage; native callbacks retain no Rust allocation or borrowed controller.
static CONTROLLER_MARKER: u8 = 0;
unsafe extern "C" fn controller_marker(_: *mut rs_lvgl_sys::lv_event_t) {}

struct DisplayClaim {
    display: NonNull<rs_lvgl_sys::lv_display_t>,
    event: NonNull<rs_lvgl_sys::lv_event_dsc_t>,
}
impl DisplayClaim {
    fn new(display: NonNull<rs_lvgl_sys::lv_display_t>) -> Result<Self, UiError> {
        let marker = (&CONTROLLER_MARKER as *const u8).cast_mut().cast();
        // SAFETY: all access occurs on the exclusive UI thread, and display
        // remains live until the controller releases its claim.
        unsafe {
            for index in 0..rs_lvgl_sys::lv_display_get_event_count(display.as_ptr()) {
                let event = rs_lvgl_sys::lv_display_get_event_dsc(display.as_ptr(), index);
                if rs_lvgl_sys::lv_event_dsc_get_user_data(event) == marker {
                    return Err(UiError::InvalidProperty);
                }
            }
            let event = NonNull::new(rs_lvgl_sys::lv_display_add_event_cb(
                display.as_ptr(),
                Some(controller_marker),
                rs_lvgl_sys::lv_event_code_t_LV_EVENT_DELETE,
                marker,
            ))
            .ok_or(UiError::Allocation)?;
            Ok(Self { display, event })
        }
    }
}
impl Drop for DisplayClaim {
    fn drop(&mut self) {
        // SAFETY: the controller releases this registration before native
        // display deletion. Other display callbacks are preserved.
        unsafe {
            for index in 0..rs_lvgl_sys::lv_display_get_event_count(self.display.as_ptr()) {
                if rs_lvgl_sys::lv_display_get_event_dsc(self.display.as_ptr(), index)
                    == self.event.as_ptr()
                {
                    rs_lvgl_sys::lv_display_remove_event(self.display.as_ptr(), index);
                    break;
                }
            }
        }
    }
}
impl<M: Clone + 'static> Drop for Page<M> {
    fn drop(&mut self) {
        self.ui.unmount();
        // SAFETY: collection only deletes screens no longer referenced by the
        // display; Ui has released its children, overlays and input group.
        unsafe { rs_lvgl_sys::lv_obj_delete(self.screen.as_ptr()) };
    }
}

/// One exclusive screen controller for a display.
///
/// `show` mounts a fresh Ui and retains outgoing pages until LVGL stops using
/// them. Rapid transitions interrupt the previous transition safely. Native
/// input is suspended during the transition. Outgoing overlays are hidden and
/// input groups detached; bind keypad/encoder devices to `current_mut()` after
/// each show. Application state stays in the caller's App.
/// A second controller on the same display returns `InvalidProperty` until
/// the first is unmounted or dropped. `show` reacquires an unmounted claim.
///
/// The display, its original screen and LVGL must remain live until `unmount`
/// or Drop. Do not load/delete these screens through another controller or raw
/// API. The original screen is restored on unmount and is never deleted here.
pub struct Screens<M: Clone + 'static> {
    display: NonNull<rs_lvgl_sys::lv_display_t>,
    original: NonNull<rs_lvgl_sys::lv_obj_t>,
    theme: Theme,
    current: Option<Page<M>>,
    retired: Vec<Page<M>>,
    claim: Option<DisplayClaim>,
}
impl<M: Clone + 'static> Screens<M> {
    pub fn new(display: &Display, theme: Theme) -> Result<Self, UiError> {
        let original = display
            .get_scr_act()
            .map_err(|_| UiError::DisplayUnavailable)?
            .raw();
        // SAFETY: the Display supplies a live active screen.
        let display = NonNull::new(unsafe { rs_lvgl_sys::lv_obj_get_display(original.as_ptr()) })
            .ok_or(UiError::DisplayUnavailable)?;
        let claim = Some(DisplayClaim::new(display)?);
        Ok(Self {
            display,
            original,
            theme,
            current: None,
            retired: Vec::new(),
            claim,
        })
    }

    /// Mount and display a new page. Validation/mount failure preserves the
    /// previous page. Timing is limited by LVGL's signed animation clock.
    pub fn show<A: App<Msg = M>>(
        &mut self,
        app: &A,
        transition: ScreenTransition,
    ) -> Result<(), UiError> {
        if transition.duration_ms > i32::MAX as u32 {
            return Err(UiError::InvalidProperty);
        }
        if self.claim.is_none() {
            self.claim = Some(DisplayClaim::new(self.display)?);
        }
        // SAFETY: native state is exclusive to this UI thread. Restore the
        // default display immediately, including on native allocation failure.
        let screen = unsafe {
            let previous = rs_lvgl_sys::lv_display_get_default();
            rs_lvgl_sys::lv_display_set_default(self.display.as_ptr());
            let screen = rs_lvgl_sys::lv_obj_create(core::ptr::null_mut());
            rs_lvgl_sys::lv_display_set_default(previous);
            NonNull::new(screen).ok_or(UiError::Allocation)?
        };
        let ui = match Ui::on_screen(screen, self.theme, crate::app::DEFAULT_QUEUE_CAPACITY) {
            Ok(ui) => ui,
            Err(error) => {
                // SAFETY: failed initialization left no mounted UI.
                unsafe { rs_lvgl_sys::lv_obj_delete(screen.as_ptr()) };
                return Err(error);
            }
        };
        let mut page = Page { ui, screen };
        page.ui.mount(app)?;
        if let Some(old) = self.current.as_mut() {
            old.ui.deactivate();
        }
        // SAFETY: both pages and the original remain live. auto_del is false;
        // only this controller deletes native screens after collecting them.
        unsafe {
            rs_lvgl_sys::lv_screen_load_anim(
                screen.as_ptr(),
                transition.effect as _,
                transition.duration_ms,
                0,
                false,
            );
            rs_lvgl_sys::lv_anim_refr_now();
        }
        if let Some(old) = self.current.replace(page) {
            self.retired.push(old);
        }
        self.collect();
        Ok(())
    }

    pub fn current(&self) -> Option<&Ui<M>> {
        self.current.as_ref().map(|page| &page.ui)
    }
    pub fn current_mut(&mut self) -> Option<&mut Ui<M>> {
        self.current.as_mut().map(|page| &mut page.ui)
    }

    /// Number of outgoing pages still required by the native display.
    pub fn retiring_pages(&self) -> usize {
        self.retired.len()
    }

    /// Dispatch native timers and release completed outgoing pages.
    pub fn timer_handler(&mut self) -> u32 {
        let wait = self
            .current
            .as_mut()
            .map(|page| page.ui.timer_handler())
            .unwrap_or(0);
        self.collect();
        wait
    }

    pub fn set_theme(&mut self, theme: Theme) {
        if let Some(page) = self.current.as_mut() {
            page.ui.set_theme(theme);
        }
        self.theme = theme;
    }

    /// Restore the original screen, cancel transitions, and release pages.
    /// An already unmounted controller can be dropped after native teardown.
    pub fn unmount(&mut self) {
        if self.current.is_none() && self.retired.is_empty() {
            self.claim = None;
            return;
        }
        // SAFETY: the original screen remains live by the controller contract.
        // Loading it cancels the in-flight transition before pages are freed.
        unsafe { rs_lvgl_sys::lv_screen_load(self.original.as_ptr()) };
        self.current = None;
        self.retired.clear();
        self.claim = None;
    }

    fn collect(&mut self) {
        if self.retired.is_empty() {
            return;
        }
        // SAFETY: the controller's display is live; all three native references
        // must stop referring to a screen before its Rust/native state drops.
        unsafe {
            let active = rs_lvgl_sys::lv_display_get_screen_active(self.display.as_ptr());
            let previous = rs_lvgl_sys::lv_display_get_screen_prev(self.display.as_ptr());
            let loading = rs_lvgl_sys::lv_display_get_screen_loading(self.display.as_ptr());
            self.retired
                .retain(|page| [active, previous, loading].contains(&page.screen.as_ptr()));
        }
    }
}
impl<M: Clone + 'static> Drop for Screens<M> {
    fn drop(&mut self) {
        self.unmount();
    }
}
