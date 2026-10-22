use crate::{Box, LvResult, Obj, Widget};
use core::{mem, num::TryFromIntError, ptr::NonNull, time::Duration};
use cty::c_void;

#[repr(u32)]
pub enum AnimRepeatCount {
    Finite(u32),
    Infinite,
}

pub struct Animation {
    pub(crate) raw: Box<rs_lvgl_sys::lv_anim_t>,
}

impl Animation {
    pub fn new<'a, 'b, T, F>(
        target: &mut T,
        duration: Duration,
        start: i32,
        end: i32,
        animator: F,
    ) -> LvResult<Self>
    where
        T: Widget<'b>,
        F: FnMut(&mut Obj, i32) + 'a,
    {
        unsafe {
            let mut anim = Animation {
                raw: {
                    Box::new({
                        let mut inner = core::mem::MaybeUninit::<rs_lvgl_sys::lv_anim_t>::uninit();
                        rs_lvgl_sys::lv_anim_init(inner.as_mut_ptr());
                        inner.assume_init()
                    })
                },
            };

            rs_lvgl_sys::lv_anim_set_duration(anim.raw.as_mut(), duration.as_millis() as u32);
            rs_lvgl_sys::lv_anim_set_values(anim.raw.as_mut(), start, end);
            rs_lvgl_sys::lv_anim_set_var(anim.raw.as_mut(), target as *mut _ as *mut c_void);
            rs_lvgl_sys::lv_anim_set_exec_cb(
                anim.raw.as_mut(),
                Some(animator_trampoline::<'a, 'b, T, F>),
            );
            rs_lvgl_sys::lv_anim_set_deleted_cb(anim.raw.as_mut(), Some(delete_animator::<F>));

            rs_lvgl_sys::lv_anim_set_user_data(
                anim.raw.as_mut(),
                Box::<F>::into_raw(Box::new(animator)) as *mut c_void,
            );

            Ok(anim)
        }
    }

    pub fn start(self) {
        unsafe {
            rs_lvgl_sys::lv_anim_start(&*self.raw as *const _);
        }
    }

    pub fn set_delay(&mut self, delay: Duration) -> Result<(), TryFromIntError> {
        unsafe {
            rs_lvgl_sys::lv_anim_set_delay(self.raw.as_mut(), delay.as_millis().try_into()?);
        }
        Ok(())
    }

    pub fn set_reverse_delay(&mut self, delay: Duration) -> Result<(), TryFromIntError> {
        (*self.raw.as_mut()).reverse_delay = delay.as_millis().try_into()?;
        Ok(())
    }

    pub fn set_reverse_duration(&mut self, time: Duration) -> Result<(), TryFromIntError> {
        unsafe {
            rs_lvgl_sys::lv_anim_set_reverse_duration(
                self.raw.as_mut(),
                time.as_millis().try_into()?,
            );
        }
        Ok(())
    }

    pub fn set_repeat_delay(&mut self, delay: Duration) -> Result<(), TryFromIntError> {
        unsafe {
            rs_lvgl_sys::lv_anim_set_repeat_delay(self.raw.as_mut(), delay.as_millis().try_into()?);
        }
        Ok(())
    }

    pub fn set_repeat_count(&mut self, count: AnimRepeatCount) {
        unsafe {
            rs_lvgl_sys::lv_anim_set_repeat_count(
                self.raw.as_mut(),
                match count {
                    AnimRepeatCount::Finite(c) => c,
                    AnimRepeatCount::Infinite => rs_lvgl_sys::LV_ANIM_REPEAT_INFINITE,
                },
            );
        }
    }

    pub fn set_early_apply(&mut self, apply: bool) {
        unsafe {
            rs_lvgl_sys::lv_anim_set_early_apply(self.raw.as_mut(), apply);
        }
    }
}

unsafe extern "C" fn animator_trampoline<'a, 'b, T, F>(obj: *mut c_void, val: i32)
where
    T: Widget<'b>,
    F: FnMut(&mut Obj, i32) + 'a,
{
    unsafe {
        let anim = NonNull::new(rs_lvgl_sys::lv_anim_get(obj, None)).unwrap();
        let obj_ptr = (*(obj as *mut T)).raw();
        if !(*anim.as_ref()).user_data.is_null() {
            let callback = &mut *((*anim.as_ref()).user_data as *mut F);
            let mut obj_nondrop = Obj::from_raw(obj_ptr).unwrap();
            callback(&mut obj_nondrop, val);
            mem::forget(obj_nondrop)
        }
    }
}

unsafe extern "C" fn delete_animator<F>(anim: *mut rs_lvgl_sys::lv_anim_t) {
    unsafe {
        let user_data = rs_lvgl_sys::lv_anim_get_user_data(anim as *const rs_lvgl_sys::lv_anim_t);
        if !user_data.is_null() {
            drop(Box::<F>::from_raw(user_data as *mut F));
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::widgets::Button;
    use crate::Display;

    #[test]
    fn anim_test() {
        crate::tests::initialize_test(true);
        let display = Display::default();
        let mut screen = display.get_scr_act().unwrap();
        let mut btn = Button::create(&mut screen).unwrap();
        let anim = Animation::new(&mut btn, Duration::from_millis(10), 0, 100, |_, _| {}).unwrap();
        anim.start();
    }
}
