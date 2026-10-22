use crate::lv_core::style::Style;
use crate::{Align, LvError, LvResult};
use core::fmt::{self, Debug};
use core::marker::PhantomData;
use core::ptr::{self, NonNull};

pub trait NativeObject {
    fn raw(&self) -> NonNull<rs_lvgl_sys::lv_obj_t>;
}

pub struct Obj<'a> {
    raw: NonNull<rs_lvgl_sys::lv_obj_t>,
    dependents: PhantomData<&'a isize>,
}

impl Debug for Obj<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("NativeObject")
            .field("raw", &"!! LVGL lv_obj_t ptr !!")
            .finish()
    }
}

impl<'a> Obj<'a> {
    pub fn create(parent: &'a mut impl NativeObject) -> LvResult<Self> {
        unsafe {
            let ptr = rs_lvgl_sys::lv_obj_create(parent.raw().as_mut());
            if let Some(nn_ptr) = ptr::NonNull::new(ptr) {
                Ok(Self {
                    raw: nn_ptr,
                    dependents: PhantomData::<&'a _>,
                })
            } else {
                Err(LvError::InvalidReference)
            }
        }
    }

    pub fn new() -> crate::LvResult<Self> {
        let mut parent = crate::display::get_scr_act()?;
        Self::create(unsafe { &mut *(&mut parent as *mut _) })
    }

    pub fn blank() -> LvResult<Self> {
        match NonNull::new(unsafe { rs_lvgl_sys::lv_obj_create(ptr::null_mut()) }) {
            Some(raw) => Ok(Self {
                raw,
                dependents: PhantomData,
            }),
            None => Err(LvError::LvOOMemory),
        }
    }
}

impl NativeObject for Obj<'_> {
    fn raw(&self) -> ptr::NonNull<rs_lvgl_sys::lv_obj_t> {
        self.raw
    }
}

pub trait Widget<'a>: NativeObject + Sized + 'a {
    type SpecialEvent;
    type Part: Into<rs_lvgl_sys::lv_part_t>;

    unsafe fn from_raw(raw_pointer: ptr::NonNull<rs_lvgl_sys::lv_obj_t>) -> Option<Self>;

    fn add_style(&mut self, part: Self::Part, style: &'a mut Style) {
        unsafe {
            rs_lvgl_sys::lv_obj_add_style(
                self.raw().as_mut(),
                style.raw.as_mut() as *mut _,
                part.into(),
            );
        };
    }

    fn set_pos(&mut self, x: i32, y: i32) {
        unsafe {
            rs_lvgl_sys::lv_obj_set_pos(
                self.raw().as_mut(),
                x as rs_lvgl_sys::lv_coord_t,
                y as rs_lvgl_sys::lv_coord_t,
            );
        }
    }

    fn set_size(&mut self, w: i32, h: i32) {
        unsafe {
            rs_lvgl_sys::lv_obj_set_size(
                self.raw().as_mut(),
                w as rs_lvgl_sys::lv_coord_t,
                h as rs_lvgl_sys::lv_coord_t,
            );
        }
    }

    fn set_width(&mut self, w: u32) {
        unsafe {
            rs_lvgl_sys::lv_obj_set_width(self.raw().as_mut(), w as rs_lvgl_sys::lv_coord_t);
        }
    }

    fn set_height(&mut self, h: u32) {
        unsafe {
            rs_lvgl_sys::lv_obj_set_height(self.raw().as_mut(), h as rs_lvgl_sys::lv_coord_t);
        }
    }

    fn set_align(&mut self, align: Align, x_mod: i32, y_mod: i32) {
        unsafe {
            rs_lvgl_sys::lv_obj_align(
                self.raw().as_mut(),
                align.into(),
                x_mod as rs_lvgl_sys::lv_coord_t,
                y_mod as rs_lvgl_sys::lv_coord_t,
            );
        }
    }
}

impl<'a> Widget<'a> for Obj<'a> {
    type SpecialEvent = u32;
    type Part = Part;

    unsafe fn from_raw(raw: NonNull<rs_lvgl_sys::lv_obj_t>) -> Option<Self> {
        Some(Self {
            raw,
            dependents: PhantomData,
        })
    }
}

macro_rules! define_object {
    ($item:ident) => {
        define_object!($item, event = (), part = $crate::Part);
    };
    ($item:ident, event = $event_type:ty) => {
        define_object!($item, event = $event_type, part = $crate::Part);
    };
    ($item:ident, part = $part_type:ty) => {
        define_object!($item, event = (), part = $part_type);
    };
    ($item:ident, part = $part_type:ty, event = $event_type:ty) => {
        define_object!($item, event = $event_type, part = $part_type);
    };
    ($item:ident, event = $event_type:ty, part = $part_type:ty) => {
        #[derive(Debug)]
        pub struct $item<'a> {
            core: $crate::Obj<'a>,
        }

        impl<'a> $item<'a> {
            pub fn on_event<F>(&mut self, f: F) -> $crate::LvResult<()>
            where
                F: FnMut(Self, $crate::support::Event<<Self as $crate::Widget<'a>>::SpecialEvent>),
            {
                use $crate::NativeObject;
                unsafe {
                    let obj = self.raw().as_mut();
                    let user_data = $crate::Box::into_raw($crate::Box::new(f)) as *mut _;
                    rs_lvgl_sys::lv_obj_set_user_data(obj, user_data);

                    rs_lvgl_sys::lv_obj_add_event_cb(
                        obj,
                        Some($crate::support::event_callback::<'a, Self, F>),
                        rs_lvgl_sys::lv_event_code_t_LV_EVENT_ALL,
                        user_data,
                    );

                    rs_lvgl_sys::lv_obj_add_event_cb(
                        obj,
                        Some($crate::support::drop_event_callback::<F>),
                        rs_lvgl_sys::lv_event_code_t_LV_EVENT_DELETE,
                        user_data,
                    );
                }
                Ok(())
            }
        }

        impl $crate::NativeObject for $item<'_> {
            fn raw(&self) -> core::ptr::NonNull<rs_lvgl_sys::lv_obj_t> {
                self.core.raw()
            }
        }

        impl<'a> $crate::Widget<'a> for $item<'a> {
            type SpecialEvent = $event_type;
            type Part = $part_type;

            unsafe fn from_raw(
                raw_pointer: core::ptr::NonNull<rs_lvgl_sys::lv_obj_t>,
            ) -> Option<Self> {
                Some(Self {
                    core: $crate::Obj::from_raw(raw_pointer).unwrap(),
                })
            }
        }
    };
}

pub enum Part {
    Main,
    Scrollbar,
    Indicator,
    Knob,
    Selected,
    Items,
    Cursor,
    CustomFirst,
    Any,
}

impl Default for Part {
    fn default() -> Self {
        Self::Main
    }
}

impl From<Part> for rs_lvgl_sys::lv_part_t {
    fn from(self_: Part) -> rs_lvgl_sys::lv_part_t {
        match self_ {
            Part::Main => rs_lvgl_sys::lv_part_t_LV_PART_MAIN,
            Part::Scrollbar => rs_lvgl_sys::lv_part_t_LV_PART_SCROLLBAR,
            Part::Indicator => rs_lvgl_sys::lv_part_t_LV_PART_INDICATOR,
            Part::Knob => rs_lvgl_sys::lv_part_t_LV_PART_KNOB,
            Part::Selected => rs_lvgl_sys::lv_part_t_LV_PART_SELECTED,
            Part::Items => rs_lvgl_sys::lv_part_t_LV_PART_ITEMS,
            Part::Cursor => rs_lvgl_sys::lv_part_t_LV_PART_CURSOR,
            Part::CustomFirst => rs_lvgl_sys::lv_part_t_LV_PART_CUSTOM_FIRST,
            Part::Any => rs_lvgl_sys::lv_part_t_LV_PART_ANY,
        }
    }
}
