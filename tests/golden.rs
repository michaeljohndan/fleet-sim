//! Golden tests: the spec's two stdin/stdout walkthroughs, compared byte for byte.
//! Fixture lines marked "derived" in tests/fixtures/README.md fill ticks the PDF skipped.

use fleet_sim::{handle_line, Simulation};

fn run_fixture(name: &str, input: &str, expected: &str) {
    let mut sim = Simulation::new();
    let inputs: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();
    let outputs: Vec<&str> = expected.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        inputs.len(),
        outputs.len(),
        "{name}: fixture files are misaligned"
    );
    for (i, (line, want)) in inputs.iter().zip(&outputs).enumerate() {
        let got = handle_line(&mut sim, line).expect("non-blank line produces a response");
        assert_eq!(
            &got,
            want,
            "{name} line {}: command {line}\n  got:  {got}\n  want: {want}",
            i + 1
        );
        if let Some(world) = sim.world() {
            world
                .check_invariants()
                .unwrap_or_else(|e| panic!("{name} line {}: invariant broken: {e}", i + 1));
        }
    }
}

#[test]
fn base_spec_walkthrough() {
    run_fixture(
        "base",
        include_str!("fixtures/base.in.jsonl"),
        include_str!("fixtures/base.expected.jsonl"),
    );
}

#[test]
fn cargo_and_history_spec_walkthrough() {
    run_fixture(
        "cargo_and_history",
        include_str!("fixtures/cargo_and_history.in.jsonl"),
        include_str!("fixtures/cargo_and_history.expected.jsonl"),
    );
}
