#![no_std]
extern crate alloc;
use alloc::boxed::Box;
use core::ptr::NonNull;
use rs_lvgl_ui::prelude::*;
pub struct Gauge;
pub struct GaugeProps {
    pub value: u8,
}
impl CustomWidget for Gauge {
    type Props = GaugeProps;
    fn validate(props: &GaugeProps) -> Result<(), UiError> {
        if props.value <= 100 {
            Ok(())
        } else {
            Err(UiError::InvalidProperty)
        }
    }
    fn create(&mut self, parent: &mut Obj<'_>, _: &GaugeProps) -> Result<CustomObject, UiError> {
        // SAFETY: the new native direct child is returned to the mounted owner.
        unsafe {
            let child = NonNull::new(rs_lvgl_sys::lv_obj_create(parent.raw().as_ptr()))
                .ok_or(UiError::Allocation)?;
            CustomObject::new(parent, child)
        }
    }
    fn apply_props(&mut self, obj: &mut CustomObject, props: &GaugeProps) -> Result<(), UiError> {
        // SAFETY: the root is borrowed for this UI-thread property hook.
        unsafe {
            rs_lvgl_sys::lv_obj_set_style_opa(obj.raw().as_ptr(), props.value, 0);
        }
        Ok(())
    }
    fn tick(&mut self, _: &mut CustomObject, _: u32) -> Result<(), UiError> {
        Ok(())
    }
    fn delete(&mut self, _: &mut CustomObject) {}
}
pub fn screen() -> View<()> {
    custom(Gauge, GaugeProps { value: 55 })
        .key(Key::Id(1))
        .map(|()| ())
}
pub fn root() -> View<()> {
    rs_lvgl_ui::custom(Box::new(Gauge), GaugeProps { value: 65 })
}
