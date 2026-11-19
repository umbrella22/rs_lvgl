//! Application-owned diagnostics; entirely absent without `embedscope`.
mod device;
mod evidence;
mod frame;
mod tree;
mod wire;

use super::{BrightnessCase, HEIGHT, WIDTH};
use escope_agent::Agent;
pub use evidence::Decision;
pub use frame::Frame;
use rs_lvgl_ui::{DiagnosticEvent, DiagnosticSink};
use std::{
    cell::RefCell,
    collections::VecDeque,
    io,
    net::{SocketAddr, TcpListener},
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Default)]
struct Logs {
    entries: VecDeque<String>,
    dropped: u64,
}

pub struct Inspector {
    listener: TcpListener,
    wire: Option<wire::Wire>,
    agent: Agent,
    device: device::Device,
    pixels: Rc<RefCell<Frame>>,
    logs: Rc<RefCell<Logs>>,
    last_count: u32,
}

impl Inspector {
    pub fn new(
        address: SocketAddr,
        display: &rs_lvgl::Display,
        pixels: Rc<RefCell<Frame>>,
        case: BrightnessCase,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;
        let seed = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u32)
            .max(1);
        let device = device::Device::new(display, pixels.clone(), seed, case);
        println!("LISTEN tcp://{}", listener.local_addr()?);
        Ok(Self {
            listener,
            wire: None,
            agent: Agent::new(seed),
            device,
            pixels,
            logs: Rc::new(RefCell::new(Logs::default())),
            last_count: 0,
        })
    }

    pub fn sink(&self) -> DiagnosticSink {
        let logs = self.logs.clone();
        Rc::new(move |event| {
            if matches!(
                event,
                DiagnosticEvent::Error { .. } | DiagnosticEvent::IgnoredProperty(_)
            ) {
                let mut logs = logs.borrow_mut();
                if logs.entries.len() < 16 {
                    logs.entries.push_back(format!("{event:?}"));
                } else {
                    logs.dropped = logs.dropped.saturating_add(1);
                }
            }
        })
    }

    pub fn poll(&mut self) -> io::Result<()> {
        self.device.release_due();
        if self.wire.is_none() {
            match self.listener.accept() {
                Ok((stream, _)) => self.wire = Some(wire::Wire::new(stream)?),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e),
            }
        }
        let mut logs = self.logs.borrow_mut();
        self.agent
            .record_dropped_logs(std::mem::take(&mut logs.dropped));
        while let Some(message) = logs.entries.pop_front() {
            let _ = self
                .agent
                .push_log(self.device.uptime(), 2, "rs_lvgl_ui.diagnostic", &message);
        }
        drop(logs);
        self.device.dropped_logs = self.agent.dropped_logs();
        if let Some(wire) = self.wire.as_mut() {
            if wire.poll(&mut self.agent, &mut self.device).is_err() {
                self.wire = None;
                self.agent.reset(&mut self.device);
            }
        }
        Ok(())
    }

    pub fn take_cancelled_brightness(&mut self) -> bool {
        std::mem::take(&mut self.device.cancelled_brightness)
    }

    pub fn rendered(
        &mut self,
        changed: bool,
        count: u32,
        decision: Option<Decision>,
    ) -> io::Result<()> {
        self.pixels.borrow_mut().finish();
        if changed {
            self.device
                .changed()
                .map_err(|e| io::Error::other(format!("tree revision: {e:?}")))?;
        }
        if count != self.last_count {
            self.last_count = count;
            let message = format!("counter={count}");
            let _ = self
                .agent
                .push_log(self.device.uptime(), 3, "example.counter", &message);
        }
        if let Some(outcome) = self
            .device
            .observe(decision)
            .map_err(|e| io::Error::other(format!("preview evidence: {e:?}")))?
        {
            let (result, frame) = outcome.log_fields();
            let message =
                serde_json::json!({"control": tree::BRIGHTNESS, "result": result, "frame": frame});
            let _ = self.agent.push_log(
                self.device.uptime(),
                3,
                evidence::LOG_TARGET,
                &message.to_string(),
            );
        }
        Ok(())
    }
}

impl Drop for Inspector {
    fn drop(&mut self) {
        self.wire = None;
        self.agent.reset(&mut self.device);
    }
}
