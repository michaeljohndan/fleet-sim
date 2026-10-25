//! Fleet simulation engine.
//!
//! Layers (each has one job; see CLAUDE.md for the full map):
//! - `lib`:      `handle_line` connects them: parse -> execute -> render
//! - `protocol`: JSON lines <-> typed commands and output views; never calls the engine
//! - `sim`:      validate-then-apply engine, committed / planned / pending state
//! - `rules`:    pure rule helpers (throughput counting, range checks)
//! - `state`:    plain data model
//! - `history`:  committed snapshots for `get_state_at`
//! - `error`:    every error and its message
//!
//! Dependencies point one way: protocol and rules depend only on state and error.

pub mod error;
pub mod history;
pub mod protocol;
pub mod rules;
pub mod sim;
pub mod state;

pub use error::SimError;
pub use protocol::{render_error, Command};
pub use sim::{Simulation, View, World};

/// Handles one raw input line. Returns `None` for a blank line (not a command),
/// otherwise exactly one response line without the trailing newline.
pub fn handle_line(sim: &mut Simulation, line: &str) -> Option<String> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let result = protocol::parse_command(line).and_then(|cmd| sim.execute(cmd));
    Some(match result {
        Ok(view) => {
            let world = sim
                .world()
                .expect("a successful command implies a running world");
            let state = world.state_for(view);
            protocol::render_state(world.vessel_names(), &state, world.cargo_mode())
        }
        Err(e) => render_error(&e),
    })
}
