//! Bounded application state. Snapshots are values, never consumable commands.

use crate::chart::ChartValue;
use alloc::{collections::VecDeque, rc::Rc, vec::Vec};
use core::num::NonZeroU32;

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartSampleKind {
    Y,
    XY,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartSample {
    Y(ChartValue),
    XY { x: ChartValue, y: ChartValue },
}

impl ChartSample {
    pub(crate) fn pair(self) -> (ChartValue, ChartValue) {
        match self {
            Self::Y(y) => (ChartValue::Missing, y),
            Self::XY { x, y } => (x, y),
        }
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartHistoryError {
    WrongSampleKind,
    ReservedValue,
    SequenceExhausted,
    EpochUnchanged,
}

/// A bounded chronological history belonging to the application.
///
/// `epoch` identifies one stream incarnation. Use a distinct epoch after clear,
/// replacement or sequence exhaustion. Clones are independent application state;
/// an adapter still checks their overlapping samples before choosing append.
#[derive(Clone, Debug)]
pub struct ChartHistory {
    epoch: u64,
    capacity: NonZeroU32,
    kind: ChartSampleKind,
    next: u64,
    samples: VecDeque<ChartSample>,
}

impl ChartHistory {
    pub fn new(epoch: u64, capacity: NonZeroU32, kind: ChartSampleKind) -> Self {
        Self::starting_at(epoch, capacity, kind, 0)
    }

    /// Start an empty history at an explicit sequence, useful when sequence
    /// identity is shared with an application data source. No capacity-sized
    /// allocation occurs before samples arrive.
    pub fn starting_at(
        epoch: u64,
        capacity: NonZeroU32,
        kind: ChartSampleKind,
        next_sequence: u64,
    ) -> Self {
        Self {
            epoch,
            capacity,
            kind,
            next: next_sequence,
            samples: VecDeque::new(),
        }
    }

    pub fn push_y(&mut self, value: impl Into<ChartValue>) -> Result<u64, ChartHistoryError> {
        self.push(ChartSample::Y(value.into()))
    }

    pub fn push_xy(
        &mut self,
        x: impl Into<ChartValue>,
        y: impl Into<ChartValue>,
    ) -> Result<u64, ChartHistoryError> {
        self.push(ChartSample::XY {
            x: x.into(),
            y: y.into(),
        })
    }

    fn push(&mut self, sample: ChartSample) -> Result<u64, ChartHistoryError> {
        if !matches!(
            (self.kind, sample),
            (ChartSampleKind::Y, ChartSample::Y(_)) | (ChartSampleKind::XY, ChartSample::XY { .. })
        ) {
            return Err(ChartHistoryError::WrongSampleKind);
        }
        let (x, y) = sample.pair();
        if x.validate().is_err() || y.validate().is_err() {
            return Err(ChartHistoryError::ReservedValue);
        }
        let next = self
            .next
            .checked_add(1)
            .ok_or(ChartHistoryError::SequenceExhausted)?;
        let sequence = self.next;
        if self.samples.len() == self.capacity.get() as usize {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
        self.next = next;
        Ok(sequence)
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
    pub fn capacity(&self) -> NonZeroU32 {
        self.capacity
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn next_sequence(&self) -> u64 {
        self.next
    }

    /// Preserve the newest samples and their sequences. A changed capacity
    /// forces full native synchronization on the next render.
    pub fn set_capacity(&mut self, capacity: NonZeroU32) {
        self.capacity = capacity;
        while self.samples.len() > capacity.get() as usize {
            self.samples.pop_front();
        }
        // VecDeque's spare allocation is bounded by its historical capacity.
        // Shrinking releases it rather than retaining an earlier larger buffer.
        self.samples.shrink_to_fit();
    }

    pub fn clear(&mut self, new_epoch: u64) -> Result<(), ChartHistoryError> {
        if self.epoch == new_epoch {
            return Err(ChartHistoryError::EpochUnchanged);
        }
        self.epoch = new_epoch;
        self.next = 0;
        self.samples.clear();
        Ok(())
    }

    /// Copy the bounded canonical state once. Cloning the resulting immutable
    /// snapshot for repeated views only clones an Rc; no mutation is exposed.
    pub fn snapshot(&self) -> ChartHistorySnapshot {
        ChartHistorySnapshot(Rc::new(SnapshotData {
            epoch: self.epoch,
            capacity: self.capacity,
            kind: self.kind,
            next: self.next,
            samples: self.samples.iter().copied().collect(),
        }))
    }
}

#[derive(Debug, Eq, PartialEq)]
struct SnapshotData {
    epoch: u64,
    capacity: NonZeroU32,
    kind: ChartSampleKind,
    next: u64,
    samples: Vec<ChartSample>,
}

/// Immutable, owned snapshot. Metadata cannot be forged independently of data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChartHistorySnapshot(Rc<SnapshotData>);

impl ChartHistorySnapshot {
    pub fn epoch(&self) -> u64 {
        self.0.epoch
    }
    pub fn capacity(&self) -> NonZeroU32 {
        self.0.capacity
    }
    pub fn kind(&self) -> ChartSampleKind {
        self.0.kind
    }
    pub fn next_sequence(&self) -> u64 {
        self.0.next
    }
    pub fn first_sequence(&self) -> u64 {
        self.0.next - self.0.samples.len() as u64
    }
    pub fn samples(&self) -> &[ChartSample] {
        &self.0.samples
    }

    pub(crate) fn displayed(
        &self,
        index: usize,
        mode: crate::ChartUpdateMode,
    ) -> (ChartValue, ChartValue) {
        let capacity = self.capacity().get() as usize;
        let offset = match mode {
            crate::ChartUpdateMode::Shift => index.checked_sub(capacity - self.samples().len()),
            crate::ChartUpdateMode::Circular => {
                let first_slot = (self.first_sequence() % capacity as u64) as usize;
                let offset = (index + capacity - first_slot) % capacity;
                (offset < self.samples().len()).then_some(offset)
            }
        };
        offset
            .and_then(|i| self.samples().get(i))
            .copied()
            .map(ChartSample::pair)
            .unwrap_or((ChartValue::Missing, ChartValue::Missing))
    }

    /// Return the first unseen sample only when metadata and retained overlap
    /// prove continuity. Empty initial progress is the sole no-overlap case.
    pub(crate) fn appended_since(&self, old: &Self) -> Option<&[ChartSample]> {
        if self.epoch() != old.epoch()
            || self.capacity() != old.capacity()
            || self.kind() != old.kind()
            || self.next_sequence() < old.next_sequence()
            || self.first_sequence() > old.next_sequence()
        {
            return None;
        }
        if old.samples().is_empty() && self.first_sequence() != old.next_sequence() {
            return None;
        }
        let first = self.first_sequence().max(old.first_sequence());
        let end = old.next_sequence();
        if first >= end && !old.samples().is_empty() {
            return None;
        }
        let new_offset = usize::try_from(first - self.first_sequence()).ok()?;
        let old_offset = usize::try_from(first - old.first_sequence()).ok()?;
        let overlap = usize::try_from(end - first).ok()?;
        if self.samples().get(new_offset..new_offset + overlap)?
            != old.samples().get(old_offset..old_offset + overlap)?
        {
            return None;
        }
        self.samples()
            .get(usize::try_from(end - self.first_sequence()).ok()?..)
    }
}
