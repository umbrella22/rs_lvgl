//! One bounded nonblocking connection. A partial TX always keeps its suffix.
use escope_agent::{Agent, Backend};
use escope_protocol::{self as p, FrameDecoder, MAX_FRAME};
use std::{
    io::{self, Read, Write},
    net::TcpStream,
    time::{Duration, Instant},
};

pub struct Wire {
    stream: TcpStream,
    decoder: FrameDecoder,
    scratch: [u8; MAX_FRAME],
    tx: [u8; MAX_FRAME],
    length: usize,
    sent: usize,
    pending_since: Instant,
}

impl Wire {
    pub fn new(stream: TcpStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        stream.set_nodelay(true)?;
        Ok(Self {
            stream,
            decoder: FrameDecoder::new(),
            scratch: [0; MAX_FRAME],
            tx: [0; MAX_FRAME],
            length: 0,
            sent: 0,
            pending_since: Instant::now(),
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        while self.sent < self.length {
            match self.stream.write(&self.tx[self.sent..self.length]) {
                Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                Ok(n) => self.sent += n,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    if self.pending_since.elapsed() > Duration::from_secs(3) {
                        return Err(io::ErrorKind::TimedOut.into());
                    }
                    return Ok(());
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        self.length = 0;
        self.sent = 0;
        Ok(())
    }

    pub fn poll(&mut self, agent: &mut Agent, device: &mut impl Backend) -> io::Result<()> {
        self.flush()?;
        if self.length != 0 {
            return Ok(());
        }
        // Bound protocol work to one response and at most 2048 RX bytes/tick.
        let mut byte = [0];
        for _ in 0..2048 {
            match self.stream.read(&mut byte) {
                Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
                Ok(_) => {
                    if let Some(Ok(frame)) = self.decoder.push(byte[0]) {
                        if let Ok(request) = p::decode_frame(frame, &mut self.scratch) {
                            self.length = agent
                                .handle(request, device, &mut self.tx)
                                .map_err(io::Error::other)?;
                            break;
                        }
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        if self.length == 0 {
            self.length = agent
                .poll_event(&mut self.tx)
                .map_err(io::Error::other)?
                .unwrap_or(0);
        }
        self.pending_since = Instant::now();
        self.flush()
    }
}
