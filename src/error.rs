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
