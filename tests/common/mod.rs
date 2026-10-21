//! Shared test harness: send JSON commands, assert ok/error, check invariants
//! after every command.
#![allow(dead_code)]

use fleet_sim::{handle_line, Simulation};
use serde_json::Value;

pub struct Harness {
    pub sim: Simulation,
}

/// 3 locations, travel [[0,4,7],[4,0,3],[7,3,0]] (the spec's map).
pub const MAP: &str = "[[0,4,7],[4,0,3],[7,3,0]]";

impl Harness {
    pub fn new() -> Self {
        Harness {
            sim: Simulation::new(),
        }
    }

    /// Base mode: v1 at 0, v2 at 2.
    pub fn base() -> Self {
        let mut h = Self::new();
        h.ok(&format!(
            r#"{{"command":"init","parameters":{{"travel_times":{MAP},"vessels":[{{"id":"v1","location":0}},{{"id":"v2","location":2}}]}}}}"#
        ));
        h
    }

    /// Cargo mode with the given throughputs and init manifest (JSON array text).
    pub fn cargo(throughput: &str, manifest: &str) -> Self {
        let mut h = Self::new();
        h.ok(&format!(
            r#"{{"command":"init","parameters":{{"travel_times":{MAP},"dock_throughput":{throughput},"vessels":[{{"id":"v1","location":0}},{{"id":"v2","location":0}}],"cargo_manifest":{manifest}}}}}"#
        ));
        h
    }

    /// Sends a raw line and returns the parsed response.
    pub fn send(&mut self, line: &str) -> Value {
        let out = handle_line(&mut self.sim, line).expect("non-blank line gets a response");
        if let Some(world) = self.sim.world() {
            world
                .check_invariants()
                .unwrap_or_else(|e| panic!("invariant broken after {line}: {e}"));
        }
        serde_json::from_str(&out).expect("response is valid JSON")
    }

    /// Asserts success and returns the payload.
    pub fn ok(&mut self, line: &str) -> Value {
        let v = self.send(line);
        assert_eq!(v["status"], "ok", "expected ok for {line}, got {v}");
        v["payload"].clone()
    }

    /// Asserts failure, that state did not change, and returns the message.
    pub fn err(&mut self, line: &str) -> String {
        let before = self.state_text();
        let v = self.send(line);
        assert_eq!(v["status"], "error", "expected error for {line}, got {v}");
        assert_eq!(
            self.state_text(),
            before,
            "rejected command changed state: {line}"
        );
        v["payload"]["message"].as_str().unwrap().to_string()
    }

    pub fn cmd(&mut self, name: &str, params: &str) -> Value {
        self.ok(&format!(r#"{{"command":"{name}","parameters":{params}}}"#))
    }

    pub fn cmd_err(&mut self, name: &str, params: &str) -> String {
        self.err(&format!(r#"{{"command":"{name}","parameters":{params}}}"#))
    }

    pub fn tick(&mut self) -> Value {
        self.cmd("tick", "{}")
    }

    pub fn state(&mut self) -> Value {
        self.cmd("get_state", "{}")
    }

    /// Whole planned state as text (or "uninitialized"), for unchanged-state checks.
    fn state_text(&mut self) -> String {
        match self.sim.world() {
            None => "uninitialized".into(),
            Some(_) => {
                handle_line(&mut self.sim, r#"{"command":"get_state","parameters":{}}"#).unwrap()
            }
        }
    }

    pub fn vessel(&mut self, id: &str) -> Value {
        self.state()["vessels"][id].clone()
    }
}

pub fn dest(v: &str, d: i64) -> String {
    format!(r#"{{"vessel_id":"{v}","destination":{d}}}"#)
}

pub fn vid(v: &str) -> String {
    format!(r#"{{"vessel_id":"{v}"}}"#)
}
