//! Bounded queue policy, enqueue outcomes and native event observers.

use alloc::{collections::VecDeque, rc::Rc};
use core::cell::{Cell, RefCell};

/// Behavior when a bounded queue has no space and no tail merge applies.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueueOverflow {
    DropOldest,
    RejectNew,
}

/// An explicit queue policy. Only the tail is considered for coalescing, so a
/// merge cannot cross an intervening message. The merger must check identity,
/// including event source identity when it affects the application's semantics.
pub struct QueuePolicy<M> {
    overflow: QueueOverflow,
    merge: Option<fn(&M, &M) -> Option<M>>,
}

impl<M> Copy for QueuePolicy<M> {}
impl<M> Clone for QueuePolicy<M> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<M> Default for QueuePolicy<M> {
    fn default() -> Self {
        Self::new(QueueOverflow::DropOldest)
    }
}
impl<M> QueuePolicy<M> {
    pub const fn new(overflow: QueueOverflow) -> Self {
        Self {
            overflow,
            merge: None,
        }
    }

    /// Supply an application-defined identity and merge operation. `None`
    /// preserves both messages in order; `Some` replaces the tail. A merger
    /// used by native handlers must not unwind, block or reenter the UI.
    pub const fn coalesce(mut self, merge: fn(&M, &M) -> Option<M>) -> Self {
        self.merge = Some(merge);
        self
    }
}

/// Result of one enqueue attempt. An eviction still successfully enqueues the
/// new message; retrying it would duplicate delivery. Rejection returns it.
#[derive(Debug, Eq, PartialEq)]
pub enum PostOutcome<M> {
    Enqueued,
    Evicted(M),
    Merged,
    Rejected(M),
}

/// Cumulative congestion counters, including native-handler messages.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct QueueStats {
    pub evicted: u64,
    pub rejected: u64,
    pub merged: u64,
}

/// Bounded message queue shared by the `Ui` and every mounted handler slot.
pub(crate) struct EventQueue<M> {
    pub(crate) diagnostics: crate::diagnostics::Diagnostics,
    items: RefCell<VecDeque<M>>,
    capacity: usize,
    dropped: Cell<u64>,
    stats: Cell<QueueStats>,
    event_policy: Cell<QueuePolicy<M>>,
    event_observer: RefCell<Option<Rc<dyn Fn(PostOutcome<M>)>>>,
}

impl<M> EventQueue<M> {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            diagnostics: crate::diagnostics::Diagnostics::default(),
            items: RefCell::new(VecDeque::with_capacity(capacity)),
            capacity,
            dropped: Cell::new(0),
            stats: Cell::new(QueueStats::default()),
            event_policy: Cell::new(QueuePolicy::default()),
            event_observer: RefCell::new(None),
        }
    }

    pub(crate) fn post(&self, msg: M) {
        let _ = self.post_with(msg, QueuePolicy::default());
    }

    pub(crate) fn post_with(&self, msg: M, policy: QueuePolicy<M>) -> PostOutcome<M> {
        let mut items = self.items.borrow_mut();
        let mut stats = self.stats.get();
        if let (Some(merge), Some(tail)) = (policy.merge, items.back_mut()) {
            if let Some(merged) = merge(tail, &msg) {
                *tail = merged;
                stats.merged += 1;
                self.stats.set(stats);
                return PostOutcome::Merged;
            }
        }
        if items.len() == self.capacity {
            if policy.overflow == QueueOverflow::RejectNew {
                stats.rejected += 1;
                self.stats.set(stats);
                return PostOutcome::Rejected(msg);
            }
            let evicted = items.pop_front().expect("full nonempty queue");
            self.dropped.set(self.dropped.get() + 1);
            stats.evicted += 1;
            self.stats.set(stats);
            items.push_back(msg);
            return PostOutcome::Evicted(evicted);
        }
        items.push_back(msg);
        PostOutcome::Enqueued
    }

    pub(super) fn post_event(&self, msg: M) {
        let outcome = self.post_with(msg, self.event_policy.get());
        let observer = self.event_observer.borrow().clone();
        if let Some(observer) = observer {
            observer(outcome);
        }
    }

    pub(crate) fn set_event_policy(&self, policy: QueuePolicy<M>) {
        self.event_policy.set(policy);
    }

    pub(crate) fn set_event_observer(&self, observer: Option<Rc<dyn Fn(PostOutcome<M>)>>) {
        *self.event_observer.borrow_mut() = observer;
    }

    pub(crate) fn stats(&self) -> QueueStats {
        self.stats.get()
    }

    pub(crate) fn pop(&self) -> Option<M> {
        self.items.borrow_mut().pop_front()
    }

    pub(crate) fn dropped(&self) -> u64 {
        self.dropped.get()
    }
}
