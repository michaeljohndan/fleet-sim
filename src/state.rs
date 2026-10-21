//! The data model: what a vessel is, what cargo looks like, what one world snapshot holds.
//!
//! Everything here is plain data. Rules live in `rules.rs`; the order of
//! operations lives in `sim.rs`.

pub type Loc = usize;

/// A vessel's status. Each variant carries only the fields that make sense for it,
/// so impossible states (an idle vessel with an arrival tick) cannot be represented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Idle {
        location: Loc,
    },
    Transit {
        from: Loc,
        to: Loc,
        arrives_at_tick: u64,
    },
    Docked {
        location: Loc,
    },
}

/// Units of cargo keyed by destination, in insertion order (decision D14).
///
/// Invariants: no zero quantities, no duplicate destinations. Adding to an existing
/// destination keeps its position; an entry that reaches zero is removed; a
/// re-added destination goes to the end. Lists are tiny (at most one entry per
/// location), so a linear scan beats hashing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CargoList {
    entries: Vec<(Loc, u64)>,
}

impl CargoList {
    pub fn quantity(&self, destination: Loc) -> u64 {
        self.entries
            .iter()
            .find(|(d, _)| *d == destination)
            .map_or(0, |(_, q)| *q)
    }

    pub fn add(&mut self, destination: Loc, units: u64) {
        if units == 0 {
            return;
        }
        match self.entries.iter_mut().find(|(d, _)| *d == destination) {
            Some((_, q)) => *q += units,
            None => self.entries.push((destination, units)),
        }
    }

    /// Removes units that validation has already confirmed are present.
    pub fn remove(&mut self, destination: Loc, units: u64) {
        let pos = self
            .entries
            .iter()
            .position(|(d, _)| *d == destination)
            .expect("validated: cargo for this destination is present");
        let q = &mut self.entries[pos].1;
        assert!(*q >= units, "validated: enough cargo is present");
        *q -= units;
        if *q == 0 {
            self.entries.remove(pos);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Loc, u64)> + '_ {
        self.entries.iter().copied()
    }

    pub fn total(&self) -> u64 {
        self.entries.iter().map(|(_, q)| q).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Every dock's inventory as one list of (location, destination, quantity),
/// in insertion order. This is exactly the shape the spec prints.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DockInventory {
    entries: Vec<(Loc, Loc, u64)>,
}

impl DockInventory {
    pub fn quantity(&self, location: Loc, destination: Loc) -> u64 {
        self.entries
            .iter()
            .find(|(l, d, _)| *l == location && *d == destination)
            .map_or(0, |(_, _, q)| *q)
    }

    pub fn add(&mut self, location: Loc, destination: Loc, units: u64) {
        if units == 0 {
            return;
        }
        match self
            .entries
            .iter_mut()
            .find(|(l, d, _)| *l == location && *d == destination)
        {
            Some((_, _, q)) => *q += units,
            None => self.entries.push((location, destination, units)),
        }
    }

    /// Removes units that validation has already confirmed are present.
    pub fn remove(&mut self, location: Loc, destination: Loc, units: u64) {
        let pos = self
            .entries
            .iter()
            .position(|(l, d, _)| *l == location && *d == destination)
            .expect("validated: dock cargo for this destination is present");
        let q = &mut self.entries[pos].2;
        assert!(*q >= units, "validated: enough dock cargo is present");
        *q -= units;
        if *q == 0 {
            self.entries.remove(pos);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Loc, Loc, u64)> + '_ {
        self.entries.iter().copied()
    }

    pub fn total(&self) -> u64 {
        self.entries.iter().map(|(_, _, q)| q).sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vessel {
    pub status: Status,
    pub cargo: CargoList,
}

/// One complete snapshot of the world. Used for committed state, planned state,
/// and every history entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldState {
    pub tick: u64,
    /// Indexed by interned vessel id; order = init order.
    pub vessels: Vec<Vessel>,
    pub docks: DockInventory,
    /// Units ever added by manifests. Not printed; used by the conservation invariant.
    pub manifested: u64,
    /// Units ever delivered (unloaded at their destination). Not printed.
    pub delivered: u64,
}

impl WorldState {
    /// Units currently in the world: on vessels plus on docks.
    pub fn units_in_world(&self) -> u64 {
        self.docks.total() + self.vessels.iter().map(|v| v.cargo.total()).sum::<u64>()
    }
}

/// Fixed facts about the map, set once at init.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Topology {
    locations: usize,
    /// Row-major N x N travel times: `travel[from * N + to]`. One allocation.
    travel: Vec<u32>,
    /// `None` = base mode (no cargo features), decision D15.
    throughput: Option<Vec<u32>>,
}

impl Topology {
    pub fn new(locations: usize, travel: Vec<u32>, throughput: Option<Vec<u32>>) -> Self {
        debug_assert_eq!(travel.len(), locations * locations);
        Topology {
            locations,
            travel,
            throughput,
        }
    }

    pub fn locations(&self) -> usize {
        self.locations
    }

    /// The single seam for travel time. Weather, routing, or time-varying
    /// travel would change only this function.
    pub fn travel_time(&self, from: Loc, to: Loc) -> u64 {
        u64::from(self.travel[from * self.locations + to])
    }

    pub fn cargo_mode(&self) -> bool {
        self.throughput.is_some()
    }

    pub fn throughput(&self, location: Loc) -> Option<u64> {
        self.throughput.as_ref().map(|t| u64::from(t[location]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_list_keeps_insertion_order_and_drops_zeros() {
        let mut c = CargoList::default();
        c.add(2, 1);
        c.add(1, 1);
        c.add(2, 1);
        assert_eq!(c.iter().collect::<Vec<_>>(), vec![(2, 2), (1, 1)]);
        c.remove(2, 2);
        assert_eq!(c.iter().collect::<Vec<_>>(), vec![(1, 1)]);
        c.add(2, 1);
        assert_eq!(c.iter().collect::<Vec<_>>(), vec![(1, 1), (2, 1)]);
    }

    #[test]
    fn dock_inventory_is_keyed_by_location_and_destination() {
        let mut d = DockInventory::default();
        d.add(0, 2, 1);
        d.add(1, 2, 3);
        d.add(0, 2, 1);
        assert_eq!(d.quantity(0, 2), 2);
        assert_eq!(d.quantity(1, 2), 3);
        d.remove(0, 2, 2);
        assert_eq!(d.iter().collect::<Vec<_>>(), vec![(1, 2, 3)]);
    }
}
