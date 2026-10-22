use super::{BufferStatus, Data, InputDriver, InputState};
use crate::Box;
use crate::{LvError, LvResult};

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum EncoderInputData {
    Press,
    LongPress,
    TurnLeft,
    TurnRight,
}

impl EncoderInputData {
    pub fn pressed(self) -> InputState {
        InputState::Pressed(Data::Encoder(self))
    }

    pub fn released(self) -> InputState {
        InputState::Released(Data::Encoder(self))
    }
}

pub struct Encoder {
    descriptor: Option<*mut rs_lvgl_sys::lv_indev_t>,
    user_data_drop: Option<unsafe fn(*mut cty::c_void)>,
}

impl InputDriver<Encoder> for Encoder {
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
                rs_lvgl_sys::lv_indev_type_t_LV_INDEV_TYPE_ENCODER,
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

impl Drop for Encoder {
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
                fill_encoder_data(data, b);
                false
            }
            BufferStatus::Buffered(b) => {
                fill_encoder_data(data, b);
                true
            }
        }
    }
}

unsafe fn fill_encoder_data(data: *mut rs_lvgl_sys::lv_indev_data_t, state: InputState) {
    match state {
        InputState::Pressed(Data::Encoder(d)) => {
            (*data).key = match d {
                EncoderInputData::Press => rs_lvgl_sys::lv_key_t_LV_KEY_ENTER,
                EncoderInputData::LongPress => rs_lvgl_sys::lv_key_t_LV_KEY_ENTER,
                EncoderInputData::TurnLeft => rs_lvgl_sys::lv_key_t_LV_KEY_LEFT,
                EncoderInputData::TurnRight => rs_lvgl_sys::lv_key_t_LV_KEY_RIGHT,
            };
            (*data).state = rs_lvgl_sys::lv_indev_state_t_LV_INDEV_STATE_PRESSED;
        }
        InputState::Released(Data::Encoder(d)) => {
            (*data).key = match d {
                EncoderInputData::Press => rs_lvgl_sys::lv_key_t_LV_KEY_ENTER,
                EncoderInputData::LongPress => rs_lvgl_sys::lv_key_t_LV_KEY_ENTER,
                EncoderInputData::TurnLeft => rs_lvgl_sys::lv_key_t_LV_KEY_LEFT,
                EncoderInputData::TurnRight => rs_lvgl_sys::lv_key_t_LV_KEY_RIGHT,
            };
            (*data).state = rs_lvgl_sys::lv_indev_state_t_LV_INDEV_STATE_RELEASED;
        }
        _ => panic!("Non-encoder data returned from encoder device!"),
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
    fn encoder_input_device() {
        crate::tests::initialize_test(true);
        let display = Display::default();

        fn read_encoder_device() -> BufferStatus {
            EncoderInputData::Press.pressed().once()
        }

        let _encoder = Encoder::register(read_encoder_device, &display).unwrap();
    }
}
