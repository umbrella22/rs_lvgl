use super::{BufferStatus, Data, InputDriver, InputState};
use crate::Box;
use crate::Point;
use crate::{LvError, LvResult};

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum PointerInputData {
    Touch(Point),
    Key(u32),
}

impl PointerInputData {
    pub fn pressed(self) -> InputState {
        InputState::Pressed(Data::Pointer(self))
    }

    pub fn released(self) -> InputState {
        InputState::Released(Data::Pointer(self))
    }
}

pub struct Pointer {
    descriptor: Option<*mut rs_lvgl_sys::lv_indev_t>,
    user_data_drop: Option<unsafe fn(*mut cty::c_void)>,
}

impl InputDriver<Pointer> for Pointer {
    fn register<F>(handler: F, _: &crate::Display) -> LvResult<Self>
    where
        F: Fn() -> BufferStatus,
    {
        unsafe {
            let indev = rs_lvgl_sys::lv_indev_create();
            if indev.is_null() {
                return Err(LvError::LvOOMemory);
            }

            rs_lvgl_sys::lv_indev_set_type(
                indev,
                rs_lvgl_sys::lv_indev_type_t_LV_INDEV_TYPE_POINTER,
            );
            rs_lvgl_sys::lv_indev_set_read_cb(indev, Some(read_input::<F>));

            let user_data = Box::<F>::into_raw(Box::new(handler)) as *mut cty::c_void;
            rs_lvgl_sys::lv_indev_set_user_data(indev, user_data);

            Ok(Self {
                descriptor: Some(indev),
                user_data_drop: Some(drop_input_handler::<F>),
            })
        }
    }

    fn get_descriptor(&self) -> Option<*mut rs_lvgl_sys::lv_indev_t> {
        self.descriptor
    }

    unsafe fn set_descriptor(&mut self, descriptor: *mut rs_lvgl_sys::lv_indev_t) -> LvResult<()> {
        if self.descriptor.is_none() {
            self.descriptor = Some(descriptor);
        } else {
            return Err(LvError::AlreadyInUse);
        }
        Ok(())
    }
}

impl Drop for Pointer {
    fn drop(&mut self) {
        if let Some(indev) = self.descriptor {
            unsafe {
                if let Some(drop_user_data) = self.user_data_drop {
                    let user_data = rs_lvgl_sys::lv_indev_get_user_data(indev);
                    if !user_data.is_null() {
                        drop_user_data(user_data);
                        rs_lvgl_sys::lv_indev_set_user_data(indev, core::ptr::null_mut());
                    }
                }
                rs_lvgl_sys::lv_indev_delete(indev);
            }
        }
    }
}

unsafe extern "C" fn read_input<F>(
    indev: *mut rs_lvgl_sys::lv_indev_t,
    data: *mut rs_lvgl_sys::lv_indev_data_t,
) where
    F: Fn() -> BufferStatus,
{
    let user_data = unsafe { rs_lvgl_sys::lv_indev_get_user_data(indev) };
    if user_data.is_null() {
        return;
    }
    let user_closure = &mut *(user_data as *mut F);
    let info = user_closure();
    unsafe {
        (*data).continue_reading = match info {
            BufferStatus::Once(b) => {
                fill_pointer_data(data, b);
                false
            }
            BufferStatus::Buffered(b) => {
                fill_pointer_data(data, b);
                true
            }
        }
    }
}

unsafe fn fill_pointer_data(data: *mut rs_lvgl_sys::lv_indev_data_t, state: InputState) {
    match state {
        InputState::Pressed(d) => {
            match d {
                Data::Pointer(PointerInputData::Touch(point)) => {
                    (*data).point.x = point.x as i32;
                    (*data).point.y = point.y as i32;
                }
                Data::Pointer(PointerInputData::Key(_)) => {}
                _ => panic!("Non-pointer data returned from pointer device!"),
            }
            (*data).state = rs_lvgl_sys::lv_indev_state_t_LV_INDEV_STATE_PRESSED;
        }
        InputState::Released(d) => {
            match d {
                Data::Pointer(PointerInputData::Touch(point)) => {
                    (*data).point.x = point.x as i32;
                    (*data).point.y = point.y as i32;
                }
                Data::Pointer(PointerInputData::Key(_)) => {}
                _ => panic!("Non-pointer data returned from pointer device!"),
            }
            (*data).state = rs_lvgl_sys::lv_indev_state_t_LV_INDEV_STATE_RELEASED;
        }
    }
}

unsafe fn drop_input_handler<F>(user_data: *mut cty::c_void) {
    unsafe {
        drop(Box::<F>::from_raw(user_data as *mut F));
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Display;

    #[test]
    fn pointer_input_device() {
        crate::tests::initialize_test(true);
        let display = Display::default();

        fn read_touchpad_device() -> BufferStatus {
            PointerInputData::Touch(Point::new(120, 23))
                .pressed()
                .once()
        }

        let _touch_screen = Pointer::register(read_touchpad_device, &display).unwrap();
    }
}
