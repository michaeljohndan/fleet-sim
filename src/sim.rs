//! The engine: validates each command, then applies it.
//!
//! Two phases per command:
//!   1. `validate` reads committed + planned state and returns an `Effect` or an
//!      error. It changes nothing.
//!   2. `apply` performs the effect. It cannot fail, so a rejected command never
//!      leaves the world half-changed (D13).
//!
//! State model (double buffer):
//!   committed  official state as of the last tick
//!   planned    committed + every command accepted since then (what get_state shows)
//!   pending    the accepted commands themselves, in order (enables cancel / replay)

use std::borrow::Cow;
use std::collections::HashMap;

use crate::error::SimError;
use crate::history::{HistoryStore, SnapshotHistory};
use crate::protocol::{Command, InitParams};
use crate::rules::{self, Manifest};
use crate::state::{CargoList, DockInventory, Loc, Status, Topology, Vessel, WorldState};

/// Which state a successful command reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Planned state: every command except `get_state_at`.
    Planned,
    /// Committed state recorded at a past tick.
    AtTick(u64),
}

/// The top-level object. A failed `init` leaves it uninitialized, so `init` can be
/// retried (D12).
#[derive(Debug, Default)]
pub struct Simulation {
    world: Option<Box<World>>,
}

impl Simulation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn world(&self) -> Option<&World> {
        self.world.as_deref()
    }

    pub fn execute(&mut self, cmd: Command) -> Result<View, SimError> {
        if let Some(world) = self.world.as_mut() {
            return world.execute(cmd);
        }
        match cmd {
            Command::Init(params) => {
                self.world = Some(Box::new(World::new(params)?));
                Ok(View::Planned)
            }
            _ => Err(SimError::NotInitialized),
        }
    }
}

/// What a validated command will do. Produced by `validate`, consumed by `apply`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Effect {
    SetStatus {
        vessel: usize,
        status: Status,
    },
    Load {
        vessel: usize,
        location: Loc,
        destination: Loc,
    },
    Unload {
        vessel: usize,
        location: Loc,
        destination: Loc,
    },
    Swap {
        vessel: usize,
        location: Loc,
        load: Loc,
        unload: Loc,
    },
    Tick {
        manifest: Manifest,
    },
}

#[derive(Debug)]
pub struct World {
    topology: Topology,
    /// Interned vessel ids: index -> name, in init order.
    names: Vec<String>,
    index: HashMap<String, usize>,
    committed: WorldState,
    planned: WorldState,
    pending: Vec<Command>,
    history: SnapshotHistory,
}

impl World {
    /// Validates every init constraint before building anything.
    pub fn new(p: InitParams) -> Result<World, SimError> {
        let n = p.travel_times.len();
        if n == 0 {
            return Err(SimError::InvalidInit(
                "travel_times must have at least one location".into(),
            ));
        }
        let mut travel = Vec::with_capacity(n * n);
        for (i, row) in p.travel_times.iter().enumerate() {
            if row.len() != n {
                return Err(SimError::InvalidInit(format!(
                    "travel_times must be square: row {i} has {} entries, expected {n}",
                    row.len()
                )));
            }
            for (j, &t) in row.iter().enumerate() {
                let ok = if i == j {
                    t == 0
                } else {
                    t > 0 && t <= i64::from(u32::MAX)
                };
                if !ok {
                    let rule = if i == j {
                        "must be 0"
                    } else {
                        "must be a positive integer"
                    };
                    return Err(SimError::InvalidInit(format!(
                        "travel_times[{i}][{j}] = {t} {rule}"
                    )));
                }
                travel.push(t as u32);
            }
        }

        let throughput = match &p.dock_throughput {
            None => None,
            Some(t) if t.len() != n => {
                return Err(SimError::InvalidInit(format!(
                    "dock_throughput has {} entries, expected {n}",
                    t.len()
                )))
            }
            Some(t) => {
                let mut out = Vec::with_capacity(n);
                for (i, &v) in t.iter().enumerate() {
                    if v <= 0 || v > i64::from(u32::MAX) {
                        return Err(SimError::InvalidInit(format!(
                            "dock_throughput[{i}] = {v} must be a positive integer"
                        )));
                    }
                    out.push(v as u32);
                }
                Some(out)
            }
        };

        let mut names = Vec::with_capacity(p.vessels.len());
        let mut index = HashMap::with_capacity(p.vessels.len());
        let mut vessels = Vec::with_capacity(p.vessels.len());
        for v in &p.vessels {
            let location = rules::location(v.location, n).ok_or_else(|| {
                SimError::InvalidInit(format!(
                    "vessel {} location {} is out of range (0-{})",
                    v.id,
                    v.location,
                    n - 1
                ))
            })?;
            if index.insert(v.id.clone(), names.len()).is_some() {
                return Err(SimError::InvalidInit(format!(
                    "duplicate vessel id {}",
                    v.id
                )));
            }
            names.push(v.id.clone());
            vessels.push(Vessel {
                status: Status::Idle { location },
                cargo: CargoList::default(),
            });
        }

        // D15: base mode rejects any cargo_manifest field, even an empty one.
        let manifest = match &p.cargo_manifest {
            Some(_) if throughput.is_none() => return Err(SimError::CargoNotEnabled),
            Some(entries) => rules::check_manifest(entries, n)?,
            None => Vec::new(),
        };

        let mut state = WorldState {
            tick: 0,
            vessels,
            docks: DockInventory::default(),
            manifested: 0,
            delivered: 0,
        };
        add_manifest(&mut state, &manifest);

        let mut history = SnapshotHistory::default();
        history.record(&state);
        Ok(World {
            topology: Topology::new(n, travel, throughput),
            names,
            index,
            committed: state.clone(),
            planned: state,
            pending: Vec::new(),
            history,
        })
    }

    pub fn execute(&mut self, cmd: Command) -> Result<View, SimError> {
        match &cmd {
            Command::Init(_) => return Err(SimError::AlreadyInitialized),
            Command::GetState {} => return Ok(View::Planned),
            Command::GetStateAt { tick } => return self.check_history_tick(*tick),
            _ => {}
        }
        let effect = self.validate(&cmd)?;
        let is_tick = matches!(effect, Effect::Tick { .. });
        self.apply(effect);
        if !is_tick {
            self.pending.push(cmd);
        }
        Ok(View::Planned)
    }

    // ---------------------------------------------------------------- validate

    fn validate(&self, cmd: &Command) -> Result<Effect, SimError> {
        match cmd {
            Command::Tick(p) => {
                // D15: base mode rejects any cargo_manifest field, even an empty one.
                let manifest = match &p.cargo_manifest {
                    Some(_) if !self.cargo_mode() => return Err(SimError::CargoNotEnabled),
                    Some(entries) => rules::check_manifest(entries, self.locations())?,
                    None => Vec::new(),
                };
                Ok(Effect::Tick { manifest })
            }
            Command::SetDestination {
                vessel_id,
                destination,
            } => {
                let vessel = self.vessel(vessel_id)?;
                let dest = self.destination(*destination)?;
                let Status::Idle { location } = self.committed.vessels[vessel].status else {
                    return Err(SimError::NotIdle(vessel_id.clone()));
                };
                // D1: a destination equal to the current location means "stay put"
                // (planned = idle here), never a zero-length trip.
                let status = if dest == location {
                    Status::Idle { location }
                } else {
                    Status::Transit {
                        from: location,
                        to: dest,
                        arrives_at_tick: self.committed.tick
                            + self.topology.travel_time(location, dest),
                    }
                };
                Ok(Effect::SetStatus { vessel, status })
            }
            Command::Dock { vessel_id } => {
                self.require_cargo_mode()?;
                let vessel = self.vessel(vessel_id)?;
                let Status::Idle { location } = self.committed.vessels[vessel].status else {
                    return Err(SimError::NotIdle(vessel_id.clone()));
                };
                Ok(Effect::SetStatus {
                    vessel,
                    status: Status::Docked { location },
                })
            }
            Command::Undock { vessel_id } => {
                self.require_cargo_mode()?;
                let vessel = self.vessel(vessel_id)?;
                let location = self.docked_location(vessel, vessel_id)?;
                Ok(Effect::SetStatus {
                    vessel,
                    status: Status::Idle { location },
                })
            }
            Command::Load {
                vessel_id,
                destination,
            } => {
                self.require_cargo_mode()?;
                let vessel = self.vessel(vessel_id)?;
                let dest = self.destination(*destination)?;
                let location = self.docked_location(vessel, vessel_id)?;
                self.require_dock_cargo(location, dest)?;
                let mut after = self.planned.vessels[vessel].cargo.clone();
                after.add(dest, 1);
                self.check_throughput(location, vessel, &after)?;
                Ok(Effect::Load {
                    vessel,
                    location,
                    destination: dest,
                })
            }
            Command::Unload {
                vessel_id,
                destination,
            } => {
                self.require_cargo_mode()?;
                let vessel = self.vessel(vessel_id)?;
                let dest = self.destination(*destination)?;
                let location = self.docked_location(vessel, vessel_id)?;
                self.require_vessel_cargo(vessel, vessel_id, dest)?;
                let mut after = self.planned.vessels[vessel].cargo.clone();
                after.remove(dest, 1);
                self.check_throughput(location, vessel, &after)?;
                Ok(Effect::Unload {
                    vessel,
                    location,
                    destination: dest,
                })
            }
            Command::Swap {
                vessel_id,
                load_destination,
                unload_destination,
            } => {
                self.require_cargo_mode()?;
                let vessel = self.vessel(vessel_id)?;
                let load = self.destination(*load_destination)?;
                let unload = self.destination(*unload_destination)?;
                let location = self.docked_location(vessel, vessel_id)?;
                self.require_dock_cargo(location, load)?;
                self.require_vessel_cargo(vessel, vessel_id, unload)?;
                let mut after = self.planned.vessels[vessel].cargo.clone();
                after.remove(unload, 1);
                after.add(load, 1);
                // D18: a swap with load == unload leaves the cargo unchanged, so the
                // net-change rule (D8) costs it 0 operations, exactly like load then
                // unload of the same destination. Cost depends only on the result.
                self.check_throughput(location, vessel, &after)?;
                Ok(Effect::Swap {
                    vessel,
                    location,
                    load,
                    unload,
                })
            }
            Command::Init(_) | Command::GetState {} | Command::GetStateAt { .. } => {
                unreachable!("handled in execute before validation")
            }
        }
    }

    fn vessel(&self, id: &str) -> Result<usize, SimError> {
        self.index
            .get(id)
            .copied()
            .ok_or_else(|| SimError::UnknownVessel(id.to_string()))
    }

    fn destination(&self, value: i64) -> Result<Loc, SimError> {
        rules::location(value, self.locations()).ok_or(SimError::DestinationOutOfRange {
            destination: value,
            locations: self.locations(),
        })
    }

    /// Golden rule for undock and every cargo command: COMMITTED state must be docked.
    fn docked_location(&self, vessel: usize, id: &str) -> Result<Loc, SimError> {
        match self.committed.vessels[vessel].status {
            Status::Docked { location } => Ok(location),
            _ => Err(SimError::NotDocked(id.to_string())),
        }
    }

    fn require_cargo_mode(&self) -> Result<(), SimError> {
        if self.cargo_mode() {
            Ok(())
        } else {
            Err(SimError::CargoNotEnabled)
        }
    }

    /// Inventory is checked against PLANNED state, so one unit can never be loaded
    /// twice (D9).
    fn require_dock_cargo(&self, location: Loc, destination: Loc) -> Result<(), SimError> {
        if self.planned.docks.quantity(location, destination) > 0 {
            Ok(())
        } else {
            Err(SimError::NoDockCargo {
                location,
                destination,
            })
        }
    }

    fn require_vessel_cargo(
        &self,
        vessel: usize,
        id: &str,
        destination: Loc,
    ) -> Result<(), SimError> {
        if self.planned.vessels[vessel].cargo.quantity(destination) > 0 {
            Ok(())
        } else {
            Err(SimError::NoVesselCargo {
                vessel: id.to_string(),
                destination,
            })
        }
    }

    /// D8: rejects the command if the resulting pending state needs more operations
    /// at this dock than its throughput allows.
    fn check_throughput(
        &self,
        location: Loc,
        vessel: usize,
        after: &CargoList,
    ) -> Result<(), SimError> {
        let limit = self
            .topology
            .throughput(location)
            .expect("cargo mode checked before throughput");
        let operations = rules::dock_operations(
            &self.committed,
            &self.planned,
            location,
            Some((vessel, after)),
        );
        if operations > limit {
            return Err(SimError::ThroughputExceeded {
                operations,
                limit,
                location,
            });
        }
        Ok(())
    }

    fn check_history_tick(&self, tick: i64) -> Result<View, SimError> {
        let t = u64::try_from(tick).map_err(|_| SimError::NegativeTick(tick))?;
        if t > self.committed.tick {
            return Err(SimError::FutureTick {
                tick,
                current: self.committed.tick,
            });
        }
        Ok(View::AtTick(t))
    }

    // ------------------------------------------------------------------- apply

    /// Performs a validated effect. Never fails.
    fn apply(&mut self, effect: Effect) {
        match effect {
            Effect::SetStatus { vessel, status } => {
                self.planned.vessels[vessel].status = status;
            }
            Effect::Load {
                vessel,
                location,
                destination,
            } => {
                self.planned.docks.remove(location, destination, 1);
                self.planned.vessels[vessel].cargo.add(destination, 1);
            }
            Effect::Unload {
                vessel,
                location,
                destination,
            } => {
                self.planned.vessels[vessel].cargo.remove(destination, 1);
                self.unload_into_dock(location, destination);
            }
            Effect::Swap {
                vessel,
                location,
                load,
                unload,
            } => {
                let cargo = &mut self.planned.vessels[vessel].cargo;
                cargo.remove(unload, 1);
                cargo.add(load, 1);
                self.planned.docks.remove(location, load, 1);
                self.unload_into_dock(location, unload);
            }
            Effect::Tick { manifest } => self.advance(&manifest),
        }
    }

    /// Cargo unloaded at its own destination is delivered and leaves the simulation.
    fn unload_into_dock(&mut self, location: Loc, destination: Loc) {
        if destination == location {
            self.planned.delivered += 1;
        } else {
            self.planned.docks.add(location, destination, 1);
        }
    }

    /// One tick: arrivals, then the manifest, then planned becomes committed.
    fn advance(&mut self, manifest: &Manifest) {
        let state = &mut self.planned;
        state.tick += 1;
        for vessel in &mut state.vessels {
            if let Status::Transit {
                to,
                arrives_at_tick,
                ..
            } = vessel.status
            {
                if arrives_at_tick <= state.tick {
                    vessel.status = Status::Idle { location: to };
                }
            }
        }
        add_manifest(state, manifest);
        self.committed = self.planned.clone();
        self.history.record(&self.committed);
        self.pending.clear();
    }

    // ------------------------------------------------------------ read access

    pub fn cargo_mode(&self) -> bool {
        self.topology.cargo_mode()
    }

    pub fn locations(&self) -> usize {
        self.topology.locations()
    }

    pub fn vessel_names(&self) -> &[String] {
        &self.names
    }

    pub fn committed(&self) -> &WorldState {
        &self.committed
    }

    pub fn planned(&self) -> &WorldState {
        &self.planned
    }

    /// Commands accepted since the last tick, in order.
    pub fn pending(&self) -> &[Command] {
        &self.pending
    }

    pub fn state_for(&self, view: View) -> Cow<'_, WorldState> {
        match view {
            View::Planned => Cow::Borrowed(&self.planned),
            View::AtTick(t) => self
                .history
                .get(t)
                .expect("tick was checked against the current tick"),
        }
    }

    /// Checks every invariant the design promises. Used by the test suite after
    /// every command; returns a description of the first violation.
    pub fn check_invariants(&self) -> Result<(), String> {
        let (c, p) = (&self.committed, &self.planned);
        if c.tick != p.tick {
            return Err(format!(
                "planned tick {} != committed tick {}",
                p.tick, c.tick
            ));
        }
        if self.history.len() as u64 != c.tick + 1 {
            return Err(format!(
                "history has {} entries at tick {}",
                self.history.len(),
                c.tick
            ));
        }
        if self.history.last() != Some(c) {
            return Err("latest history entry differs from committed state".into());
        }
        for (label, s) in [("committed", c), ("planned", p)] {
            if s.units_in_world() + s.delivered != s.manifested {
                return Err(format!(
                    "{label}: cargo not conserved ({} in world + {} delivered != {} manifested)",
                    s.units_in_world(),
                    s.delivered,
                    s.manifested
                ));
            }
        }
        if p.manifested != c.manifested {
            return Err("cargo was manifested outside a tick".into());
        }
        for (i, v) in c.vessels.iter().enumerate() {
            let name = &self.names[i];
            if let Status::Transit {
                arrives_at_tick, ..
            } = v.status
            {
                if arrives_at_tick <= c.tick {
                    return Err(format!("{name} is still in transit after its arrival tick"));
                }
            }
            let docked = matches!(v.status, Status::Docked { .. });
            if !docked && p.vessels[i].cargo != v.cargo {
                return Err(format!("{name} changed cargo without being docked"));
            }
            if !self.cargo_mode() && !v.cargo.is_empty() {
                return Err(format!("{name} has cargo in base mode"));
            }
        }
        if self.cargo_mode() {
            for loc in 0..self.locations() {
                let ops = rules::dock_operations(c, p, loc, None);
                let limit = self.topology.throughput(loc).unwrap_or(0);
                if ops > limit {
                    return Err(format!("dock {loc} uses {ops} operations, limit {limit}"));
                }
            }
        }
        Ok(())
    }
}

fn add_manifest(state: &mut WorldState, manifest: &Manifest) {
    for &(location, destination, quantity) in manifest {
        state.docks.add(location, destination, quantity);
        state.manifested += quantity;
    }
}
