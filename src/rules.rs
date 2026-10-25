//! Pure rule helpers: no I/O, no mutation. Every function here can be unit-tested
//! with plain values.
//!
//! The golden rule: a command is valid if and only if the COMMITTED state allows it.
//! A valid command replaces the vessel's pending command (last one wins). Cargo
//! commands also check PLANNED inventory and dock throughput.

use crate::error::SimError;
use crate::state::{CargoList, Loc, Status, WorldState};

/// Operations needed to turn `committed` cargo into `planned` cargo (decision D8).
///
/// L = units gained, U = units lost, counted per destination against the committed
/// state. One swap covers one load plus one unload, so the cost is max(L, U).
/// Examples: two loads = 2, a swap = 1, a delivery = 1, load-then-unload of the
/// same unit = 0.
pub fn vessel_operations(committed: &CargoList, planned: &CargoList) -> u64 {
    let mut gained = 0;
    let mut lost = 0;
    for (dest, now) in planned.iter() {
        let before = committed.quantity(dest);
        if now > before {
            gained += now - before;
        } else {
            lost += before - now;
        }
    }
    for (dest, before) in committed.iter() {
        if planned.quantity(dest) == 0 {
            lost += before;
        }
    }
    gained.max(lost)
}

/// Total operations at one dock across every vessel whose COMMITTED status is
/// docked there. `candidate` substitutes one vessel's hypothetical planned cargo,
/// so a command can be checked before anything is changed.
pub fn dock_operations(
    committed: &WorldState,
    planned: &WorldState,
    location: Loc,
    candidate: Option<(usize, &CargoList)>,
) -> u64 {
    committed
        .vessels
        .iter()
        .enumerate()
        .filter(|(_, v)| v.status == Status::Docked { location })
        .map(|(i, v)| {
            let planned_cargo = match candidate {
                Some((idx, cargo)) if idx == i => cargo,
                _ => &planned.vessels[i].cargo,
            };
            vessel_operations(&v.cargo, planned_cargo)
        })
        .sum()
}

/// A location index from the wire, checked against the map size.
pub fn location(value: i64, locations: usize) -> Option<Loc> {
    usize::try_from(value).ok().filter(|v| *v < locations)
}

/// Checked manifest entries as (location, destination, quantity).
pub type Manifest = Vec<(Loc, Loc, u64)>;

/// Validates a whole manifest before any of it is applied (all or nothing).
/// Entries are raw (location, destination, quantity) values from the wire.
pub fn check_manifest(
    entries: impl IntoIterator<Item = (i64, i64, i64)>,
    locations: usize,
) -> Result<Manifest, SimError> {
    entries
        .into_iter()
        .enumerate()
        .map(|(i, (raw_loc, raw_dest, raw_qty))| {
            let loc = location(raw_loc, locations).ok_or_else(|| {
                SimError::InvalidManifest(format!(
                    "entry {i} location {raw_loc} is out of range (0-{})",
                    locations - 1
                ))
            })?;
            let dest = location(raw_dest, locations).ok_or_else(|| {
                SimError::InvalidManifest(format!(
                    "entry {i} destination {raw_dest} is out of range (0-{})",
                    locations - 1
                ))
            })?;
            let qty = u64::try_from(raw_qty)
                .ok()
                .filter(|q| *q > 0)
                .ok_or_else(|| {
                    SimError::InvalidManifest(format!(
                        "entry {i} quantity {raw_qty} must be a positive integer"
                    ))
                })?;
            Ok((loc, dest, qty))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cargo(items: &[(Loc, u64)]) -> CargoList {
        let mut c = CargoList::default();
        for (d, q) in items {
            c.add(*d, *q);
        }
        c
    }

    #[test]
    fn two_loads_cost_two() {
        assert_eq!(vessel_operations(&cargo(&[]), &cargo(&[(2, 1), (1, 1)])), 2);
    }

    #[test]
    fn swap_costs_one() {
        assert_eq!(vessel_operations(&cargo(&[(2, 1)]), &cargo(&[(1, 1)])), 1);
    }

    #[test]
    fn delivery_costs_one() {
        assert_eq!(
            vessel_operations(&cargo(&[(2, 1), (1, 1)]), &cargo(&[(1, 1)])),
            1
        );
    }

    #[test]
    fn load_then_unload_same_unit_costs_zero() {
        assert_eq!(vessel_operations(&cargo(&[(2, 1)]), &cargo(&[(2, 1)])), 0);
    }

    #[test]
    fn three_loads_and_one_unload_cost_three() {
        let before = cargo(&[(0, 1)]);
        let after = cargo(&[(1, 3)]);
        assert_eq!(vessel_operations(&before, &after), 3);
    }

    #[test]
    fn location_rejects_negative_and_out_of_range() {
        assert_eq!(location(-1, 3), None);
        assert_eq!(location(3, 3), None);
        assert_eq!(location(2, 3), Some(2));
    }
}
