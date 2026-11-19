use super::frame::Frame;
use std::time::{Duration, Instant};

const PRESENTATION_WINDOW: Duration = Duration::from_millis(250);
pub(super) const LOG_TARGET: &str = "escope.ui.evidence.v1";

#[derive(Clone, Copy)]
pub enum Decision {
    Rejected,
    Unchanged,
    Accepted([u8; 3]),
}

pub(super) enum Outcome {
    InputUnobserved,
    Rejected,
    Unchanged,
    PreviewUnconfirmed,
    Presented(u64),
}

impl Outcome {
    pub(super) fn log_fields(&self) -> (&'static str, Option<u64>) {
        match *self {
            Self::InputUnobserved => ("input-unobserved", None),
            Self::Rejected => ("rejected", None),
            Self::Unchanged => ("unchanged", None),
            Self::PreviewUnconfirmed => ("preview-unconfirmed", None),
            Self::Presented(frame) => ("presented", Some(frame)),
        }
    }
}

pub(super) struct Attempt {
    before: u64,
    center: (usize, usize),
    before_rgb: Option<[u8; 3]>,
    decision: Option<Decision>,
    release_observed_at: Option<Instant>,
}

impl Attempt {
    pub(super) fn new(frame: &Frame, center: (usize, usize)) -> Self {
        Self {
            before: frame.id,
            center,
            before_rgb: frame.pixel(center.0, center.1).filter(|_| frame.id != 0),
            decision: None,
            release_observed_at: None,
        }
    }

    pub(super) fn observe(
        &mut self,
        decision: Option<Decision>,
        released: bool,
        completed: &Frame,
        center: (usize, usize),
        now: Instant,
    ) -> Option<Outcome> {
        if let Some(decision) = decision {
            if !matches!(decision, Decision::Unchanged) || self.decision.is_none() {
                self.decision = Some(decision);
            }
        }
        if !released {
            return None;
        }
        let released = *self.release_observed_at.get_or_insert(now);
        match self.decision {
            None => Some(Outcome::InputUnobserved),
            Some(Decision::Rejected) => Some(Outcome::Rejected),
            Some(Decision::Unchanged) => Some(Outcome::Unchanged),
            Some(Decision::Accepted(rgb))
                if completed.id > self.before
                    && self.center == center
                    && self.before_rgb.is_some_and(|before| before != rgb)
                    && completed.pixel(center.0, center.1) == Some(rgb) =>
            {
                Some(Outcome::Presented(completed.id))
            }
            Some(Decision::Accepted(_)) if now.duration_since(released) >= PRESENTATION_WINDOW => {
                Some(Outcome::PreviewUnconfirmed)
            }
            Some(Decision::Accepted(_)) => None,
        }
    }
}
