//! Every error the simulation can return, with every message in one place.
//!
//! Messages the spec defines are reproduced exactly; the rest follow the same
//! lowercase "subject + problem" style (decision D16).

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimError {
    /// Malformed JSON, an unknown command, or missing / mistyped parameters (D11).
    InvalidCommand(String),
    /// Any command other than `init` before a successful `init`.
    NotInitialized,
    /// A second successful `init`.
    AlreadyInitialized,
    /// `init` data that breaks the spec's constraints (D12).
    InvalidInit(String),
    /// A `cargo_manifest` entry out of range or with a non-positive quantity.
    InvalidManifest(String),
    /// A cargo command or manifest when `init` had no `dock_throughput` (D15).
    CargoNotEnabled,
    UnknownVessel(String),
    DestinationOutOfRange {
        destination: i64,
        locations: usize,
    },
    /// Spec wording: the vessel's committed state is not idle.
    NotIdle(String),
    /// Spec wording: the vessel's committed state is not docked.
    NotDocked(String),
    NoDockCargo {
        location: usize,
        destination: usize,
    },
    NoVesselCargo {
        vessel: String,
        destination: usize,
    },
    /// Spec wording: the new pending state needs more operations than the dock allows.
    ThroughputExceeded {
        operations: u64,
        limit: u64,
        location: usize,
    },
    /// Spec wording: `get_state_at` asked for a tick that has not happened yet.
    FutureTick {
        tick: i64,
        current: u64,
    },
    NegativeTick(i64),
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimError::InvalidCommand(reason) => write!(f, "invalid command: {reason}"),
            SimError::NotInitialized => write!(f, "simulation not initialized"),
            SimError::AlreadyInitialized => write!(f, "simulation already initialized"),
            SimError::InvalidInit(reason) => write!(f, "invalid init: {reason}"),
            SimError::InvalidManifest(reason) => write!(f, "invalid cargo_manifest: {reason}"),
            SimError::CargoNotEnabled => {
                write!(f, "cargo operations require dock_throughput at init")
            }
            SimError::UnknownVessel(id) => write!(f, "unknown vessel {id}"),
            SimError::DestinationOutOfRange {
                destination,
                locations,
            } => write!(
                f,
                "destination {destination} is out of range (0-{})",
                locations.saturating_sub(1)
            ),
            SimError::NotIdle(id) => write!(f, "vessel {id} is not idle"),
            SimError::NotDocked(id) => write!(f, "vessel {id} is not docked"),
            SimError::NoDockCargo {
                location,
                destination,
            } => write!(f, "dock {location} has no cargo for destination {destination}"),
            SimError::NoVesselCargo {
                vessel,
                destination,
            } => write!(f, "vessel {vessel} has no cargo for destination {destination}"),
            SimError::ThroughputExceeded {
                operations,
                limit,
                location,
            } => write!(
                f,
                "transfer of {operations} operations exceeds dock throughput of {limit} at location {location}"
            ),
            SimError::FutureTick { tick, current } => {
                write!(f, "tick {tick} is in the future (current tick: {current})")
            }
            SimError::NegativeTick(tick) => write!(f, "tick {tick} is negative"),
        }
    }
}

impl std::error::Error for SimError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// No wildcard arm: a new variant fails to compile until it gets a pinned message below.
    fn has_pinned_message(e: &SimError) -> bool {
        match e {
            SimError::InvalidCommand(_)
            | SimError::NotInitialized
            | SimError::AlreadyInitialized
            | SimError::InvalidInit(_)
            | SimError::InvalidManifest(_)
            | SimError::CargoNotEnabled
            | SimError::UnknownVessel(_)
            | SimError::DestinationOutOfRange { .. }
            | SimError::NotIdle(_)
            | SimError::NotDocked(_)
            | SimError::NoDockCargo { .. }
            | SimError::NoVesselCargo { .. }
            | SimError::ThroughputExceeded { .. }
            | SimError::FutureTick { .. }
            | SimError::NegativeTick(_) => true,
        }
    }

    /// D16: every message lives in `Display`, spec wording is exact, and the rest
    /// follow the same lowercase "subject + problem" style with no trailing period.
    #[test]
    fn d16_every_error_message_is_pinned() {
        let cases = [
            (
                SimError::InvalidCommand("expected value".into()),
                "invalid command: expected value",
            ),
            (SimError::NotInitialized, "simulation not initialized"),
            (
                SimError::AlreadyInitialized,
                "simulation already initialized",
            ),
            (SimError::InvalidInit("bad".into()), "invalid init: bad"),
            (
                SimError::InvalidManifest("bad".into()),
                "invalid cargo_manifest: bad",
            ),
            (
                SimError::CargoNotEnabled,
                "cargo operations require dock_throughput at init",
            ),
            (SimError::UnknownVessel("v9".into()), "unknown vessel v9"),
            (
                SimError::DestinationOutOfRange {
                    destination: 5,
                    locations: 3,
                },
                "destination 5 is out of range (0-2)",
            ),
            (SimError::NotIdle("v1".into()), "vessel v1 is not idle"),
            (SimError::NotDocked("v1".into()), "vessel v1 is not docked"),
            (
                SimError::NoDockCargo {
                    location: 0,
                    destination: 1,
                },
                "dock 0 has no cargo for destination 1",
            ),
            (
                SimError::NoVesselCargo {
                    vessel: "v1".into(),
                    destination: 2,
                },
                "vessel v1 has no cargo for destination 2",
            ),
            (
                SimError::ThroughputExceeded {
                    operations: 3,
                    limit: 2,
                    location: 0,
                },
                "transfer of 3 operations exceeds dock throughput of 2 at location 0",
            ),
            (
                SimError::FutureTick {
                    tick: 99,
                    current: 13,
                },
                "tick 99 is in the future (current tick: 13)",
            ),
            (SimError::NegativeTick(-1), "tick -1 is negative"),
        ];
        for (err, text) in cases {
            assert!(has_pinned_message(&err));
            assert_eq!(err.to_string(), text);
            assert!(text.starts_with(|c: char| c.is_lowercase()), "{text}");
            assert!(!text.ends_with('.'), "{text}");
        }
    }
}
