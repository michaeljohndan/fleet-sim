//! One test (or more) per decision in the State Matrix workbook, plus the spec's
//! stated rules. Golden rule: validity is checked against COMMITTED state; a valid
//! command replaces the vessel's pending command; cargo amounts use PLANNED state.

mod common;
use common::{dest, vid, Harness};
use serde_json::json;

// ------------------------------------------------------------ init / phases

#[test]
fn commands_before_init_are_rejected() {
    let mut h = Harness::new();
    assert_eq!(h.cmd_err("tick", "{}"), "simulation not initialized");
    assert_eq!(h.cmd_err("get_state", "{}"), "simulation not initialized");
}

#[test]
fn second_init_is_rejected() {
    let mut h = Harness::base();
    let msg = h.cmd_err(
        "init",
        r#"{"travel_times":[[0]],"vessels":[{"id":"x","location":0}]}"#,
    );
    assert_eq!(msg, "simulation already initialized");
}

#[test]
fn d12_invalid_init_is_rejected_and_init_can_be_retried() {
    let mut h = Harness::new();
    let bad = [
        (
            r#"{"travel_times":[],"vessels":[]}"#,
            "at least one location",
        ),
        (
            r#"{"travel_times":[[0,1],[1]],"vessels":[]}"#,
            "must be square",
        ),
        (
            r#"{"travel_times":[[1,1],[1,0]],"vessels":[]}"#,
            "must be 0",
        ),
        (
            r#"{"travel_times":[[0,0],[1,0]],"vessels":[]}"#,
            "must be a positive integer",
        ),
        (
            r#"{"travel_times":[[0,1],[1,0]],"vessels":[{"id":"a","location":2}]}"#,
            "out of range",
        ),
        (
            r#"{"travel_times":[[0,1],[1,0]],"vessels":[{"id":"a","location":0},{"id":"a","location":1}]}"#,
            "duplicate vessel id a",
        ),
        (
            r#"{"travel_times":[[0,1],[1,0]],"dock_throughput":[1],"vessels":[]}"#,
            "dock_throughput has 1 entries",
        ),
        (
            r#"{"travel_times":[[0,1],[1,0]],"dock_throughput":[1,0],"vessels":[]}"#,
            "must be a positive integer",
        ),
        (
            r#"{"travel_times":[[0,1],[1,0]],"dock_throughput":[1,1],"vessels":[],"cargo_manifest":[{"location":5,"destination":0,"quantity":1}]}"#,
            "out of range",
        ),
    ];
    for (params, needle) in bad {
        let msg = h.cmd_err("init", params);
        assert!(
            msg.contains(needle),
            "init {params}: got '{msg}', wanted '{needle}'"
        );
        assert!(
            h.sim.world().is_none(),
            "failed init must leave the world uninitialized"
        );
    }
    h.cmd(
        "init",
        r#"{"travel_times":[[0,1],[1,0]],"vessels":[{"id":"a","location":0}]}"#,
    );
}

// --------------------------------------------------------------- base spec

#[test]
fn set_destination_sets_arrival_and_last_command_wins() {
    let mut h = Harness::base();
    h.cmd("set_destination", &dest("v1", 2));
    let v1 = h.cmd("set_destination", &dest("v1", 1))["vessels"]["v1"].clone();
    assert_eq!(
        v1,
        json!({"state":"transit","from":0,"to":1,"arrives_at_tick":4})
    );
}

#[test]
fn set_destination_in_transit_is_rejected_with_spec_message() {
    let mut h = Harness::base();
    h.cmd("set_destination", &dest("v1", 1));
    h.tick();
    assert_eq!(
        h.cmd_err("set_destination", &dest("v1", 2)),
        "vessel v1 is not idle"
    );
}

#[test]
fn arrival_reports_idle_and_vessel_can_be_commanded_same_tick() {
    let mut h = Harness::base();
    h.cmd("set_destination", &dest("v1", 1));
    for _ in 0..3 {
        assert_eq!(h.tick()["vessels"]["v1"]["state"], "transit");
    }
    assert_eq!(
        h.tick()["vessels"]["v1"],
        json!({"state":"idle","location":1})
    );
    h.cmd("set_destination", &dest("v1", 2));
}

#[test]
fn unknown_vessel_and_out_of_range_destination_are_rejected() {
    let mut h = Harness::base();
    assert_eq!(
        h.cmd_err("set_destination", &dest("v9", 1)),
        "unknown vessel v9"
    );
    assert_eq!(
        h.cmd_err("set_destination", &dest("v1", 3)),
        "destination 3 is out of range (0-2)"
    );
    assert_eq!(
        h.cmd_err("set_destination", &dest("v1", -1)),
        "destination -1 is out of range (0-2)"
    );
}

#[test]
fn d1_set_destination_to_current_location_stays_idle_and_cancels_pending_trip() {
    let mut h = Harness::base();
    h.cmd("set_destination", &dest("v1", 2));
    let v1 = h.cmd("set_destination", &dest("v1", 0))["vessels"]["v1"].clone();
    assert_eq!(v1, json!({"state":"idle","location":0}));
    assert_eq!(
        h.tick()["vessels"]["v1"],
        json!({"state":"idle","location":0})
    );
}

// ------------------------------------------------------------ dock / undock

#[test]
fn d2_dock_twice_same_tick_is_a_no_op() {
    let mut h = Harness::cargo("[2,2,2]", "[]");
    h.cmd("dock", &vid("v1"));
    let v1 = h.cmd("dock", &vid("v1"))["vessels"]["v1"].clone();
    assert_eq!(v1, json!({"state":"docked","location":0,"cargo":[]}));
}

#[test]
fn d2_dock_when_committed_docked_is_not_idle() {
    let mut h = Harness::cargo("[2,2,2]", "[]");
    h.cmd("dock", &vid("v1"));
    h.tick();
    assert_eq!(h.cmd_err("dock", &vid("v1")), "vessel v1 is not idle");
}

#[test]
fn dock_and_set_destination_override_each_other() {
    let mut h = Harness::cargo("[2,2,2]", "[]");
    h.cmd("dock", &vid("v1"));
    assert_eq!(
        h.cmd("set_destination", &dest("v1", 1))["vessels"]["v1"]["state"],
        "transit"
    );
    assert_eq!(
        h.cmd("dock", &vid("v1"))["vessels"]["v1"]["state"],
        "docked"
    );
    assert_eq!(h.tick()["vessels"]["v1"]["state"], "docked");
}

#[test]
fn d1_set_destination_to_current_location_cancels_pending_dock() {
    let mut h = Harness::cargo("[2,2,2]", "[]");
    h.cmd("dock", &vid("v1"));
    let v1 = h.cmd("set_destination", &dest("v1", 0))["vessels"]["v1"].clone();
    assert_eq!(v1["state"], "idle");
}

#[test]
fn d3_undock_after_pending_dock_is_not_docked() {
    let mut h = Harness::cargo("[2,2,2]", "[]");
    h.cmd("dock", &vid("v1"));
    assert_eq!(h.cmd_err("undock", &vid("v1")), "vessel v1 is not docked");
}

#[test]
fn d4_undock_twice_same_tick_is_a_no_op_but_not_after_tick() {
    let mut h = Harness::cargo("[2,2,2]", "[]");
    h.cmd("dock", &vid("v1"));
    h.tick();
    h.cmd("undock", &vid("v1"));
    assert_eq!(
        h.cmd("undock", &vid("v1"))["vessels"]["v1"]["state"],
        "idle"
    );
    h.tick();
    assert_eq!(h.cmd_err("undock", &vid("v1")), "vessel v1 is not docked");
}

#[test]
fn d5_dock_after_pending_undock_is_not_idle() {
    let mut h = Harness::cargo("[2,2,2]", "[]");
    h.cmd("dock", &vid("v1"));
    h.tick();
    h.cmd("undock", &vid("v1"));
    assert_eq!(h.cmd_err("dock", &vid("v1")), "vessel v1 is not idle");
}

#[test]
fn set_destination_while_docked_is_not_idle() {
    let mut h = Harness::cargo("[2,2,2]", "[]");
    h.cmd("dock", &vid("v1"));
    h.tick();
    assert_eq!(
        h.cmd_err("set_destination", &dest("v1", 1)),
        "vessel v1 is not idle"
    );
}

// -------------------------------------------------------------------- cargo

const TWO_UNITS: &str =
    r#"[{"location":0,"destination":2,"quantity":1},{"location":0,"destination":1,"quantity":1}]"#;

fn docked(throughput: &str, manifest: &str) -> Harness {
    let mut h = Harness::cargo(throughput, manifest);
    h.cmd("dock", &vid("v1"));
    h.tick();
    h
}

#[test]
fn cargo_commands_need_committed_docked() {
    let mut h = Harness::cargo("[2,2,2]", TWO_UNITS);
    h.cmd("dock", &vid("v1"));
    assert_eq!(h.cmd_err("load", &dest("v1", 2)), "vessel v1 is not docked");
}

#[test]
fn d6_load_then_undock_commits_both() {
    let mut h = docked("[2,2,2]", TWO_UNITS);
    h.cmd("load", &dest("v1", 2));
    h.cmd("undock", &vid("v1"));
    let v1 = h.tick()["vessels"]["v1"].clone();
    assert_eq!(
        v1,
        json!({"state":"idle","location":0,"cargo":[{"destination":2,"quantity":1}]})
    );
}

#[test]
fn d7_undock_then_load_is_allowed_and_matches_reverse_order() {
    let mut a = docked("[2,2,2]", TWO_UNITS);
    a.cmd("load", &dest("v1", 2));
    a.cmd("undock", &vid("v1"));
    let mut b = docked("[2,2,2]", TWO_UNITS);
    b.cmd("undock", &vid("v1"));
    b.cmd("load", &dest("v1", 2));
    assert_eq!(a.tick(), b.tick());
}

#[test]
fn d8_load_then_unload_same_unit_costs_zero_operations() {
    let mut h = docked(
        "[1,2,2]",
        r#"[{"location":0,"destination":2,"quantity":2}]"#,
    );
    h.cmd("load", &dest("v1", 2));
    h.cmd("unload", &dest("v1", 2));
    h.cmd("load", &dest("v1", 2));
    assert_eq!(
        h.cmd_err("load", &dest("v1", 2)),
        "transfer of 2 operations exceeds dock throughput of 1 at location 0"
    );
}

#[test]
fn d8_separate_load_and_unload_cost_one_like_a_swap() {
    let mut h = docked(
        "[1,2,2]",
        r#"[{"location":0,"destination":2,"quantity":1},{"location":0,"destination":1,"quantity":1}]"#,
    );
    h.cmd("load", &dest("v1", 2));
    h.tick();
    h.cmd("load", &dest("v1", 1));
    h.cmd("unload", &dest("v1", 2));
    let v1 = h.tick()["vessels"]["v1"]["cargo"].clone();
    assert_eq!(v1, json!([{"destination":1,"quantity":1}]));
}

#[test]
fn spec_third_operation_exceeds_throughput_of_two() {
    let mut h = docked(
        "[2,4,1]",
        r#"[{"location":0,"destination":2,"quantity":2},{"location":0,"destination":1,"quantity":1}]"#,
    );
    h.cmd("load", &dest("v1", 2));
    h.cmd("load", &dest("v1", 1));
    assert_eq!(
        h.cmd_err("load", &dest("v1", 2)),
        "transfer of 3 operations exceeds dock throughput of 2 at location 0"
    );
}

#[test]
fn throughput_is_shared_by_all_vessels_at_a_dock() {
    let mut h = Harness::cargo(
        "[2,2,2]",
        r#"[{"location":0,"destination":1,"quantity":3}]"#,
    );
    h.cmd("dock", &vid("v1"));
    h.cmd("dock", &vid("v2"));
    h.tick();
    h.cmd("load", &dest("v1", 1));
    h.cmd("load", &dest("v2", 1));
    assert!(h
        .cmd_err("load", &dest("v1", 1))
        .starts_with("transfer of 3 operations"));
}

#[test]
fn d9_vessel_can_load_what_another_unloaded_and_a_unit_cannot_be_loaded_twice() {
    let mut h = Harness::cargo(
        "[4,2,2]",
        r#"[{"location":0,"destination":1,"quantity":1}]"#,
    );
    h.cmd("dock", &vid("v1"));
    h.cmd("dock", &vid("v2"));
    h.tick();
    h.cmd("load", &dest("v1", 1));
    assert_eq!(
        h.cmd_err("load", &dest("v2", 1)),
        "dock 0 has no cargo for destination 1"
    );
    h.cmd("unload", &dest("v1", 1));
    h.cmd("load", &dest("v2", 1));
    let s = h.tick();
    assert_eq!(s["vessels"]["v1"]["cargo"], json!([]));
    assert_eq!(
        s["vessels"]["v2"]["cargo"],
        json!([{"destination":1,"quantity":1}])
    );
}

#[test]
fn swap_costs_one_and_reorders_like_the_spec() {
    let mut h = docked(
        "[1,2,2]",
        r#"[{"location":0,"destination":2,"quantity":1},{"location":0,"destination":1,"quantity":1}]"#,
    );
    h.cmd("load", &dest("v1", 2));
    h.tick();
    let s = h.cmd(
        "swap",
        r#"{"vessel_id":"v1","load_destination":1,"unload_destination":2}"#,
    );
    assert_eq!(
        s["vessels"]["v1"]["cargo"],
        json!([{"destination":1,"quantity":1}])
    );
    assert_eq!(
        s["cargo"],
        json!([{"location":0,"destination":2,"quantity":1}])
    );
}

#[test]
fn unload_at_destination_delivers_and_removes_cargo() {
    let mut h = Harness::cargo(
        "[2,2,2]",
        r#"[{"location":0,"destination":0,"quantity":1}]"#,
    );
    h.cmd("dock", &vid("v1"));
    h.tick();
    h.cmd("load", &dest("v1", 0));
    h.tick();
    let s = h.cmd("unload", &dest("v1", 0));
    assert_eq!(s["vessels"]["v1"]["cargo"], json!([]));
    assert_eq!(s["cargo"], json!([]));
}

#[test]
fn missing_cargo_is_rejected() {
    let mut h = docked("[2,2,2]", "[]");
    assert_eq!(
        h.cmd_err("load", &dest("v1", 1)),
        "dock 0 has no cargo for destination 1"
    );
    assert_eq!(
        h.cmd_err("unload", &dest("v1", 1)),
        "vessel v1 has no cargo for destination 1"
    );
}

// --------------------------------------------------------------- D11 - D16

#[test]
fn d11_malformed_input_is_an_error_line_and_changes_nothing() {
    let mut h = Harness::base();
    for line in [
        "not json",
        r#"{"command":"fly","parameters":{}}"#,
        r#"{"command":"set_destination","parameters":{"vessel_id":"v1"}}"#,
        r#"{"command":"set_destination","parameters":{"vessel_id":"v1","destination":"two"}}"#,
        r#"{"command":"set_destination","parameters":{"vessel_id":"v1","destination":1,"speed":9}}"#,
    ] {
        assert!(h.err(line).starts_with("invalid command:"), "{line}");
    }
    assert!(
        fleet_sim::handle_line(&mut h.sim, "   ").is_none(),
        "blank lines are skipped"
    );
}

#[test]
fn d13_rejected_tick_does_not_advance_time() {
    let mut h = Harness::cargo("[2,2,2]", "[]");
    let msg = h.cmd_err(
        "tick",
        r#"{"cargo_manifest":[{"location":0,"destination":1,"quantity":1},{"location":0,"destination":9,"quantity":1}]}"#,
    );
    assert!(
        msg.starts_with("invalid cargo_manifest: entry 1 destination 9"),
        "{msg}"
    );
    let msg = h.cmd_err(
        "tick",
        r#"{"cargo_manifest":[{"location":0,"destination":1,"quantity":0}]}"#,
    );
    assert!(msg.contains("must be a positive integer"), "{msg}");
    assert_eq!(h.state()["tick"], 0);
}

#[test]
fn d14_vessels_print_in_init_order_and_cargo_in_insertion_order() {
    // Check the raw output line: serde_json::Value would re-sort the keys.
    let mut sim = fleet_sim::Simulation::new();
    let text = fleet_sim::handle_line(
        &mut sim,
        r#"{"command":"init","parameters":{"travel_times":[[0,1],[1,0]],"dock_throughput":[3,3],"vessels":[{"id":"zeta","location":0},{"id":"alpha","location":1}],"cargo_manifest":[{"location":0,"destination":1,"quantity":1},{"location":0,"destination":0,"quantity":1}]}}"#,
    )
    .unwrap();
    assert!(
        text.find("zeta").unwrap() < text.find("alpha").unwrap(),
        "{text}"
    );
    assert!(
        text.ends_with(r#""cargo":[{"location":0,"destination":1,"quantity":1},{"location":0,"destination":0,"quantity":1}]}}"#),
        "{text}"
    );
}

#[test]
fn d15_base_mode_has_no_cargo_fields_and_rejects_cargo_commands() {
    let mut h = Harness::base();
    let s = h.state();
    assert!(s.get("cargo").is_none());
    assert!(s["vessels"]["v1"].get("cargo").is_none());
    assert_eq!(
        h.cmd_err("dock", &vid("v1")),
        "cargo operations require dock_throughput at init"
    );
    assert_eq!(
        h.cmd_err(
            "tick",
            r#"{"cargo_manifest":[{"location":0,"destination":1,"quantity":1}]}"#
        ),
        "cargo operations require dock_throughput at init"
    );
}

// ------------------------------------------------------------------ history

#[test]
fn get_state_at_returns_committed_snapshots_and_rejects_bad_ticks() {
    let mut h = Harness::base();
    h.cmd("set_destination", &dest("v1", 1));
    let t1 = h.tick();
    h.tick();
    assert_eq!(h.cmd("get_state_at", r#"{"tick":1}"#), t1);
    assert_eq!(
        h.cmd("get_state_at", r#"{"tick":0}"#)["vessels"]["v1"],
        json!({"state":"idle","location":0})
    );
    assert_eq!(
        h.cmd_err("get_state_at", r#"{"tick":99}"#),
        "tick 99 is in the future (current tick: 2)"
    );
    assert_eq!(
        h.cmd_err("get_state_at", r#"{"tick":-1}"#),
        "tick -1 is negative"
    );
}

#[test]
fn get_state_at_current_tick_shows_committed_not_pending() {
    let mut h = Harness::base();
    h.tick();
    assert_eq!(
        h.cmd("get_state_at", r#"{"tick":1}"#),
        h.state(),
        "identical at a tick boundary"
    );
    h.cmd("set_destination", &dest("v1", 1));
    assert_eq!(
        h.cmd("get_state_at", r#"{"tick":1}"#)["vessels"]["v1"]["state"],
        "idle"
    );
    assert_eq!(h.state()["vessels"]["v1"]["state"], "transit");
}

#[test]
fn pending_log_records_accepted_commands_and_clears_on_tick() {
    let mut h = Harness::base();
    h.cmd("set_destination", &dest("v1", 1));
    h.cmd_err("set_destination", &dest("v9", 1));
    assert_eq!(h.sim.world().unwrap().pending().len(), 1);
    h.tick();
    assert!(h.sim.world().unwrap().pending().is_empty());
}

// ---------------------------------------------------------------------- D18

/// Dock 0 throughput 1; v1 already carries one dest-2 unit (committed);
/// the dock still holds one dest-2 unit and one dest-1 unit.
fn carrying_dest_two() -> Harness {
    let mut h = docked(
        "[1,2,2]",
        r#"[{"location":0,"destination":2,"quantity":2},{"location":0,"destination":1,"quantity":1}]"#,
    );
    h.cmd("load", &dest("v1", 2));
    h.tick();
    h
}

#[test]
fn d18_same_destination_swap_costs_zero_operations() {
    let mut h = carrying_dest_two();
    h.cmd(
        "swap",
        r#"{"vessel_id":"v1","load_destination":2,"unload_destination":2}"#,
    );
    // The swap used none of the budget of 1, so a real load still fits...
    h.cmd("load", &dest("v1", 1));
    // ...and the next one is the first to exceed it.
    assert_eq!(
        h.cmd_err("load", &dest("v1", 2)),
        "transfer of 2 operations exceeds dock throughput of 1 at location 0"
    );
}

#[test]
fn d18_same_destination_swap_changes_no_cargo() {
    let mut h = carrying_dest_two();
    let before = h.state();
    h.cmd(
        "swap",
        r#"{"vessel_id":"v1","load_destination":2,"unload_destination":2}"#,
    );
    let after = h.tick();
    assert_eq!(
        after["vessels"]["v1"]["cargo"],
        before["vessels"]["v1"]["cargo"]
    );
    // Dock contents are identical; only their order may differ (D17).
    let sorted = |v: &serde_json::Value| {
        let mut items: Vec<String> = v["cargo"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.to_string())
            .collect();
        items.sort();
        items
    };
    assert_eq!(sorted(&after), sorted(&before));
}

#[test]
fn d18_every_path_to_the_same_state_costs_the_same() {
    // Throughput 1: each path must cost exactly 1 to be accepted.
    let swap = r#"{"vessel_id":"v1","load_destination":1,"unload_destination":2}"#;
    let mut a = carrying_dest_two();
    a.cmd("swap", swap);
    let mut b = carrying_dest_two();
    b.cmd("load", &dest("v1", 1));
    b.cmd("unload", &dest("v1", 2));
    let mut c = carrying_dest_two();
    c.cmd("unload", &dest("v1", 2));
    c.cmd("load", &dest("v1", 1));
    let (ta, tb, tc) = (a.tick(), b.tick(), c.tick());
    assert_eq!(ta["vessels"], tb["vessels"]);
    assert_eq!(ta["vessels"], tc["vessels"]);
    assert_eq!(
        ta["vessels"]["v1"]["cargo"],
        json!([{"destination":1,"quantity":1}])
    );
}

// ---------------------------------- decision coverage: D5 D7 D9 D10 D15 D17

#[test]
fn d5_dock_after_committed_undock_redocks() {
    let mut h = docked("[2,2,2]", "[]");
    h.cmd("undock", &vid("v1"));
    h.tick();
    assert_eq!(
        h.cmd("dock", &vid("v1"))["vessels"]["v1"]["state"],
        "docked"
    );
    assert_eq!(h.tick()["vessels"]["v1"]["state"], "docked");
}

#[test]
fn d7_unload_and_swap_after_pending_undock_are_allowed() {
    let mut a = carrying_dest_two();
    a.cmd("undock", &vid("v1"));
    assert_eq!(
        a.cmd("unload", &dest("v1", 2))["vessels"]["v1"],
        json!({"state":"idle","location":0,"cargo":[]})
    );
    let mut b = carrying_dest_two();
    b.cmd("undock", &vid("v1"));
    b.cmd(
        "swap",
        r#"{"vessel_id":"v1","load_destination":1,"unload_destination":2}"#,
    );
    assert_eq!(
        b.tick()["vessels"]["v1"],
        json!({"state":"idle","location":0,"cargo":[{"destination":1,"quantity":1}]})
    );
}

#[test]
fn d9_transfer_between_vessels_costs_each_vessel() {
    // Netting is per vessel, not per dock: a v1 -> v2 hand-off leaves the dock's
    // inventory unchanged but still costs 1 + 1, so a third operation is over.
    let mut h = Harness::cargo(
        "[2,2,2]",
        r#"[{"location":0,"destination":1,"quantity":2}]"#,
    );
    h.cmd("dock", &vid("v1"));
    h.cmd("dock", &vid("v2"));
    h.tick();
    h.cmd("load", &dest("v1", 1));
    h.tick();
    h.cmd("unload", &dest("v1", 1));
    h.cmd("load", &dest("v2", 1));
    assert_eq!(
        h.cmd_err("load", &dest("v2", 1)),
        "transfer of 3 operations exceeds dock throughput of 2 at location 0"
    );
}

#[test]
fn d10_no_implicit_undo_validity_ignores_pending_commands() {
    // Committed idle: undock is rejected the same way with or without a pending dock.
    let mut alone = Harness::cargo("[2,2,2]", "[]");
    let mut after_dock = Harness::cargo("[2,2,2]", "[]");
    after_dock.cmd("dock", &vid("v1"));
    assert_eq!(
        after_dock.cmd_err("undock", &vid("v1")),
        alone.cmd_err("undock", &vid("v1"))
    );
    // The pending dock was not undone.
    assert_eq!(after_dock.tick()["vessels"]["v1"]["state"], "docked");

    // Committed docked: dock and set_destination are rejected the same way with or
    // without a pending undock.
    let mut alone = docked("[2,2,2]", "[]");
    let mut after_undock = docked("[2,2,2]", "[]");
    after_undock.cmd("undock", &vid("v1"));
    for (name, params) in [("dock", vid("v1")), ("set_destination", dest("v1", 1))] {
        assert_eq!(
            after_undock.cmd_err(name, &params),
            alone.cmd_err(name, &params),
            "{name}"
        );
    }
    // The pending undock was not undone.
    assert_eq!(after_undock.tick()["vessels"]["v1"]["state"], "idle");
}

#[test]
fn d15_init_manifest_needs_dock_throughput() {
    let mut h = Harness::new();
    assert_eq!(
        h.cmd_err(
            "init",
            r#"{"travel_times":[[0,1],[1,0]],"vessels":[],"cargo_manifest":[{"location":0,"destination":1,"quantity":1}]}"#
        ),
        "cargo operations require dock_throughput at init"
    );
    // D12: a failed init leaves the world uninitialized, so init can be retried.
    assert!(h.sim.world().is_none());
    h.cmd("init", r#"{"travel_times":[[0,1],[1,0]],"vessels":[]}"#);
}

#[test]
fn d15_base_mode_rejects_any_cargo_manifest_even_empty() {
    let mut h = Harness::new();
    assert_eq!(
        h.cmd_err(
            "init",
            r#"{"travel_times":[[0,1],[1,0]],"vessels":[],"cargo_manifest":[]}"#
        ),
        "cargo operations require dock_throughput at init"
    );
    assert!(h.sim.world().is_none());

    let mut h = Harness::base();
    assert_eq!(
        h.cmd_err("tick", r#"{"cargo_manifest":[]}"#),
        "cargo operations require dock_throughput at init"
    );
    assert_eq!(h.state()["tick"], 0);

    // `null` counts as absent, so base mode accepts it on tick and on init.
    assert_eq!(h.cmd("tick", r#"{"cargo_manifest":null}"#)["tick"], 1);
    let mut h = Harness::new();
    h.cmd(
        "init",
        r#"{"travel_times":[[0,1],[1,0]],"vessels":[],"cargo_manifest":null}"#,
    );
}

#[test]
fn d15_every_cargo_command_is_rejected_in_base_mode() {
    let mut h = Harness::base();
    let swap = r#"{"vessel_id":"v1","load_destination":1,"unload_destination":2}"#;
    for (name, params) in [
        ("undock", vid("v1")),
        ("load", dest("v1", 1)),
        ("unload", dest("v1", 1)),
        ("swap", swap.to_string()),
    ] {
        assert_eq!(
            h.cmd_err(name, &params),
            "cargo operations require dock_throughput at init",
            "{name}"
        );
    }
    let s = h.tick();
    assert!(s.get("cargo").is_none());
    assert!(s["vessels"]["v1"].get("cargo").is_none());
}

#[test]
fn d17_load_then_unload_moves_dock_entry_to_end() {
    // TWO_UNITS puts dest 2 before dest 1 on dock 0.
    let mut h = docked("[2,2,2]", TWO_UNITS);
    h.cmd("load", &dest("v1", 2)); // dest-2 entry reaches 0 and is removed
    let s = h.cmd("unload", &dest("v1", 2)); // re-added at the end
    let moved = json!([
        {"location":0,"destination":1,"quantity":1},
        {"location":0,"destination":2,"quantity":1}
    ]);
    assert_eq!(s["cargo"], moved);
    assert_eq!(s["vessels"]["v1"]["cargo"], json!([]));
    // Only the order changed, and the new order is what commits.
    assert_eq!(h.tick()["cargo"], moved);
}

#[test]
fn d17_unload_then_load_moves_vessel_entry_to_end() {
    let mut h = docked("[2,2,2]", TWO_UNITS);
    h.cmd("load", &dest("v1", 2));
    h.cmd("load", &dest("v1", 1));
    h.tick(); // v1 carries [dest 2, dest 1]
    h.cmd("unload", &dest("v1", 2));
    let s = h.cmd("load", &dest("v1", 2));
    assert_eq!(
        s["vessels"]["v1"]["cargo"],
        json!([{"destination":1,"quantity":1},{"destination":2,"quantity":1}])
    );
    assert_eq!(s["cargo"], json!([]));
}

// ------------------------------------------------------------ D19 D20 recall

/// Two ports with the given travel matrix; v1 idle at 0.
fn two_ports(travel: &str) -> Harness {
    let mut h = Harness::new();
    h.cmd(
        "init",
        &format!(r#"{{"travel_times":{travel},"vessels":[{{"id":"v1","location":0}}]}}"#),
    );
    h
}

fn ticks(h: &mut Harness, n: u64) {
    for _ in 0..n {
        h.tick();
    }
}

#[test]
fn d19_recall_is_rejected_when_committed_idle() {
    let mut h = Harness::base();
    assert_eq!(
        h.cmd_err("recall", &vid("v1")),
        "vessel v1 is not in transit"
    );
    assert_eq!(h.cmd_err("recall", &vid("v9")), "unknown vessel v9");
}

#[test]
fn d19_recall_does_not_cancel_a_pending_trip() {
    let mut h = Harness::base();
    h.cmd("set_destination", &dest("v1", 1));
    // Planned shows transit, but committed is idle (same pattern as D5).
    assert_eq!(
        h.cmd_err("recall", &vid("v1")),
        "vessel v1 is not in transit"
    );
    assert_eq!(
        h.tick()["vessels"]["v1"],
        json!({"state":"transit","from":0,"to":1,"arrives_at_tick":4})
    );
}

#[test]
fn d19_recall_is_rejected_when_docked_or_dock_is_pending() {
    let mut h = Harness::cargo("[2,2,2]", "[]");
    h.cmd("dock", &vid("v1"));
    assert_eq!(
        h.cmd_err("recall", &vid("v1")),
        "vessel v1 is not in transit"
    );
    h.tick();
    assert_eq!(
        h.cmd_err("recall", &vid("v1")),
        "vessel v1 is not in transit"
    );
}

#[test]
fn d19_recall_is_rejected_on_the_arrival_tick() {
    let mut h = Harness::base();
    h.cmd("set_destination", &dest("v1", 1));
    ticks(&mut h, 3);
    h.cmd("recall", &vid("v1")); // last tick before arrival is still in transit
    let mut h = Harness::base();
    h.cmd("set_destination", &dest("v1", 1));
    ticks(&mut h, 4);
    assert_eq!(h.vessel("v1"), json!({"state":"idle","location":1}));
    assert_eq!(
        h.cmd_err("recall", &vid("v1")),
        "vessel v1 is not in transit"
    );
    assert_eq!(
        h.tick()["vessels"]["v1"],
        json!({"state":"idle","location":1})
    );
}

#[test]
fn d19_repeat_recall_same_tick_is_idempotent_and_rejected_after_tick() {
    let mut h = two_ports("[[0,10],[10,0]]");
    h.cmd("set_destination", &dest("v1", 1));
    ticks(&mut h, 6);
    let first = h.cmd("recall", &vid("v1"));
    assert_eq!(h.cmd("recall", &vid("v1")), first);
    for _ in 7..12 {
        h.tick();
        assert_eq!(
            h.cmd_err("recall", &vid("v1")),
            "vessel v1 is already returning"
        );
    }
    h.tick(); // tick 12: back at 0
    assert_eq!(
        h.cmd_err("recall", &vid("v1")),
        "vessel v1 is not in transit"
    );
    h.cmd("set_destination", &dest("v1", 1));
    h.tick();
    h.cmd("recall", &vid("v1")); // a new outbound trip can be recalled
}

#[test]
fn d20_sixty_percent_out_takes_sixty_percent_back() {
    // Example 1: travel 10, depart tick 0, recall at tick 6 -> back at tick 12.
    let mut h = two_ports("[[0,10],[10,0]]");
    h.cmd("set_destination", &dest("v1", 1));
    ticks(&mut h, 6);
    let returning = json!({"state":"returning","from":1,"to":0,"arrives_at_tick":12});
    assert_eq!(h.cmd("recall", &vid("v1"))["vessels"]["v1"], returning);
    for t in 7..12 {
        let s = h.tick();
        assert_eq!(s["tick"], t);
        assert_eq!(s["vessels"]["v1"], returning);
    }
    assert_eq!(
        h.tick()["vessels"]["v1"],
        json!({"state":"idle","location":0})
    );
    let at7 = h.cmd("get_state_at", r#"{"tick":7}"#);
    assert_eq!(at7["vessels"]["v1"], returning);

    // Example 2: travel 5, depart tick 3 (arrives 8), recall at tick 6 -> back at 9.
    let mut h = two_ports("[[0,5],[5,0]]");
    ticks(&mut h, 3);
    h.cmd("set_destination", &dest("v1", 1));
    ticks(&mut h, 3);
    assert_eq!(
        h.cmd("recall", &vid("v1"))["vessels"]["v1"],
        json!({"state":"returning","from":1,"to":0,"arrives_at_tick":9})
    );
}

#[test]
fn d20_earliest_and_latest_recall() {
    let mut h = two_ports("[[0,10],[10,0]]");
    h.cmd("set_destination", &dest("v1", 1));
    h.tick();
    assert_eq!(
        h.cmd("recall", &vid("v1"))["vessels"]["v1"]["arrives_at_tick"],
        2
    );
    let mut h = two_ports("[[0,10],[10,0]]");
    h.cmd("set_destination", &dest("v1", 1));
    ticks(&mut h, 9);
    assert_eq!(
        h.cmd("recall", &vid("v1"))["vessels"]["v1"]["arrives_at_tick"],
        18
    );
}

#[test]
fn d20_return_uses_outbound_elapsed_not_reverse_travel_time() {
    let mut h = two_ports("[[0,10],[2,0]]");
    h.cmd("set_destination", &dest("v1", 1));
    ticks(&mut h, 6);
    assert_eq!(
        h.cmd("recall", &vid("v1"))["vessels"]["v1"]["arrives_at_tick"],
        12
    );
}

#[test]
fn d20_returning_vessel_keeps_cargo_and_rejects_other_commands() {
    let mut h = Harness::new();
    h.cmd(
        "init",
        r#"{"travel_times":[[0,10],[10,0]],"dock_throughput":[1,1],"vessels":[{"id":"v1","location":0}],"cargo_manifest":[{"location":0,"destination":1,"quantity":1}]}"#,
    );
    h.cmd("dock", &vid("v1"));
    h.tick();
    h.cmd("load", &dest("v1", 1));
    h.cmd("undock", &vid("v1"));
    h.tick();
    h.cmd("set_destination", &dest("v1", 1)); // tick 2, arrives 12
    ticks(&mut h, 3);
    h.cmd("recall", &vid("v1")); // tick 5: elapsed 3, back at 8
    let s = h.tick();
    assert_eq!(
        s["vessels"]["v1"],
        json!({"state":"returning","from":1,"to":0,"arrives_at_tick":8,
               "cargo":[{"destination":1,"quantity":1}]})
    );
    assert_eq!(
        h.cmd_err("set_destination", &dest("v1", 1)),
        "vessel v1 is not idle"
    );
    assert_eq!(h.cmd_err("dock", &vid("v1")), "vessel v1 is not idle");
    assert_eq!(h.cmd_err("undock", &vid("v1")), "vessel v1 is not docked");
    assert_eq!(
        h.cmd_err("unload", &dest("v1", 1)),
        "vessel v1 is not docked"
    );
}

#[test]
fn recall_output_prints_returning_in_fixed_key_order() {
    let mut sim = fleet_sim::Simulation::new();
    let lines = [
        r#"{"command":"init","parameters":{"travel_times":[[0,10],[10,0]],"vessels":[{"id":"v1","location":0}]}}"#,
        r#"{"command":"set_destination","parameters":{"vessel_id":"v1","destination":1}}"#,
        r#"{"command":"tick","parameters":{}}"#,
    ];
    for line in lines {
        fleet_sim::handle_line(&mut sim, line).unwrap();
    }
    let out = fleet_sim::handle_line(
        &mut sim,
        r#"{"command":"recall","parameters":{"vessel_id":"v1"}}"#,
    )
    .unwrap();
    assert_eq!(
        out,
        r#"{"status":"ok","payload":{"tick":1,"vessels":{"v1":{"state":"returning","from":1,"to":0,"arrives_at_tick":2}}}}"#
    );
}
