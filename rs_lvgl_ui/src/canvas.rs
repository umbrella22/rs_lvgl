//! Declarative ARGB8888 canvas drawing.
use crate::{Color, Font, Node, UiError, WidgetKind};
use alloc::{string::String, vec::Vec};
use core::ptr::NonNull;

/// Commands draw in order, in canvas pixels; drawing outside the canvas is
/// clipped. Pixel coordinates themselves must be inside its dimensions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DrawCommand {
    Pixel {
        x: u16,
        y: u16,
        color: Color,
        opacity: u8,
    },
    Rect {
        x: i16,
        y: i16,
        width: u16,
        height: u16,
        radius: u16,
        color: Color,
        opacity: u8,
    },
    Line {
        from: (i16, i16),
        to: (i16, i16),
        width: u16,
        color: Color,
        opacity: u8,
    },
    Text {
        x: i16,
        y: i16,
        width: u16,
        height: u16,
        text: String,
        font: Font,
        color: Color,
        opacity: u8,
    },
}

/// A full canvas frame. Changed descriptors redraw the existing native object;
/// resized frames replace its aligned buffer before releasing the previous one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Canvas {
    pub width: u16,
    pub height: u16,
    pub background: Color,
    pub opacity: u8,
    pub commands: Vec<DrawCommand>,
}
impl Canvas {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            background: Color::rgb(0, 0, 0),
            opacity: 0,
            commands: Vec::new(),
        }
    }
    pub fn background(mut self, color: Color, opacity: u8) -> Self {
        self.background = color;
        self.opacity = opacity;
        self
    }
    pub fn draw(mut self, command: DrawCommand) -> Self {
        self.commands.push(command);
        self
    }
    pub(crate) fn validate(&self) -> Result<(), UiError> {
        crate::graphics::DrawBuffer::stride(self.width, self.height)?;
        for command in &self.commands {
            let valid = match command {
                DrawCommand::Pixel { x, y, .. } => *x < self.width && *y < self.height,
                DrawCommand::Rect { width, height, .. } => *width > 0 && *height > 0,
                DrawCommand::Line { width, .. } => *width > 0,
                DrawCommand::Text {
                    width,
                    height,
                    text,
                    font,
                    ..
                } => {
                    *width > 0 && *height > 0 && !text.as_bytes().contains(&0) && !font.0.is_null()
                }
            };
            if !valid {
                return Err(UiError::InvalidProperty);
            }
        }
        Ok(())
    }
}
impl<M> Node<M> {
    pub fn canvas(frame: Canvas) -> Self {
        Self::new(WidgetKind::Canvas { frame })
    }
}

pub(crate) fn draw(obj: NonNull<rs_lvgl_sys::lv_obj_t>, frame: &Canvas) {
    // SAFETY: validation completed and the object references an aligned native
    // buffer. Each layer finishes synchronously before any local text or layer
    // storage is released. This also preserves command order around pixel writes.
    unsafe {
        rs_lvgl_sys::lv_canvas_fill_bg(obj.as_ptr(), frame.background.to_raw(), frame.opacity);
        for command in &frame.commands {
            if let DrawCommand::Pixel {
                x,
                y,
                color,
                opacity,
            } = command
            {
                rs_lvgl_sys::lv_canvas_set_px(
                    obj.as_ptr(),
                    (*x).into(),
                    (*y).into(),
                    color.to_raw(),
                    *opacity,
                );
                continue;
            }
            let mut layer = core::mem::zeroed();
            rs_lvgl_sys::lv_canvas_init_layer(obj.as_ptr(), &mut layer);
            let mut text_storage = None;
            match command {
                DrawCommand::Rect {
                    x,
                    y,
                    width,
                    height,
                    radius,
                    color,
                    opacity,
                } => {
                    let mut dsc = core::mem::zeroed();
                    rs_lvgl_sys::lv_draw_rect_dsc_init(&mut dsc);
                    dsc.bg_color = color.to_raw();
                    dsc.bg_opa = *opacity;
                    dsc.radius = (*radius).into();
                    rs_lvgl_sys::lv_draw_rect(&mut layer, &dsc, &area(*x, *y, *width, *height));
                }
                DrawCommand::Line {
                    from,
                    to,
                    width,
                    color,
                    opacity,
                } => {
                    let mut dsc = core::mem::zeroed();
                    rs_lvgl_sys::lv_draw_line_dsc_init(&mut dsc);
                    dsc.p1.x = from.0 as _;
                    dsc.p1.y = from.1 as _;
                    dsc.p2.x = to.0 as _;
                    dsc.p2.y = to.1 as _;
                    dsc.width = (*width).into();
                    dsc.color = color.to_raw();
                    dsc.opa = *opacity;
                    rs_lvgl_sys::lv_draw_line(&mut layer, &dsc);
                }
                DrawCommand::Text {
                    x,
                    y,
                    width,
                    height,
                    text,
                    font,
                    color,
                    opacity,
                } => {
                    text_storage =
                        Some(cstr_core::CString::new(text.as_str()).expect("validated text"));
                    let mut dsc = core::mem::zeroed();
                    rs_lvgl_sys::lv_draw_label_dsc_init(&mut dsc);
                    dsc.text = text_storage.as_ref().unwrap().as_ptr();
                    dsc.font = font.0;
                    dsc.color = color.to_raw();
                    dsc.opa = *opacity;
                    rs_lvgl_sys::lv_draw_label(&mut layer, &dsc, &area(*x, *y, *width, *height));
                }
                DrawCommand::Pixel { .. } => unreachable!(),
            }
            rs_lvgl_sys::lv_canvas_finish_layer(obj.as_ptr(), &mut layer);
            drop(text_storage);
        }
        rs_lvgl_sys::lv_obj_invalidate(obj.as_ptr());
    }
}
fn area(x: i16, y: i16, width: u16, height: u16) -> rs_lvgl_sys::lv_area_t {
    rs_lvgl_sys::lv_area_t {
        x1: x.into(),
        y1: y.into(),
        x2: i32::from(x) + i32::from(width) - 1,
        y2: i32::from(y) + i32::from(height) - 1,
    }
}
