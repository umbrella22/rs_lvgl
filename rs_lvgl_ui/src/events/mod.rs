//! Native payloads, bounded message queues and callback registrations.

mod queue;
mod slot;
mod types;
mod value;

pub(crate) use queue::EventQueue;
pub use queue::{PostOutcome, QueueOverflow, QueuePolicy, QueueStats};
pub(crate) use slot::HandlerSlot;
pub(crate) use types::map_event;
pub use types::{Event, GestureDirection, InputEvent, PointerEvent, ScrollOffset};
pub(crate) use value::ValueSource;
