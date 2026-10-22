use crate::{LvError, LvResult, NativeObject, Obj, Part, Widget};

#[derive(Debug)]
pub struct Screen<'a> {
    raw: Obj<'a>,
}

impl Screen<'_> {
    pub fn blank() -> LvResult<Self> {
        Ok(Self { raw: Obj::blank()? })
    }
}

impl NativeObject for Screen<'_> {
    fn raw(&self) -> core::ptr::NonNull<rs_lvgl_sys::lv_obj_t> {
        self.raw.raw()
    }
}

impl<'a> Widget<'a> for Screen<'a> {
    type SpecialEvent = u32;
    type Part = Part;

    unsafe fn from_raw(raw: core::ptr::NonNull<rs_lvgl_sys::lv_obj_t>) -> Option<Self> {
        match Self::try_from(Obj::from_raw(raw)?) {
            Ok(s) => Some(s),
            Err(_) => None,
        }
    }
}

impl<'a> TryFrom<Obj<'a>> for Screen<'a> {
    type Error = LvError;

    fn try_from(value: Obj<'a>) -> Result<Self, Self::Error> {
        let parent = unsafe { rs_lvgl_sys::lv_obj_get_parent(value.raw().as_ptr()) };
        if parent.is_null() {
            Ok(Self { raw: value })
        } else {
            Err(LvError::InvalidReference)
        }
    }
}

#[allow(clippy::from_over_into)]
impl<'a> Into<Obj<'a>> for Screen<'a> {
    fn into(self) -> Obj<'a> {
        self.raw
    }
}

impl<'a> AsRef<Obj<'a>> for Screen<'a> {
    fn as_ref(&self) -> &Obj<'a> {
        &self.raw
    }
}

impl<'a> AsMut<Obj<'a>> for Screen<'a> {
    fn as_mut(&mut self) -> &mut Obj<'a> {
        &mut self.raw
    }
}
