//! The wire format: JSON lines in, JSON lines out.
//!
//! Input parses straight into a typed `Command` (serde, adjacently tagged).
//! Output is written from lightweight views that borrow the state, in a fixed key
//! order, so the internal model and the JSON shape can change independently.
//!
//! Parse and format only: this module never calls the engine. `handle_line` in
//! `lib.rs` connects the two.

use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Serialize, Serializer};

use crate::error::SimError;
use crate::state::{Status, Vessel, WorldState};

/// One input line. Unknown commands, unknown or missing parameters, and wrong
/// types are rejected by the parser before the engine sees them (D11).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "command",
    content = "parameters",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Command {
    Init(InitParams),
    Tick(TickParams),
    SetDestination {
        vessel_id: String,
        destination: i64,
    },
    Dock {
        vessel_id: String,
    },
    Undock {
        vessel_id: String,
    },
    Load {
        vessel_id: String,
        destination: i64,
    },
    Unload {
        vessel_id: String,
        destination: i64,
    },
    Swap {
        vessel_id: String,
        load_destination: i64,
        unload_destination: i64,
    },
    GetState {},
    GetStateAt {
        tick: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitParams {
    pub travel_times: Vec<Vec<i64>>,
    pub vessels: Vec<VesselInit>,
    #[serde(default)]
    pub dock_throughput: Option<Vec<i64>>,
    #[serde(default)]
    pub cargo_manifest: Option<Vec<ManifestEntry>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VesselInit {
    pub id: String,
    pub location: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TickParams {
    #[serde(default)]
    pub cargo_manifest: Option<Vec<ManifestEntry>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestEntry {
    pub location: i64,
    pub destination: i64,
    pub quantity: i64,
}

/// Parses one trimmed, non-blank input line into a `Command` (D11).
pub fn parse_command(line: &str) -> Result<Command, SimError> {
    serde_json::from_str::<Command>(line).map_err(|e| SimError::InvalidCommand(e.to_string()))
}

/// An `ok` response line for `state`. `names` are the vessel ids in init order
/// (D14); `cargo_mode` decides whether cargo fields are printed (D15).
pub fn render_state(names: &[String], state: &WorldState, cargo_mode: bool) -> String {
    to_line(&Response {
        status: "ok",
        payload: StateView {
            names,
            state,
            cargo_mode,
        },
    })
}

pub fn render_error(err: &SimError) -> String {
    to_line(&Response {
        status: "error",
        payload: ErrorPayload {
            message: err.to_string(),
        },
    })
}

fn to_line<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("in-memory views always serialize")
}

#[derive(Serialize)]
struct Response<P> {
    status: &'static str,
    payload: P,
}

#[derive(Serialize)]
struct ErrorPayload {
    message: String,
}

/// `{"tick":..,"vessels":{..},"cargo":[..]}`; `cargo` only in cargo mode (D15).
struct StateView<'a> {
    names: &'a [String],
    state: &'a WorldState,
    cargo_mode: bool,
}

impl Serialize for StateView<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(None)?;
        map.serialize_entry("tick", &self.state.tick)?;
        map.serialize_entry(
            "vessels",
            &VesselsView {
                names: self.names,
                vessels: &self.state.vessels,
                cargo_mode: self.cargo_mode,
            },
        )?;
        if self.cargo_mode {
            map.serialize_entry("cargo", &DocksView(self.state))?;
        }
        map.end()
    }
}

/// Vessels as a JSON object in init order. serde_json would sort a map's keys,
/// so the order is written explicitly (D14).
struct VesselsView<'a> {
    names: &'a [String],
    vessels: &'a [Vessel],
    cargo_mode: bool,
}

impl Serialize for VesselsView<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(Some(self.vessels.len()))?;
        for (name, vessel) in self.names.iter().zip(self.vessels) {
            map.serialize_entry(
                name,
                &VesselView {
                    vessel,
                    cargo_mode: self.cargo_mode,
                },
            )?;
        }
        map.end()
    }
}

struct VesselView<'a> {
    vessel: &'a Vessel,
    cargo_mode: bool,
}

impl Serialize for VesselView<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(None)?;
        match self.vessel.status {
            Status::Idle { location } => {
                map.serialize_entry("state", "idle")?;
                map.serialize_entry("location", &location)?;
            }
            Status::Transit {
                from,
                to,
                arrives_at_tick,
            } => {
                map.serialize_entry("state", "transit")?;
                map.serialize_entry("from", &from)?;
                map.serialize_entry("to", &to)?;
                map.serialize_entry("arrives_at_tick", &arrives_at_tick)?;
            }
            Status::Docked { location } => {
                map.serialize_entry("state", "docked")?;
                map.serialize_entry("location", &location)?;
            }
        }
        if self.cargo_mode {
            map.serialize_entry("cargo", &CargoView(self.vessel))?;
        }
        map.end()
    }
}

struct CargoView<'a>(&'a Vessel);

impl Serialize for CargoView<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Entry {
            destination: usize,
            quantity: u64,
        }
        let mut seq = s.serialize_seq(None)?;
        for (destination, quantity) in self.0.cargo.iter() {
            seq.serialize_element(&Entry {
                destination,
                quantity,
            })?;
        }
        seq.end()
    }
}

struct DocksView<'a>(&'a WorldState);

impl Serialize for DocksView<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Entry {
            location: usize,
            destination: usize,
            quantity: u64,
        }
        let mut seq = s.serialize_seq(None)?;
        for (location, destination, quantity) in self.0.docks.iter() {
            seq.serialize_element(&Entry {
                location,
                destination,
                quantity,
            })?;
        }
        seq.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{CargoList, DockInventory};

    /// A hand-built state renders without a `World`: protocol needs only plain data.
    /// Raw lines are compared because `serde_json::Value` would hide key order.
    #[test]
    fn render_state_needs_no_world() {
        let names = vec!["v2".to_string(), "v10".to_string()];
        let mut cargo = CargoList::default();
        cargo.add(2, 1);
        let mut docks = DockInventory::default();
        docks.add(1, 0, 3);
        let state = WorldState {
            tick: 3,
            vessels: vec![
                Vessel {
                    status: Status::Docked { location: 1 },
                    cargo,
                },
                Vessel {
                    status: Status::Transit {
                        from: 0,
                        to: 2,
                        arrives_at_tick: 5,
                    },
                    cargo: CargoList::default(),
                },
            ],
            docks,
            manifested: 4,
            delivered: 0,
        };

        assert_eq!(
            render_state(&names, &state, false),
            r#"{"status":"ok","payload":{"tick":3,"vessels":{"v2":{"state":"docked","location":1},"v10":{"state":"transit","from":0,"to":2,"arrives_at_tick":5}}}}"#
        );
        assert_eq!(
            render_state(&names, &state, true),
            r#"{"status":"ok","payload":{"tick":3,"vessels":{"v2":{"state":"docked","location":1,"cargo":[{"destination":2,"quantity":1}]},"v10":{"state":"transit","from":0,"to":2,"arrives_at_tick":5,"cargo":[]}},"cargo":[{"location":1,"destination":0,"quantity":3}]}}"#
        );
    }
}
