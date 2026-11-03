use super::{frame::Frame, HEIGHT, WIDTH};
use escope_agent::Backend;
use escope_lvgl::{native::NativeTree, UiBridge};
use escope_protocol::{self as p, capability, decode_payload, encode_payload, Operation, Status};
use rs_lvgl::{
    input_device::{InputDriver, Pointer, PointerInputData},
    point::Point,
};
use std::{cell::Cell, cell::RefCell, ptr::NonNull, rc::Rc, time::Instant};

#[derive(Clone, Copy)]
struct Sample {
    x: i32,
    y: i32,
    pressed: bool,
}

struct Capture {
    id: u32,
    pixels: Vec<u8>,
}

pub struct Device {
    tree: UiBridge<NativeTree<128>>,
    pixels: Rc<RefCell<Frame>>,
    pointer: Pointer,
    sample: Rc<Cell<Sample>>,
    release_at: Option<Instant>,
    capture: Option<Capture>,
    next_capture: u32,
    started: Instant,
    boot: u32,
    pub dropped_logs: u64,
}

impl Device {
    pub fn new(display: &rs_lvgl::Display, pixels: Rc<RefCell<Frame>>, boot: u32) -> Self {
        let sample = Rc::new(Cell::new(Sample {
            x: 0,
            y: 0,
            pressed: false,
        }));
        let input = sample.clone();
        let pointer = Pointer::register(
            move || {
                let s = input.get();
                let point = PointerInputData::Touch(Point::new(s.x, s.y));
                if s.pressed {
                    point.pressed()
                } else {
                    point.released()
                }
                .once()
            },
            display,
        )
        .expect("pointer");
        // SAFETY: the active screen lives until after Inspector and Device drop.
        // Construction, every read and teardown occur on this sole UI thread.
        let tree = unsafe {
            NativeTree::new(NonNull::new(rs_lvgl::sys::lv_screen_active()).expect("screen"))
        };
        Self {
            tree: UiBridge::new(tree),
            pixels,
            pointer,
            sample,
            release_at: None,
            capture: None,
            next_capture: 1,
            started: Instant::now(),
            boot,
            dropped_logs: 0,
        }
    }

    pub fn uptime(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    pub fn changed(&mut self) -> Result<(), Status> {
        self.tree.source_mut().changed()
    }

    pub fn release_due(&mut self) {
        if self
            .release_at
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.release_at = None;
            self.sample.set(Sample {
                pressed: false,
                ..self.sample.get()
            });
            self.read_pointer();
        }
    }

    fn read_pointer(&mut self) {
        // Pressed flags are part of NativeTree output even before a view update.
        self.changed()
            .expect("tree revision exhausted; restart the example");
        // SAFETY: live pointer/display, UI-loop safe point, no callback is active.
        // Native callbacks enqueue messages; the outer loop drains them later.
        unsafe { rs_lvgl::sys::lv_indev_read(self.pointer.get_descriptor().unwrap()) };
    }
}

fn reply<T: p::minicbor::Encode<()>>(value: &T, out: &mut [u8]) -> Result<usize, Status> {
    encode_payload(value, out).map_err(|_| Status::Capacity)
}

impl Backend for Device {
    fn capabilities(&self) -> u32 {
        capability::LOGS
            | capability::METRICS
            | capability::UI_TREE
            | capability::INPUT
            | capability::CAPTURE
    }

    fn info(&self) -> p::DeviceInfo<'_> {
        p::DeviceInfo {
            name: "rs_lvgl native example",
            model: "LVGL 9.6 / memory display",
            firmware: env!("CARGO_PKG_VERSION"),
            build_id: "rs-lvgl-0.3.1+escope-0.1.0",
            boot_id: self.boot,
            uptime_ms: self.uptime(),
        }
    }

    fn reset(&mut self) {
        self.capture = None;
        self.release_at = None;
        self.sample.set(Sample {
            pressed: false,
            ..self.sample.get()
        });
        // Complete an accepted tap with release, including a short-lived CLI
        // connection. This can enqueue its click; it never leaves a held input.
        self.read_pointer();
    }

    fn handle(&mut self, op: u16, payload: &[u8], out: &mut [u8]) -> Result<usize, Status> {
        match Operation::try_from(op)? {
            Operation::Metrics => {
                if !payload.is_empty() {
                    return Err(Status::InvalidArgument);
                }
                reply(
                    &p::Metrics {
                        uptime_ms: self.uptime(),
                        queue_depth: Some(0),
                        dropped_logs: self.dropped_logs,
                        ..Default::default()
                    },
                    out,
                )
            }
            Operation::UiTree => self.tree.tree(payload, out),
            Operation::InputTap => {
                let tap: p::InputTap =
                    decode_payload(payload).map_err(|_| Status::InvalidArgument)?;
                if tap.x >= WIDTH || tap.y >= HEIGHT || !(1..=5000).contains(&tap.hold_ms) {
                    return Err(Status::InvalidArgument);
                }
                if self.release_at.is_some() {
                    return Err(Status::Busy);
                }
                self.sample.set(Sample {
                    x: tap.x as i32,
                    y: tap.y as i32,
                    pressed: true,
                });
                self.release_at =
                    Some(Instant::now() + std::time::Duration::from_millis(tap.hold_ms.into()));
                self.read_pointer();
                Ok(0)
            }
            Operation::CaptureStart => {
                if !payload.is_empty() {
                    return Err(Status::InvalidArgument);
                }
                if self.capture.is_some() {
                    return Err(Status::Busy);
                }
                let frame = self.pixels.borrow();
                if frame.id == 0 {
                    return Err(Status::Busy);
                }
                let id = self.next_capture;
                self.next_capture = id.checked_add(1).ok_or(Status::Capacity)?;
                self.capture = Some(Capture {
                    id,
                    pixels: frame.pixels.clone(),
                });
                reply(
                    &p::CaptureMeta {
                        capture_id: id,
                        width: WIDTH,
                        height: HEIGHT,
                        stride: WIDTH * 3,
                        format: p::PixelFormat::Rgb888,
                        total_bytes: WIDTH * HEIGHT * 3,
                        frame_id: frame.id,
                    },
                    out,
                )
            }
            Operation::CaptureRead => {
                let query: p::CaptureRead =
                    decode_payload(payload).map_err(|_| Status::InvalidArgument)?;
                let capture = self
                    .capture
                    .as_ref()
                    .filter(|c| c.id == query.capture_id)
                    .ok_or(Status::Stale)?;
                let start = query.offset as usize;
                let end = start
                    .checked_add(query.length as usize)
                    .ok_or(Status::InvalidArgument)?;
                if query.length == 0 || query.length > 768 || end > capture.pixels.len() {
                    return Err(Status::InvalidArgument);
                }
                reply(
                    &p::CaptureChunk {
                        capture_id: capture.id,
                        offset: query.offset,
                        data: &capture.pixels[start..end],
                        done: end == capture.pixels.len(),
                    },
                    out,
                )
            }
            Operation::CaptureEnd => {
                let end: p::CaptureEnd =
                    decode_payload(payload).map_err(|_| Status::InvalidArgument)?;
                if !self
                    .capture
                    .as_ref()
                    .is_some_and(|c| c.id == end.capture_id)
                {
                    return Err(Status::Stale);
                }
                self.capture = None;
                Ok(0)
            }
            Operation::Cancel => {
                let cancel: p::Cancel =
                    decode_payload(payload).map_err(|_| Status::InvalidArgument)?;
                if cancel.request_id == 0 {
                    return Err(Status::InvalidArgument);
                }
                // Explicit Cancel suppresses a pending click. Disconnect
                // instead completes an already accepted tap.
                self.capture = None;
                self.release_at = None;
                self.sample.set(Sample {
                    pressed: false,
                    ..self.sample.get()
                });
                // SAFETY: live native input on its owning UI thread. The next
                // sample emits PRESS_LOST and clears native pressed state.
                unsafe {
                    rs_lvgl::sys::lv_indev_wait_release(self.pointer.get_descriptor().unwrap())
                };
                self.read_pointer();
                Ok(0)
            }
            _ => Err(Status::Unsupported),
        }
    }
}
