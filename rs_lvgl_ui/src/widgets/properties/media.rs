//! Property updates for media widgets.

use super::super::Props;
use super::super::Retained;
use crate::node::ImageSource;
use crate::UiError;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ptr::NonNull;
use rs_lvgl_sys::lv_obj_t;

pub(super) fn apply(
    obj: NonNull<lv_obj_t>,
    new: &Props,
    old: Option<&Props>,
    retained: &mut Retained,
) -> Result<(), UiError> {
    match (new, old) {
        #[cfg(feature = "widget-image")]
        (Props::Image { source, scale }, old) => {
            let (old_source, old_scale) = match old {
                Some(Props::Image { source, scale }) => (Some(*source), *scale),
                _ => (None, crate::Scale::percent(0)),
            };
            let source_changed = old_source != Some(*source);
            // SAFETY: `obj` is a live image object. `retained` keeps a
            // generated pixel descriptor alive for as long as the mounted
            // node exists; raw descriptors are the caller's responsibility.
            unsafe {
                let rebuild = source_changed
                    || (!matches!(retained, Retained::Image(_))
                        && source.is_pixels());
                if rebuild {
                    let new_descriptor = source.descriptor();
                    let pointer = source.ptr(new_descriptor.as_deref());
                    let previous = rs_lvgl_sys::lv_image_get_src(obj.as_ptr());
                    rs_lvgl_sys::lv_image_set_src(obj.as_ptr(), pointer);
                    // The native setter returns void. Decoder rejection clears
                    // the source; string-allocation failure can leave the old
                    // source installed. Preserve its retained data until the
                    // caller deletes the failed subtree in either case.
                    let installed = rs_lvgl_sys::lv_image_get_src(obj.as_ptr());
                    match source {
                        ImageSource::Text(_) => {
                            if installed.is_null() {
                                return Err(UiError::InvalidProperty);
                            }
                            if installed == previous && installed != pointer {
                                return Err(UiError::Allocation);
                            }
                        }
                        _ if installed != pointer => return Err(UiError::InvalidProperty),
                        _ => {}
                    }
                    *retained = match new_descriptor {
                        Some(descriptor) => Retained::Image(descriptor),
                        None => Retained::None,
                    };
                }
                if old_scale != *scale {
                    rs_lvgl_sys::lv_image_set_scale(obj.as_ptr(), scale.to_raw() as u32);
                }
            }
        }
        #[cfg(feature = "widget-imagebutton")]
        (Props::ImageButton { released, pressed }, old) => {
            let changed = match old {
                Some(Props::ImageButton {
                    released: old_released,
                    pressed: old_pressed,
                }) => old_released != released || old_pressed != pressed,
                _ => true,
            } || (!matches!(retained, Retained::Images(_))
                && core::iter::once(released)
                    .chain(pressed)
                    .any(|source| source.is_pixels()));
            if changed {
                let (descriptors, pointers) =
                    image_pointers(core::iter::once(*released).chain(*pressed));
                let pressed_pointer = if pressed.is_some() {
                    pointers[1]
                } else {
                    core::ptr::null()
                };
                // SAFETY: `obj` is a live image button. LVGL stores the
                // pointers; generated descriptors move into `retained`
                // without changing address, after LVGL switched to them.
                unsafe {
                    rs_lvgl_sys::lv_imagebutton_set_src(
                        obj.as_ptr(),
                        rs_lvgl_sys::lv_imagebutton_state_t_LV_IMAGEBUTTON_STATE_RELEASED,
                        core::ptr::null(),
                        pointers[0],
                        core::ptr::null(),
                    );
                    rs_lvgl_sys::lv_imagebutton_set_src(
                        obj.as_ptr(),
                        rs_lvgl_sys::lv_imagebutton_state_t_LV_IMAGEBUTTON_STATE_PRESSED,
                        core::ptr::null(),
                        pressed_pointer,
                        core::ptr::null(),
                    );
                }
                *retained = Retained::Images(descriptors);
            }
        }
        #[cfg(feature = "widget-animimage")]
        (
            Props::AnimatedImage {
                frames,
                duration_ms,
                repeat,
            },
            old,
        ) => {
            let changed = match old {
                Some(Props::AnimatedImage {
                    frames: old_frames,
                    duration_ms: old_duration,
                    repeat: old_repeat,
                }) => old_frames != frames || old_duration != duration_ms || old_repeat != repeat,
                _ => true,
            } || !matches!(retained, Retained::Frames { .. });
            if changed {
                let (descriptors, mut pointers) = image_pointers(frames.iter().copied());
                // SAFETY: `obj` is a live animated image. LVGL stores the
                // pointer array and the image stores the current frame; the
                // first frame is shown before the previous buffers are
                // dropped, and restarting replaces the running animation
                // (LVGL deletes it together with the object).
                unsafe {
                    rs_lvgl_sys::lv_animimg_set_src(
                        obj.as_ptr(),
                        pointers.as_mut_ptr(),
                        pointers.len(),
                    );
                    rs_lvgl_sys::lv_image_set_src(obj.as_ptr(), pointers[0]);
                    rs_lvgl_sys::lv_animimg_set_duration(obj.as_ptr(), *duration_ms);
                    rs_lvgl_sys::lv_animimg_set_repeat_count(obj.as_ptr(), *repeat);
                    rs_lvgl_sys::lv_animimg_start(obj.as_ptr());
                }
                *retained = Retained::Frames {
                    _descriptors: descriptors,
                    _pointers: pointers,
                };
            }
        }
        #[cfg(feature = "widget-line")]
        (Props::Line { points }, old) => {
            let changed = match old {
                Some(Props::Line { points: old_points }) => old_points != points,
                _ => true,
            } || !matches!(retained, Retained::Points(_));
            if changed {
                let buffer: Vec<rs_lvgl_sys::lv_point_precise_t> = points
                    .iter()
                    .map(|&(x, y)| rs_lvgl_sys::lv_point_precise_t {
                        x: x as _,
                        y: y as _,
                    })
                    .collect();
                // SAFETY: `obj` is a live line. LVGL stores the pointer; the
                // buffer is moved into `retained` (its heap address does not
                // change) and outlives every use, because the old buffer is
                // only dropped after LVGL points at the new one.
                unsafe {
                    rs_lvgl_sys::lv_line_set_points(
                        obj.as_ptr(),
                        buffer.as_ptr(),
                        buffer.len() as u32,
                    )
                };
                *retained = Retained::Points(buffer);
            }
        }
        _ => unreachable!("property dispatcher selected a different widget family"),
    }
    Ok(())
}

/// Build image pointers for `sources`, generating RGB565 descriptors.
///
/// The descriptors are boxed so their addresses survive moving the vector
/// into [`Retained`] and while the vector grows.
#[allow(clippy::vec_box)]
#[cfg(any(feature = "widget-imagebutton", feature = "widget-animimage"))]
fn image_pointers(
    sources: impl Iterator<Item = ImageSource>,
) -> (
    Vec<Box<rs_lvgl_sys::lv_image_dsc_t>>,
    Vec<*const cty::c_void>,
) {
    let mut descriptors = Vec::new();
    let mut pointers = Vec::new();
    for source in sources {
        match source.descriptor() {
            Some(descriptor) => {
                pointers.push(source.ptr(Some(&descriptor)));
                descriptors.push(descriptor);
            }
            None => pointers.push(source.ptr(None)),
        }
    }
    (descriptors, pointers)
}
