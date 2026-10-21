//! Fleet simulation engine.
//!
//! Layers (each has one job; see CLAUDE.md for the full map):
//! - `protocol`: JSON lines <-> typed commands and output views
//! - `sim`:      validate-then-apply engine, committed / planned / pending state
//! - `rules`:    pure rule helpers (throughput counting, range checks)
//! - `state`:    plain data model
//! - `history`:  committed snapshots for `get_state_at`
//! - `error`:    every error and its message

pub mod error;
pub mod history;
pub mod protocol;
pub mod rules;
pub mod sim;
pub mod state;

pub use error::SimError;
pub use protocol::{handle_line, render_error, Command};
pub use sim::{Simulation, View, World};
