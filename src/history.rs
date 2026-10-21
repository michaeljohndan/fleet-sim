//! Committed-state history for `get_state_at` (Follow-on 2).
//!
//! `init` records tick 0; every `tick` records the newly committed state after all
//! transitions. Intra-tick (planned) states are never recorded.
//!
//! The `HistoryStore` trait is the seam: v1 keeps a full snapshot per tick
//! (O(ticks x vessels) memory, O(1) lookup). The planned upgrade is a checkpoint
//! every K ticks plus a per-tick command log, replayed on lookup. Only this module
//! would change.

use std::borrow::Cow;

use crate::state::WorldState;

pub trait HistoryStore {
    /// Records the committed state at the end of its tick. Ticks arrive in order.
    fn record(&mut self, state: &WorldState);
    /// The committed state at `tick`, if that tick has happened.
    fn get(&self, tick: u64) -> Option<Cow<'_, WorldState>>;
}

#[derive(Debug, Default)]
pub struct SnapshotHistory {
    snapshots: Vec<WorldState>,
}

impl HistoryStore for SnapshotHistory {
    fn record(&mut self, state: &WorldState) {
        debug_assert_eq!(
            state.tick as usize,
            self.snapshots.len(),
            "ticks recorded in order"
        );
        self.snapshots.push(state.clone());
    }

    fn get(&self, tick: u64) -> Option<Cow<'_, WorldState>> {
        usize::try_from(tick)
            .ok()
            .and_then(|t| self.snapshots.get(t))
            .map(Cow::Borrowed)
    }
}

impl SnapshotHistory {
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    pub fn last(&self) -> Option<&WorldState> {
        self.snapshots.last()
    }
}
