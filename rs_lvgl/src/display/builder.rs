use super::{
    alignment::InvalidationAlignment, Area, BufferInfo, BufferLayout, ColorFormat, Display,
    DisplayConfigError, PixelBuffer,
};
use crate::{sys, Box};
use core::{
    cell::{Cell, RefCell, UnsafeCell},
    ptr::NonNull,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderMode {
    Partial,
    Direct,
    Full,
}

impl RenderMode {
    fn to_raw(self) -> sys::lv_display_render_mode_t {
        match self {
            Self::Partial => sys::lv_display_render_mode_t_LV_DISPLAY_RENDER_MODE_PARTIAL,
            Self::Direct => sys::lv_display_render_mode_t_LV_DISPLAY_RENDER_MODE_DIRECT,
            Self::Full => sys::lv_display_render_mode_t_LV_DISPLAY_RENDER_MODE_FULL,
        }
    }
}

/// Buffer count is independent of the render mode. Both buffers use one layout.
pub enum DisplayBuffers {
    Single(PixelBuffer),
    Double {
        front: PixelBuffer,
        back: PixelBuffer,
    },
}

impl DisplayBuffers {
    fn capacity(&self) -> usize {
        match self {
            Self::Single(a) => a.len(),
            Self::Double { front, back } => front.len().min(back.len()),
        }
    }

    fn pointers(&mut self, layout: BufferLayout) -> Result<(*mut u8, *mut u8), DisplayConfigError> {
        match self {
            Self::Single(a) => {
                layout.validate(a)?;
                Ok((a.as_mut_ptr(), core::ptr::null_mut()))
            }
            Self::Double { front, back } => {
                layout.validate(front)?;
                layout.validate(back)?;
                Ok((front.as_mut_ptr(), back.as_mut_ptr()))
            }
        }
    }
}

/// Metadata for one native flush. PARTIAL starts at `origin`; DIRECT/FULL
/// references the full framebuffer. Row padding is never part of the dirty width.
pub struct FlushInfo {
    pub area: Area,
    buffer: BufferInfo,
    origin: (i32, i32),
    last: bool,
}

impl FlushInfo {
    pub fn buffer(&self) -> &BufferInfo {
        &self.buffer
    }
    pub fn data(&self) -> NonNull<u8> {
        self.buffer.data()
    }
    pub fn stride(&self) -> usize {
        self.buffer.stride()
    }
    pub fn format(&self) -> ColorFormat {
        self.buffer.format()
    }
    pub fn origin(&self) -> (i32, i32) {
        self.origin
    }
    pub fn is_last(&self) -> bool {
        self.last
    }

    /// Borrow the dirty pixels row by row, excluding padding and other pixels.
    ///
    /// # Safety
    /// Satisfy [`BufferInfo::as_bytes`] for every returned row. Use only during
    /// this flush, before completion permits native reuse. A synchronous callback
    /// must finish reading before returning; an async backend must finish before
    /// its wait callback returns or it signals flush-ready. Never reenter LVGL
    /// with a row borrow alive.
    pub unsafe fn rows(&self) -> impl ExactSizeIterator<Item = &[u8]> {
        let x = usize::try_from(self.area.x1 - self.origin.0).expect("flush x origin");
        let y = usize::try_from(self.area.y1 - self.origin.1).expect("flush y origin");
        let width = usize::try_from(self.area.x2 - self.area.x1 + 1).expect("flush width");
        let height = usize::try_from(self.area.y2 - self.area.y1 + 1).expect("flush height");
        let bpp = self.format().bytes_per_pixel();
        // SAFETY: caller provides the quiescent native storage guarantee above.
        let data = unsafe { self.buffer.as_bytes() };
        (0..height).map(move |row| {
            let start = (y + row) * self.stride() + x * bpp;
            &data[start..start + width * bpp]
        })
    }
}

/// Opt-in byte-buffer display registration. Defaults to RGB888 and PARTIAL;
/// native configuration and existing `Display::register` behavior are unchanged.
///
/// LVGL must be initialized and all access serialized on its UI thread.
pub struct DisplayBuilder {
    width: u32,
    height: u32,
    format: ColorFormat,
    mode: RenderMode,
    stride: Option<u32>,
    alignment: (u32, u32),
    buffers: Option<DisplayBuffers>,
}

struct State<F, W> {
    flush: RefCell<F>,
    wait: RefCell<W>,
    format: ColorFormat,
    mode: RenderMode,
    asynchronous: bool,
    alignment: InvalidationAlignment,
    rendering: Cell<bool>,
    // Drop backend captures before releasing the memory they described.
    _buffers: UnsafeCell<DisplayBuffers>,
}

impl DisplayBuilder {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: ColorFormat::Rgb888,
            mode: RenderMode::Partial,
            stride: None,
            alignment: (1, 1),
            buffers: None,
        }
    }
    pub fn color_format(mut self, format: ColorFormat) -> Self {
        self.format = format;
        self
    }
    pub fn render_mode(mut self, mode: RenderMode) -> Self {
        self.mode = mode;
        self
    }
    /// Initial bytes per row; omission uses the native stride handler. LVGL
    /// reshapes PARTIAL buffers for each dirty region, so use `FlushInfo::stride`
    /// on every flush. DIRECT/FULL preserve the configured row stride.
    pub fn stride(mut self, bytes: u32) -> Self {
        self.stride = Some(bytes);
        self
    }
    pub fn buffers(mut self, buffers: DisplayBuffers) -> Self {
        self.buffers = Some(buffers);
        self
    }

    /// Expand invalidations outward to a logical pixel grid before LVGL saves
    /// them for drawing and DIRECT synchronization. Counts must be nonzero;
    /// `(1, 1)` is the default. Partial edge groups clip to display bounds.
    ///
    /// PARTIAL buffers must hold at least `min(rows, height)` full-width rows.
    /// The setting is immutable after registration and does not alter byte
    /// stride, native resolution, panel configuration or physical safe areas.
    pub fn invalidation_alignment(mut self, columns: u32, rows: u32) -> Self {
        self.alignment = (columns, rows);
        self
    }

    /// Register a synchronous callback. LVGL receives flush-ready on return.
    /// Native byte access remains unsafe; the callback must finish all accesses
    /// before returning. Callbacks and their destructors must not reenter LVGL
    /// or unwind across C. Metadata inspection itself requires no unsafe borrow.
    pub fn build<F>(self, flush: F) -> Result<Display, DisplayConfigError>
    where
        F: FnMut(&FlushInfo) + 'static,
    {
        self.install(flush, || {}, false)
    }

    /// Register an asynchronous backend with a blocking native wait hook.
    ///
    /// # Safety
    /// `wait` must finish/cancel the prior transfer and all reads/writes of its
    /// buffer before returning, even after errors. It must not depend on a task
    /// blocked by this UI-thread call. Neither callback nor its destructor may
    /// unwind, reenter LVGL, delete displays, or change registered buffer state.
    /// Stop all pending transfer/render work before native deletion or deinit.
    /// Do not signal completion before hardware and cache obligations are met.
    pub unsafe fn build_async<F, W>(self, flush: F, wait: W) -> Result<Display, DisplayConfigError>
    where
        F: FnMut(&FlushInfo) + 'static,
        W: FnMut() + 'static,
    {
        self.install(flush, wait, true)
    }

    fn install<F, W>(
        self,
        flush: F,
        wait: W,
        asynchronous: bool,
    ) -> Result<Display, DisplayConfigError>
    where
        F: FnMut(&FlushInfo) + 'static,
        W: FnMut() + 'static,
    {
        let mut layout = BufferLayout::new(self.width, self.height, self.format, self.stride)?;
        let alignment = InvalidationAlignment::new(
            self.alignment.0,
            self.alignment.1,
            self.width,
            self.height,
        )?;
        let mut buffers = self.buffers.ok_or(DisplayConfigError::MissingBuffer)?;
        if self.mode == RenderMode::Partial {
            let rows = buffers.capacity() / layout.stride as usize;
            if rows < alignment.minimum_buffer_rows() as usize {
                return Err(DisplayConfigError::BufferTooSmall);
            }
            layout.height = rows.min(self.height as usize) as u32;
        }
        let (front, back) = buffers.pointers(layout)?;
        let len = u32::try_from(layout.len()?).map_err(|_| DisplayConfigError::BufferTooSmall)?;
        // Allocate Rust state first. Buffers remain alive through all native setup.
        let state = Box::new(State {
            _buffers: UnsafeCell::new(buffers),
            flush: RefCell::new(flush),
            wait: RefCell::new(wait),
            format: self.format,
            mode: self.mode,
            asynchronous,
            alignment,
            rendering: Cell::new(false),
        });
        // SAFETY: dimensions, byte arithmetic, capacity and alignment were checked.
        let disp = NonNull::new(unsafe {
            sys::lv_display_create(layout.width as i32, self.height as i32)
        })
        .ok_or(DisplayConfigError::NativeAllocation)?;
        unsafe {
            sys::lv_display_set_color_format(disp.as_ptr(), self.format.to_raw());
            sys::lv_display_set_buffers_with_stride(
                disp.as_ptr(),
                front.cast(),
                back.cast(),
                len,
                layout.stride,
                self.mode.to_raw(),
            );
            if sys::lv_display_get_buf_active(disp.as_ptr()).is_null() {
                sys::lv_display_delete(disp.as_ptr());
                return Err(DisplayConfigError::NativeConfiguration);
            }
            let state = Box::into_raw(state);
            let event_filter = if self.alignment == (1, 1) {
                sys::lv_event_code_t_LV_EVENT_DELETE
            } else {
                sys::lv_event_code_t_LV_EVENT_ALL
            };
            let hook = sys::lv_display_add_event_cb(
                disp.as_ptr(),
                Some(display_event::<F, W>),
                event_filter,
                state.cast(),
            );
            if hook.is_null() {
                sys::lv_display_delete(disp.as_ptr());
                drop(Box::from_raw(state));
                return Err(DisplayConfigError::NativeAllocation);
            }
            sys::lv_display_set_user_data(disp.as_ptr(), state.cast());
            sys::lv_display_set_flush_cb(disp.as_ptr(), Some(flush_callback::<F, W>));
            if asynchronous {
                sys::lv_display_set_flush_wait_cb(disp.as_ptr(), Some(wait_callback::<F, W>));
            }
        }
        Ok(Display::from_raw(disp))
    }
}

// SAFETY: LVGL retains State through its delete event, and calls on one UI thread.
unsafe extern "C" fn flush_callback<F: FnMut(&FlushInfo), W: FnMut()>(
    disp: *mut sys::lv_display_t,
    area: *const sys::lv_area_t,
    pixels: *mut u8,
) {
    unsafe {
        let state = &*sys::lv_display_get_user_data(disp).cast::<State<F, W>>();
        let active = &*sys::lv_display_get_buf_active(disp);
        let area = &*area;
        let origin = if state.mode == RenderMode::Partial {
            (area.x1, area.y1)
        } else {
            (
                sys::lv_display_get_offset_x(disp),
                sys::lv_display_get_offset_y(disp),
            )
        };
        let info = FlushInfo {
            area: Area {
                x1: area.x1,
                x2: area.x2,
                y1: area.y1,
                y2: area.y2,
            },
            buffer: BufferInfo {
                data: NonNull::new(pixels).expect("native flush data"),
                len: active.data_size as usize,
                width: active.header.w(),
                height: active.header.h(),
                stride: active.header.stride() as usize,
                format: state.format,
            },
            origin,
            last: sys::lv_display_flush_is_last(disp),
        };
        (state.flush.borrow_mut())(&info);
        if !state.asynchronous {
            sys::lv_display_flush_ready(disp);
        }
    }
}

unsafe extern "C" fn wait_callback<F, W: FnMut()>(disp: *mut sys::lv_display_t) {
    // SAFETY: build_async's backend contract completes transfer before reuse.
    let state = unsafe { &*sys::lv_display_get_user_data(disp).cast::<State<F, W>>() };
    (state.wait.borrow_mut())();
}

unsafe extern "C" fn display_event<F, W>(event: *mut sys::lv_event_t) {
    // SAFETY: LVGL retains this state and serializes its display events. Native
    // deletion removes the event list before deleting widgets; no event can
    // reference state after its DELETE hook. No application callback runs here.
    let state = unsafe { sys::lv_event_get_user_data(event).cast::<State<F, W>>() };
    match unsafe { sys::lv_event_get_code(event) } {
        sys::lv_event_code_t_LV_EVENT_DELETE => unsafe { drop(Box::from_raw(state)) },
        sys::lv_event_code_t_LV_EVENT_RENDER_START => unsafe { (*state).rendering.set(true) },
        sys::lv_event_code_t_LV_EVENT_RENDER_READY => unsafe { (*state).rendering.set(false) },
        sys::lv_event_code_t_LV_EVENT_INVALIDATE_AREA => unsafe {
            // Real invalidations occur outside rendering. The pinned renderer
            // uses the same event for PARTIAL row-capacity probes while drawing.
            let area = &mut *sys::lv_event_get_param(event).cast::<sys::lv_area_t>();
            if (*state).rendering.get() {
                (*state).alignment.fit_partial_probe(area);
            } else {
                (*state).alignment.expand(area);
            }
        },
        _ => {}
    }
}

impl Display {
    /// Native identity; dereferencing it requires the native lifetime/thread rules.
    /// Raw calls must preserve builder-managed dimensions, rotation, format,
    /// buffers, callbacks and user data. Quiesce and recreate to change them.
    pub fn raw(&self) -> NonNull<sys::lv_display_t> {
        self.disp
    }

    /// Active-buffer metadata, without granting a safe borrow of native pixels.
    pub fn active_buffer(&self) -> Option<BufferInfo> {
        // SAFETY: Display's native-lifetime contract excludes deletion/reentry.
        unsafe {
            let buffer = sys::lv_display_get_buf_active(self.disp.as_ptr()).as_ref()?;
            Some(BufferInfo {
                data: NonNull::new(buffer.data)?,
                len: buffer.data_size as usize,
                width: buffer.header.w(),
                height: buffer.header.h(),
                stride: buffer.header.stride() as usize,
                format: ColorFormat::from_raw(sys::lv_display_get_color_format(
                    self.disp.as_ptr(),
                ))?,
            })
        }
    }

    /// Complete a raw backend transfer and permit LVGL to reuse its buffer.
    ///
    /// # Safety
    /// All hardware/render work using the submitted buffer must have completed,
    /// and no Rust byte borrow may remain. Signal only for the current pending
    /// flush of this live display on its valid native execution context. A DMA
    /// interrupt must obey the platform's synchronization/LVGL thread rules.
    pub unsafe fn flush_ready(&mut self) {
        unsafe { sys::lv_display_flush_ready(self.disp.as_ptr()) };
    }
}
