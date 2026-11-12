use crate::{
    display::BufferLayout, sys, Box, ColorFormat, DisplayConfigError, NativeObject, PixelBuffer,
};
use core::{cell::UnsafeCell, ptr};

/// Native RGB888/RGB565 Canvas storage with an explicit byte stride.
///
/// Attachment retains both data and draw descriptor until replacement or the
/// native delete event. This requires synchronous software rendering with image
/// caches disabled, matching the loaded-resource contract. LVGL must be initialized.
/// RGB565_SWAPPED editable Canvas operations are not supported by pinned LVGL.
pub struct CanvasBuffer {
    layout: BufferLayout,
    pixels: PixelBuffer,
}

struct State {
    draw: UnsafeCell<sys::lv_draw_buf_t>,
    pixels: UnsafeCell<PixelBuffer>,
}

impl CanvasBuffer {
    pub fn new(
        width: u32,
        height: u32,
        format: ColorFormat,
        stride: Option<u32>,
        mut pixels: PixelBuffer,
    ) -> Result<Self, DisplayConfigError> {
        if format == ColorFormat::Rgb565Swapped {
            return Err(DisplayConfigError::UnsupportedFormat);
        }
        let layout = BufferLayout::new(width, height, format, stride)?;
        layout.validate(&mut pixels)?;
        Ok(Self { layout, pixels })
    }
}

impl super::Canvas<'_> {
    /// Attach initialized storage and retire any prior attachment after LVGL
    /// switches descriptors. Ordinary Rust widget-handle drop does not free it.
    /// Native delete callbacks must not render, reuse or recursively delete this
    /// object. Raw native changes must preserve this attachment's lifetime rules.
    pub fn attach_buffer(&mut self, buffer: CanvasBuffer) -> Result<(), DisplayConfigError> {
        // The pinned synchronous/no-cache configuration cannot retain draw tasks
        // or decoded references after the setter/delete event returns.
        if unsafe { sys::lv_rs_loaded_resource_support() } & 3 != 3 {
            return Err(DisplayConfigError::NativeConfiguration);
        }
        let CanvasBuffer { layout, pixels } = buffer;
        let mut pixels = pixels;
        let data = pixels.as_mut_ptr();
        let size = layout.len()? as u32;
        let state = Box::new(State {
            // SAFETY: the descriptor is initialized by lv_draw_buf_init before use.
            draw: UnsafeCell::new(unsafe { core::mem::zeroed() }),
            pixels: UnsafeCell::new(pixels),
        });
        let raw = self.raw().as_ptr();
        // SAFETY: live Canvas on the UI thread, checked layout and retained data.
        unsafe {
            if sys::lv_draw_buf_init(
                state.draw.get(),
                layout.width,
                layout.height,
                layout.format.to_raw(),
                layout.stride,
                data.cast(),
                size,
            ) != sys::lv_result_t_LV_RESULT_OK
            {
                return Err(DisplayConfigError::NativeConfiguration);
            }
            let mut previous = ptr::null_mut::<State>();
            for index in 0..sys::lv_obj_get_event_count(raw) {
                let event = sys::lv_obj_get_event_dsc(raw, index);
                if sys::lv_event_dsc_get_cb(event).is_some_and(|callback| {
                    ptr::fn_addr_eq(
                        callback,
                        delete_state as unsafe extern "C" fn(*mut sys::lv_event_t),
                    )
                }) {
                    previous = sys::lv_event_dsc_get_user_data(event).cast();
                    break;
                }
            }
            let state = Box::into_raw(state);
            if sys::lv_obj_add_event_cb(
                raw,
                Some(delete_state),
                sys::lv_event_code_t_LV_EVENT_DELETE,
                state.cast(),
            )
            .is_null()
            {
                drop(Box::from_raw(state));
                return Err(DisplayConfigError::NativeAllocation);
            }
            sys::lv_canvas_set_draw_buf(raw, (*state).draw.get());
            if sys::lv_canvas_get_draw_buf(raw) != (*state).draw.get() {
                sys::lv_obj_remove_event_cb_with_user_data(raw, Some(delete_state), state.cast());
                drop(Box::from_raw(state));
                return Err(DisplayConfigError::NativeConfiguration);
            }
            if !previous.is_null() {
                sys::lv_obj_remove_event_cb_with_user_data(
                    raw,
                    Some(delete_state),
                    previous.cast(),
                );
                drop(Box::from_raw(previous));
            }
        }
        Ok(())
    }
}

unsafe extern "C" fn delete_state(event: *mut sys::lv_event_t) {
    // SAFETY: delete is serialized after synchronous drawing. Clear the inherited
    // image reference before freeing our descriptor/data. The pinned Canvas
    // destructor only drops the cache key at &canvas->draw_buf; it does not read
    // the external descriptor. No deletion callback may draw/reuse the object.
    unsafe {
        let state = Box::from_raw(sys::lv_event_get_user_data(event).cast::<State>());
        let raw = sys::lv_event_get_target(event).cast::<sys::lv_obj_t>();
        if sys::lv_canvas_get_draw_buf(raw) == state.draw.get() {
            sys::lv_image_set_src(raw, ptr::null());
        }
        // Keep pixel storage as an UnsafeCell: C writes it independently of the
        // Rust registration descriptor. Destruction releases its allocator pair.
        let _ = state.pixels.get();
        drop(state);
    }
}
