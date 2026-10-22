use crate::functions::CoreError;
use crate::Box;
use crate::Color;
use crate::Screen;
use crate::{disp_get_default, get_str_act, NativeObject};
#[cfg(feature = "nightly")]
use core::error::Error;
use core::fmt;
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::ptr::NonNull;
use core::{ptr, result};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DisplayError {
    NotAvailable,
    FailedToRegister,
    NotRegistered,
}

impl fmt::Display for DisplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Display {}",
            match self {
                DisplayError::NotAvailable => "not available",
                DisplayError::FailedToRegister => "failed to register",
                DisplayError::NotRegistered => "not registered",
            }
        )
    }
}

#[cfg(feature = "nightly")]
impl Error for DisplayError {}

type Result<T> = result::Result<T, DisplayError>;

/// A handle to a display registered with LVGL.
///
/// LVGL owns the registration, not this handle: dropping it does not remove the
/// display or invalidate handles obtained through [`Display::default`] or
/// widgets created on the default screen. The buffer and callback live until
/// the native display is deleted, for example by [`Display::delete`] or `deinit`.
/// Raw/native backends that delete displays must also stop using their Rust
/// handles and associated objects before deletion.
pub struct Display {
    pub(crate) disp: NonNull<rs_lvgl_sys::lv_display_t>,
}

impl Display {
    pub(crate) fn from_raw(disp: NonNull<rs_lvgl_sys::lv_display_t>) -> Self {
        Self { disp }
    }

    /// Register a display with room for at most `N` pixels per refresh.
    ///
    /// The Rust callback receives RGB888 [`Color`] values. This display therefore
    /// renders in RGB888, independently of LVGL's configured default format.
    /// `N` must provide enough bytes for at least one aligned display row.
    pub fn register<F, const N: usize>(
        hor_res: u32,
        ver_res: u32,
        display_update: F,
    ) -> Result<Self>
    where
        F: FnMut(&DisplayRefresh<N>) + 'static,
    {
        let driver = DisplayDriver::<N>::new(
            hor_res,
            ver_res,
            Some(rs_lvgl_sys::lv_color_format_t_LV_COLOR_FORMAT_RGB888),
        )?;
        let display = driver.into_display(display_update, None)?;
        unsafe {
            rs_lvgl_sys::lv_display_set_flush_cb(
                display.disp.as_ptr(),
                Some(disp_flush_trampoline::<F, N>),
            );
        }
        Ok(display)
    }

    pub fn get_scr_act(&self) -> Result<Screen<'_>> {
        Ok(get_str_act(Some(self))?.try_into()?)
    }

    pub fn set_scr_act(&self, screen: &mut Screen) {
        let scr_ptr = unsafe { screen.raw().as_mut() };
        unsafe { rs_lvgl_sys::lv_screen_load(scr_ptr) }
    }

    /// Register a raw display using LVGL's configured default color format.
    ///
    /// The buffer contains `N * size_of::<lv_color_t>()` **bytes**. In a format
    /// other than RGB888, `N` is not the number of pixels in a flush. The optional
    /// cleanup hook runs when LVGL deletes the display, not when a handle drops.
    ///
    /// # Safety
    ///
    /// The flush callback must honor the display's actual color format, area and
    /// stride, and call `lv_display_flush_ready` after finishing each transfer.
    /// Both callbacks and any data they access must remain valid until deletion;
    /// neither callback may unwind. The cleanup hook must be safe to run during
    /// LVGL's display-delete event, before its screens have been destroyed,
    /// without refreshing, using, or recursively deleting the display. All
    /// asynchronous rendering and transfers must finish before deletion.
    pub unsafe fn register_raw<const N: usize>(
        hor_res: u32,
        ver_res: u32,
        flush_cb: Option<
            unsafe extern "C" fn(
                *mut rs_lvgl_sys::lv_display_t,
                *const rs_lvgl_sys::lv_area_t,
                *mut u8,
            ),
        >,
        drop: Option<unsafe extern "C" fn()>,
    ) -> Result<Self> {
        let display = DisplayDriver::<N>::new(hor_res, ver_res, None)?.into_display((), drop)?;
        unsafe { rs_lvgl_sys::lv_display_set_flush_cb(display.disp.as_ptr(), flush_cb) };
        Ok(display)
    }

    /// Delete the native display and release its registered buffer and callback.
    ///
    /// # Safety
    ///
    /// No other display alias, screen, widget, animation or input callback may
    /// subsequently access this display or its objects. All rendering, including
    /// asynchronous draw workers, GPU/DMA work and pending flushes, must finish
    /// before deletion. No render, flush, event or input callback for the display
    /// may be in progress. Destructors and deletion callbacks must not refresh,
    /// use, or recursively delete this display, or call `deinit` during deletion.
    /// This includes handles created implicitly on the default display. Like
    /// `deinit`, deletion cannot be safe while those handles are not tied to this
    /// handle's lifetime.
    pub unsafe fn delete(self) {
        unsafe { rs_lvgl_sys::lv_display_delete(self.disp.as_ptr()) };
    }
}

impl Default for Display {
    fn default() -> Self {
        disp_get_default().expect("LVGL must be INITIALIZED")
    }
}

pub(crate) fn get_scr_act() -> Result<Screen<'static>> {
    Ok(get_str_act(None)?.try_into()?)
}

pub struct DrawBuffer<const N: usize> {
    _refresh_buffer: Pin<Box<[MaybeUninit<rs_lvgl_sys::lv_color_t>; N]>>,
}

impl<const N: usize> Default for DrawBuffer<N> {
    fn default() -> Self {
        let buf = Box::pin([MaybeUninit::uninit(); N]);
        Self {
            _refresh_buffer: buf,
        }
    }
}

struct DisplayDriver<const N: usize> {
    disp: NonNull<rs_lvgl_sys::lv_display_t>,
    buffer: DrawBuffer<N>,
}

// The native display owns this state. The delete event releases it even if all
// Rust handles have already gone away, including when lv_deinit deletes displays.
struct DisplayState<F, const N: usize> {
    callback: F,
    _buffer: DrawBuffer<N>,
    drop_hook: Option<unsafe extern "C" fn()>,
}

impl<F, const N: usize> Drop for DisplayState<F, N> {
    fn drop(&mut self) {
        if let Some(drop_hook) = self.drop_hook {
            unsafe { drop_hook() };
        }
    }
}

impl<const N: usize> DisplayDriver<N> {
    fn new(
        hor_res: u32,
        ver_res: u32,
        color_format: Option<rs_lvgl_sys::lv_color_format_t>,
    ) -> Result<Self> {
        let width = i32::try_from(hor_res).map_err(|_| DisplayError::FailedToRegister)?;
        let height = i32::try_from(ver_res).map_err(|_| DisplayError::FailedToRegister)?;
        let size = N
            .checked_mul(core::mem::size_of::<rs_lvgl_sys::lv_color_t>())
            .and_then(|bytes| u32::try_from(bytes).ok())
            .ok_or(DisplayError::FailedToRegister)?;
        if width == 0 || height == 0 || size == 0 {
            return Err(DisplayError::FailedToRegister);
        }

        let disp = NonNull::new(unsafe { rs_lvgl_sys::lv_display_create(width, height) })
            .ok_or(DisplayError::FailedToRegister)?;
        unsafe {
            if let Some(format) = color_format {
                rs_lvgl_sys::lv_display_set_color_format(disp.as_ptr(), format);
            }
            let format = rs_lvgl_sys::lv_display_get_color_format(disp.as_ptr());
            let stride = rs_lvgl_sys::lv_draw_buf_width_to_stride(hor_res, format);
            if stride == 0 || size < stride {
                rs_lvgl_sys::lv_display_delete(disp.as_ptr());
                return Err(DisplayError::FailedToRegister);
            }
        }

        let mut buffer = DrawBuffer::default();
        unsafe {
            let buf_ptr = buffer._refresh_buffer.as_mut().get_mut() as *mut _ as *mut cty::c_void;
            rs_lvgl_sys::lv_display_set_buffers(
                disp.as_ptr(),
                buf_ptr,
                ptr::null_mut(),
                size,
                rs_lvgl_sys::lv_display_render_mode_t_LV_DISPLAY_RENDER_MODE_PARTIAL,
            );
        }
        Ok(Self { disp, buffer })
    }

    fn into_display<F>(
        self,
        callback: F,
        drop_hook: Option<unsafe extern "C" fn()>,
    ) -> Result<Display> {
        let state = Box::into_raw(Box::new(DisplayState {
            callback,
            _buffer: self.buffer,
            drop_hook,
        }));
        unsafe {
            let event = rs_lvgl_sys::lv_display_add_event_cb(
                self.disp.as_ptr(),
                Some(drop_display_state::<F, N>),
                rs_lvgl_sys::lv_event_code_t_LV_EVENT_DELETE,
                state.cast(),
            );
            if event.is_null() {
                rs_lvgl_sys::lv_display_delete(self.disp.as_ptr());
                drop(Box::from_raw(state));
                return Err(DisplayError::FailedToRegister);
            }
            rs_lvgl_sys::lv_display_set_user_data(self.disp.as_ptr(), state.cast());
        }
        Ok(Display::from_raw(self.disp))
    }
}

unsafe extern "C" fn drop_display_state<F, const N: usize>(event: *mut rs_lvgl_sys::lv_event_t) {
    let state = unsafe { rs_lvgl_sys::lv_event_get_user_data(event) };
    unsafe { drop(Box::from_raw(state as *mut DisplayState<F, N>)) };
}

pub struct Area {
    pub x1: i32,
    pub x2: i32,
    pub y1: i32,
    pub y2: i32,
}

pub struct DisplayRefresh<const N: usize> {
    pub area: Area,
    /// Row-major RGB888 colors for `area`, without stride padding. Only the
    /// first `(x2 - x1 + 1) * (y2 - y1 + 1)` entries belong to this refresh;
    /// the remaining entries are initialized to the default color.
    pub colors: [Color; N],
}

#[cfg(feature = "embedded_graphics")]
mod embedded_graphics_impl {
    use crate::{Color, DisplayRefresh};
    use embedded_graphics::prelude::*;
    use embedded_graphics::Pixel;

    impl<const N: usize> DisplayRefresh<N> {
        pub fn as_pixels<C>(&self) -> impl IntoIterator<Item = Pixel<C>> + '_
        where
            C: PixelColor + From<Color>,
        {
            let area = &self.area;
            let x1 = area.x1;
            let x2 = area.x2;
            let y1 = area.y1;
            let y2 = area.y2;

            let ys = y1..=y2;
            let xs = (x1..=x2).enumerate();
            let x_len = (x2 - x1 + 1) as usize;

            ys.enumerate().flat_map(move |(iy, y)| {
                xs.clone().map(move |(ix, x)| {
                    let color_len = x_len * iy + ix;
                    let raw_color = self.colors[color_len];
                    Pixel(Point::new(x as i32, y as i32), raw_color.into())
                })
            })
        }
    }
}

unsafe extern "C" fn disp_flush_trampoline<F, const N: usize>(
    disp: *mut rs_lvgl_sys::lv_display_t,
    area: *const rs_lvgl_sys::lv_area_t,
    px_map: *mut u8,
) where
    F: FnMut(&DisplayRefresh<N>),
{
    let user_data = unsafe { rs_lvgl_sys::lv_display_get_user_data(disp) };
    if !user_data.is_null() {
        let state = unsafe { &mut *(user_data as *mut DisplayState<F, N>) };
        let area = unsafe { &*area };
        let active = unsafe { rs_lvgl_sys::lv_display_get_buf_active(disp) };
        let stride = unsafe { (*active).header.stride() } as usize;
        let area = Area {
            x1: area.x1,
            x2: area.x2,
            y1: area.y1,
            y2: area.y2,
        };
        let colors = unsafe { read_rgb888_refresh::<N>(px_map, &area, stride) };
        let update = DisplayRefresh { area, colors };
        let callback = &mut state.callback;
        callback(&update);
    }
    rs_lvgl_sys::lv_display_flush_ready(disp);
}

// Only the area pixels are initialized by LVGL; the remainder of the backing
// buffer (and any row padding) must not be read as colors.
unsafe fn read_rgb888_refresh<const N: usize>(
    px_map: *const u8,
    area: &Area,
    stride: usize,
) -> [Color; N] {
    let width = (area.x2 - area.x1 + 1) as usize;
    let height = (area.y2 - area.y1 + 1) as usize;
    let mut colors = [Color::default(); N];
    for (y, row) in colors[..width * height].chunks_mut(width).enumerate() {
        for (x, color) in row.iter_mut().enumerate() {
            let offset = y * stride + x * core::mem::size_of::<rs_lvgl_sys::lv_color_t>();
            let raw = unsafe {
                ptr::read_unaligned(px_map.add(offset).cast::<rs_lvgl_sys::lv_color_t>())
            };
            *color = Color::from_raw(raw);
        }
    }
    colors
}

impl From<CoreError> for DisplayError {
    fn from(err: CoreError) -> Self {
        use DisplayError::*;
        match err {
            CoreError::ResourceNotAvailable => NotAvailable,
            CoreError::OperationFailed => NotAvailable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests;

    #[test]
    fn refresh_decoding_ignores_padding_and_unused_capacity() {
        // BGR bytes: red/green, padding, blue/white. The LVGL buffer may have
        // fewer initialized pixels than the Rust callback's fixed capacity.
        let pixels = [0, 0, 255, 0, 255, 0, 99, 99, 255, 0, 0, 255, 255, 255];
        let area = Area {
            x1: 3,
            x2: 4,
            y1: 5,
            y2: 6,
        };
        let colors = unsafe { read_rgb888_refresh::<8>(pixels.as_ptr(), &area, 8) };
        let rgb: std::vec::Vec<_> = colors.iter().map(|c| (c.r(), c.g(), c.b())).collect();
        assert_eq!(
            rgb,
            [
                (255, 0, 0),
                (0, 255, 0),
                (0, 0, 255),
                (255, 255, 255),
                (0, 0, 0),
                (0, 0, 0),
                (0, 0, 0),
                (0, 0, 0)
            ]
        );
    }

    #[test]
    fn registered_display_refreshes_with_matching_format_and_capacity() {
        use std::{cell::RefCell, rc::Rc};
        tests::initialize_test(false);
        let captured = Rc::new(RefCell::new(std::vec::Vec::new()));
        let capture = captured.clone();
        let display = Display::register::<_, 40>(10, 10, move |refresh| {
            let count = ((refresh.area.x2 - refresh.area.x1 + 1)
                * (refresh.area.y2 - refresh.area.y1 + 1)) as usize;
            let all_red = refresh.colors[..count]
                .iter()
                .all(|color| (color.r(), color.g(), color.b()) == (255, 0, 0));
            #[cfg(feature = "embedded_graphics")]
            let pixels = refresh
                .as_pixels::<embedded_graphics::pixelcolor::Rgb888>()
                .into_iter()
                .count();
            #[cfg(not(feature = "embedded_graphics"))]
            let pixels = count;
            capture.borrow_mut().push((count, all_red, pixels));
        })
        .unwrap();
        unsafe {
            assert_eq!(
                rs_lvgl_sys::lv_display_get_color_format(display.disp.as_ptr()),
                rs_lvgl_sys::lv_color_format_t_LV_COLOR_FORMAT_RGB888
            );
            let screen = rs_lvgl_sys::lv_display_get_screen_active(display.disp.as_ptr());
            rs_lvgl_sys::lv_obj_set_style_bg_color(screen, Color::from_rgb((255, 0, 0)).raw, 0);
            rs_lvgl_sys::lv_obj_set_style_bg_opa(screen, 255, 0);
            rs_lvgl_sys::lv_refr_now(display.disp.as_ptr());
        }
        let refreshes = captured.borrow();
        assert!(!refreshes.is_empty());
        assert_eq!(refreshes.iter().map(|r| r.0).sum::<usize>(), 100);
        assert!(refreshes
            .iter()
            .all(|&(count, red, pixels)| count <= 40 && red && pixels == count));
        assert!(refreshes.iter().any(|r| r.0 < 40));
        // No screen/widget handles or transfers survive deletion.
        unsafe { display.delete() };
        assert_eq!(Rc::strong_count(&captured), 1, "callback must be dropped");
    }

    #[test]
    fn dropping_handle_keeps_aliases_and_implicit_widgets_alive() {
        use crate::{Obj, Widget};
        use std::{cell::Cell, rc::Rc};
        tests::initialize_test(false);
        let calls = Rc::new(Cell::new(0));
        let callback_calls = calls.clone();
        let owned = Display::register::<_, 100>(10, 10, move |_| {
            callback_calls.set(callback_calls.get() + 1);
        })
        .unwrap();
        let alias = Display::default();
        let original_screen = alias.get_scr_act().unwrap().raw();
        let mut label = crate::widgets::Label::new().unwrap();
        let mut object = Obj::new().unwrap();
        let mut blank_screen = Screen::blank().unwrap();
        drop(owned);
        let replacement = Display::register::<_, 100>(10, 10, |_| {}).unwrap();
        assert_eq!(alias.get_scr_act().unwrap().raw(), original_screen);
        assert_ne!(replacement.get_scr_act().unwrap().raw(), original_screen);
        label.set_text(cstr_core::CStr::from_bytes_with_nul(b"alive\0").unwrap());
        object.set_size(2, 2);
        blank_screen.set_size(10, 10);
        unsafe { rs_lvgl_sys::lv_refr_now(alias.disp.as_ptr()) };
        assert!(calls.get() > 0);
        assert_eq!(Rc::strong_count(&calls), 2);
        drop((label, object, blank_screen));
        unsafe {
            alias.delete();
            replacement.delete();
        }
        assert_eq!(Rc::strong_count(&calls), 1);
    }

    #[cfg(not(feature = "custom_allocator"))]
    #[test]
    fn deinit_releases_callback_after_all_display_handles_are_dropped() {
        use std::rc::Rc;
        tests::initialize_test(false);
        let owned_data = Rc::new(());
        let captured_data = owned_data.clone();
        let display = Display::register::<_, 100>(10, 10, move |_| {
            let _ = &captured_data;
        })
        .unwrap();
        drop(display);
        assert_eq!(Rc::strong_count(&owned_data), 2);
        unsafe { crate::deinit() };
        assert_eq!(Rc::strong_count(&owned_data), 1);
        crate::init();
    }

    #[test]
    fn raw_cleanup_runs_once_when_native_display_is_deleted() {
        use core::sync::atomic::{AtomicUsize, Ordering};
        static DROPS: AtomicUsize = AtomicUsize::new(0);
        unsafe extern "C" fn cleanup() {
            DROPS.fetch_add(1, Ordering::Relaxed);
        }
        tests::initialize_test(false);
        DROPS.store(0, Ordering::Relaxed);
        let display = unsafe { Display::register_raw::<40>(10, 10, None, Some(cleanup)) }.unwrap();
        let alias = Display::default();
        drop(display);
        assert_eq!(DROPS.load(Ordering::Relaxed), 0);
        unsafe { alias.delete() };
        assert_eq!(DROPS.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn buffer_must_fit_an_aligned_display_row() {
        tests::initialize_test(false);
        assert!(Display::register::<_, 0>(10, 10, |_| {}).is_err());
        assert!(Display::register::<_, 1>(10, 10, |_| {}).is_err());
        assert!(Display::register::<_, 40>(0, 10, |_| {}).is_err());
        assert!(unsafe { rs_lvgl_sys::lv_display_get_default() }.is_null());
    }

    #[test]
    fn get_scr_act_return_display() {
        tests::initialize_test(true);
        let _screen = get_str_act(None).expect("We can get the active screen");
    }

    #[test]
    fn get_default_display() {
        tests::initialize_test(true);
        let display = Display::default();
        let _screen_direct = display
            .get_scr_act()
            .expect("Return screen directly from the display instance");
        let _screen_default = get_scr_act().expect("Return screen from the default display");
    }

    #[test]
    fn register_display_directly() -> Result<()> {
        crate::tests::initialize_test(true);
        let display = Display::default();
        let _screen = display
            .get_scr_act()
            .expect("Return screen directly from the display instance");
        Ok(())
    }
}
