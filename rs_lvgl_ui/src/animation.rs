//! Native animations scoped to mounted nodes. Descriptors are values; LVGL
//! timers retain only live native objects or stable mounted transition storage.
use crate::{node::Selector, Node, Style, UiError};
use alloc::{boxed::Box, vec::Vec};
use core::{ffi::c_void, ptr::NonNull};
use rs_lvgl_sys::{lv_obj_t, lv_style_prop_t};

/// Built-in native easing paths. No application callback crosses the C ABI.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Easing {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Step,
}
impl Easing {
    fn raw(self) -> rs_lvgl_sys::lv_anim_path_cb_t {
        Some(match self {
            Self::Linear => rs_lvgl_sys::lv_anim_path_linear,
            Self::EaseIn => rs_lvgl_sys::lv_anim_path_ease_in,
            Self::EaseOut => rs_lvgl_sys::lv_anim_path_ease_out,
            Self::EaseInOut => rs_lvgl_sys::lv_anim_path_ease_in_out,
            Self::Step => rs_lvgl_sys::lv_anim_path_step,
        })
    }
}

macro_rules! style_properties {
    ($($name:ident => $raw:ident),* $(,)?) => {
        /// Native style properties supported by state transitions.
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum TransitionProperty { $($name),* }
        impl TransitionProperty {
            fn raw(self) -> lv_style_prop_t { match self { $(Self::$name => rs_lvgl_sys::$raw as lv_style_prop_t),* } }
        }
    }
}
style_properties! {
    BackgroundColor => _lv_style_id_t_LV_STYLE_BG_COLOR,
    BackgroundOpacity => _lv_style_id_t_LV_STYLE_BG_OPA,
    TextColor => _lv_style_id_t_LV_STYLE_TEXT_COLOR,
    BorderColor => _lv_style_id_t_LV_STYLE_BORDER_COLOR,
    BorderWidth => _lv_style_id_t_LV_STYLE_BORDER_WIDTH,
    Radius => _lv_style_id_t_LV_STYLE_RADIUS,
    Opacity => _lv_style_id_t_LV_STYLE_OPA,
    Width => _lv_style_id_t_LV_STYLE_WIDTH,
    Height => _lv_style_id_t_LV_STYLE_HEIGHT,
    X => _lv_style_id_t_LV_STYLE_X,
    Y => _lv_style_id_t_LV_STYLE_Y,
    ShadowWidth => _lv_style_id_t_LV_STYLE_SHADOW_WIDTH,
    ShadowColor => _lv_style_id_t_LV_STYLE_SHADOW_COLOR,
    ShadowOpacity => _lv_style_id_t_LV_STYLE_SHADOW_OPA,
    ShadowOffsetX => _lv_style_id_t_LV_STYLE_SHADOW_OFFSET_X,
    ShadowOffsetY => _lv_style_id_t_LV_STYLE_SHADOW_OFFSET_Y,
    ShadowSpread => _lv_style_id_t_LV_STYLE_SHADOW_SPREAD,
    OutlineWidth => _lv_style_id_t_LV_STYLE_OUTLINE_WIDTH,
    OutlineColor => _lv_style_id_t_LV_STYLE_OUTLINE_COLOR,
    OutlineOpacity => _lv_style_id_t_LV_STYLE_OUTLINE_OPA,
    OutlinePad => _lv_style_id_t_LV_STYLE_OUTLINE_PAD,
    GradientColor => _lv_style_id_t_LV_STYLE_BG_GRAD_COLOR,
    ImageRecolor => _lv_style_id_t_LV_STYLE_IMAGE_RECOLOR,
    ImageRecolorOpacity => _lv_style_id_t_LV_STYLE_IMAGE_RECOLOR_OPA,
    Rotation => _lv_style_id_t_LV_STYLE_TRANSFORM_ROTATION,
    ScaleX => _lv_style_id_t_LV_STYLE_TRANSFORM_SCALE_X,
    ScaleY => _lv_style_id_t_LV_STYLE_TRANSFORM_SCALE_Y,
    LetterSpace => _lv_style_id_t_LV_STYLE_TEXT_LETTER_SPACE,
    LineSpace => _lv_style_id_t_LV_STYLE_TEXT_LINE_SPACE,
}

/// Animate selected style properties when native widget state changes.
/// The property list is static so `Style` remains Copy. Mounting translates it
/// into stable, zero-terminated native storage. Duration and delay use milliseconds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StyleTransition {
    properties: &'static [TransitionProperty],
    duration: u32,
    delay: u32,
    easing: Easing,
}
impl StyleTransition {
    pub const fn new(properties: &'static [TransitionProperty], duration_ms: u32) -> Self {
        Self {
            properties,
            duration: duration_ms,
            delay: 0,
            easing: Easing::Linear,
        }
    }
    pub const fn delay(mut self, milliseconds: u32) -> Self {
        self.delay = milliseconds;
        self
    }
    pub const fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
    pub(crate) fn validate(self) -> Result<(), UiError> {
        valid_time(self.duration)?;
        valid_time(self.delay)?;
        if self.properties.is_empty() {
            return Err(UiError::InvalidProperty);
        }
        Ok(())
    }
}
impl Style {
    /// Install a state transition on the selector receiving this style.
    pub fn transition(mut self, transition: StyleTransition) -> Self {
        self.transition = Some(transition);
        self
    }
}

pub(crate) struct NativeTransition {
    selector: u32,
    properties: Box<[lv_style_prop_t]>,
    // C retains this allocation until its local style is detached or the object is deleted.
    _descriptor: Box<rs_lvgl_sys::lv_style_transition_dsc_t>,
}

pub(crate) fn clear_transitions(obj: NonNull<lv_obj_t>, retained: &mut Vec<NativeTransition>) {
    // SAFETY: descriptors belong to this live object. Detach them and cancel
    // their property transitions before releasing stable storage. A scalar
    // local write cancels the corresponding native transition; restore whether
    // a local override existed so inherited/class values remain inherited.
    unsafe {
        for old in retained.iter() {
            rs_lvgl_sys::lv_obj_remove_local_style_prop(
                obj.as_ptr(),
                rs_lvgl_sys::_lv_style_id_t_LV_STYLE_TRANSITION as _,
                old.selector,
            );
            for &prop in old.properties.iter().take(old.properties.len() - 1) {
                let mut value = core::mem::zeroed();
                let found = rs_lvgl_sys::lv_obj_get_local_style_prop(
                    obj.as_ptr(),
                    prop,
                    &mut value,
                    old.selector,
                );
                if found != rs_lvgl_sys::lv_style_res_t_LV_STYLE_RES_FOUND {
                    value = rs_lvgl_sys::lv_obj_get_style_prop(
                        obj.as_ptr(),
                        old.selector & rs_lvgl_sys::lv_part_t_LV_PART_ANY,
                        prop,
                    );
                }
                rs_lvgl_sys::lv_obj_set_local_style_prop(obj.as_ptr(), prop, value, old.selector);
                if found != rs_lvgl_sys::lv_style_res_t_LV_STYLE_RES_FOUND {
                    rs_lvgl_sys::lv_obj_remove_local_style_prop(obj.as_ptr(), prop, old.selector);
                }
            }
        }
    }
    retained.clear();
}

pub(crate) fn install_transitions(
    obj: NonNull<lv_obj_t>,
    style: &Style,
    parts: &[(Selector, Style)],
) -> Vec<NativeTransition> {
    let mut retained = Vec::new();
    for (selector, style) in
        core::iter::once((0, style)).chain(parts.iter().map(|(s, v)| (s.to_raw(), v)))
    {
        let Some(transition) = style.transition else {
            continue;
        };
        let properties: Box<[_]> = transition
            .properties
            .iter()
            .map(|p| p.raw())
            .chain(core::iter::once(0))
            .collect();
        // SAFETY: initialized by LVGL below; all fields of this C descriptor
        // permit zero initialization. Box addresses survive Vec reallocations.
        let mut descriptor = Box::new(unsafe { core::mem::zeroed() });
        unsafe {
            rs_lvgl_sys::lv_style_transition_dsc_init(
                &mut *descriptor,
                properties.as_ptr(),
                transition.easing.raw(),
                transition.duration,
                transition.delay,
                core::ptr::null_mut(),
            );
            rs_lvgl_sys::lv_obj_set_style_transition(obj.as_ptr(), &*descriptor, selector);
        }
        retained.push(NativeTransition {
            selector,
            properties,
            _descriptor: descriptor,
        });
    }
    retained
}

/// Scalar properties accepted by explicit node animations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnimationProperty {
    X,
    Y,
    Width,
    Height,
    Opacity,
    Rotation,
    ScaleX,
    ScaleY,
}
macro_rules! animator {
    ($name:ident, $setter:ident) => {
        unsafe extern "C" fn $name(obj: *mut c_void, value: i32) {
            // SAFETY: LVGL cancels all animations using obj before object destruction.
            // These callbacks do not allocate, call Rust closures, or unwind.
            unsafe {
                rs_lvgl_sys::$setter(obj.cast(), value as _, 0);
            }
        }
    };
}
animator!(animate_x, lv_obj_set_style_x);
animator!(animate_y, lv_obj_set_style_y);
animator!(animate_width, lv_obj_set_style_width);
animator!(animate_height, lv_obj_set_style_height);
animator!(animate_opacity, lv_obj_set_style_opa);
animator!(animate_rotation, lv_obj_set_style_transform_rotation);
animator!(animate_scale_x, lv_obj_set_style_transform_scale_x);
animator!(animate_scale_y, lv_obj_set_style_transform_scale_y);
impl AnimationProperty {
    fn callback(self) -> rs_lvgl_sys::lv_anim_exec_xcb_t {
        Some(match self {
            Self::X => animate_x,
            Self::Y => animate_y,
            Self::Width => animate_width,
            Self::Height => animate_height,
            Self::Opacity => animate_opacity,
            Self::Rotation => animate_rotation,
            Self::ScaleX => animate_scale_x,
            Self::ScaleY => animate_scale_y,
        })
    }
    fn validate(self, value: i32) -> Result<(), UiError> {
        let max = rs_lvgl_sys::LV_COORD_MAX as i32;
        let valid = match self {
            Self::Opacity => (0..=255).contains(&value),
            Self::Width | Self::Height | Self::ScaleX | Self::ScaleY => (0..=max).contains(&value),
            _ => (-max..=max).contains(&value),
        };
        if valid {
            Ok(())
        } else {
            Err(UiError::InvalidProperty)
        }
    }
}

/// A node animation. Unchanged descriptors do not restart after view updates.
/// Changing its descriptor restarts that property; omitting it cancels the timer.
/// Multiple properties with delays form a timeline on the same node. Native
/// style/layout rules still apply (animate floating children in flex layouts).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Animation {
    property: AnimationProperty,
    from: i32,
    to: i32,
    duration: u32,
    delay: u32,
    easing: Easing,
    repeat: u32,
    reverse: u32,
}
impl Animation {
    pub const fn new(property: AnimationProperty, from: i32, to: i32, duration_ms: u32) -> Self {
        Self {
            property,
            from,
            to,
            duration: duration_ms,
            delay: 0,
            easing: Easing::Linear,
            repeat: 1,
            reverse: 0,
        }
    }
    pub const fn delay(mut self, milliseconds: u32) -> Self {
        self.delay = milliseconds;
        self
    }
    pub const fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
    /// Total play count; `u32::MAX` repeats indefinitely. Zero is rejected.
    pub const fn repeat(mut self, count: u32) -> Self {
        self.repeat = count;
        self
    }
    /// Reverse playback duration in milliseconds; zero disables reverse playback.
    pub const fn reverse(mut self, milliseconds: u32) -> Self {
        self.reverse = milliseconds;
        self
    }
}
impl<M> Node<M> {
    pub fn animate(mut self, animation: Animation) -> Self {
        self.animations.push(animation);
        self
    }
}
fn valid_time(time: u32) -> Result<(), UiError> {
    if time > i32::MAX as u32 {
        Err(UiError::InvalidProperty)
    } else {
        Ok(())
    }
}
pub(crate) fn validate(animations: &[Animation]) -> Result<(), UiError> {
    for (i, a) in animations.iter().enumerate() {
        valid_time(a.duration)?;
        valid_time(a.delay)?;
        valid_time(a.reverse)?;
        a.property.validate(a.from)?;
        a.property.validate(a.to)?;
        if a.repeat == 0 || animations[..i].iter().any(|p| p.property == a.property) {
            return Err(UiError::InvalidProperty);
        }
    }
    Ok(())
}
pub(crate) fn sync(
    obj: NonNull<lv_obj_t>,
    old: &[Animation],
    new: &[Animation],
) -> Result<(), UiError> {
    // SAFETY: object is mounted. lv_anim_start copies the initialized native
    // descriptor, and native object destruction cancels every animation with var=obj.
    unsafe {
        for a in old.iter().filter(|a| !new.contains(a)) {
            rs_lvgl_sys::lv_anim_delete(obj.as_ptr().cast(), a.property.callback());
        }
        for a in new.iter().filter(|a| !old.contains(a)) {
            let mut native = core::mem::zeroed();
            rs_lvgl_sys::lv_anim_init(&mut native);
            rs_lvgl_sys::lv_anim_set_var(&mut native, obj.as_ptr().cast());
            rs_lvgl_sys::lv_anim_set_exec_cb(&mut native, a.property.callback());
            rs_lvgl_sys::lv_anim_set_values(&mut native, a.from, a.to);
            rs_lvgl_sys::lv_anim_set_duration(&mut native, a.duration);
            rs_lvgl_sys::lv_anim_set_delay(&mut native, a.delay);
            rs_lvgl_sys::lv_anim_set_early_apply(&mut native, a.delay == 0);
            rs_lvgl_sys::lv_anim_set_path_cb(&mut native, a.easing.raw());
            rs_lvgl_sys::lv_anim_set_repeat_count(&mut native, a.repeat);
            rs_lvgl_sys::lv_anim_set_reverse_duration(&mut native, a.reverse);
            if rs_lvgl_sys::lv_anim_start(&native).is_null() {
                return Err(UiError::Allocation);
            }
        }
    }
    Ok(())
}
