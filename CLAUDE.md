# Fleet Sim — agent and engineer context

Rust take-home for Saronic: a deterministic, turn-based fleet simulator. One JSON command
per stdin line, exactly one JSON response per stdout line. The engineer (John) owns every
decision; the agent proposes, the engineer approves.

## Commands

- Full gate (fmt + clippy + all tests): `scripts/check.sh` — must pass before every commit
- Tests only: `scripts/check.sh --fast` or `cargo test`
- Reproduce a random-test failure: `FLEET_SIM_SEED=<seed> cargo test --test simulation`
- Deeper random run: `FLEET_SIM_SEEDS=5000 cargo test --release --test simulation`
- Find the commit that broke things: `scripts/bisect.sh <last-good-commit>`
- Install the pre-commit gate (once per clone): `scripts/install-hooks.sh`
- Run: `cargo run --release < tests/fixtures/base.in.jsonl`
- Benchmark: `cargo run --release --example bench -- <vessels> <ticks>`

## Working agreement (IMPORTANT — follow exactly)

### 1. Propose before changing anything
Before editing any file, post a proposal and WAIT for the engineer's explicit approval:

```
PROPOSAL <n>: <short title>
What changes:   <files / functions / tests, exactly>
Why:            <the reason, citing the spec or a decision ID (D1–D16) where relevant>
Problem fixed:  <the concrete bug, gap, or request this resolves>
Behavior change: <none | exactly which outputs change, with an example input>
Tests:          <tests added or changed, and why the old ones were wrong if changed>
Risk:           <what could break; how the gate would catch it>
```

- One proposal = one logical change = one commit. Do not bundle unrelated edits.
- Never change behavior covered by a decision (D1–D16) or a fixture without saying so
  explicitly under "Behavior change".
- No new dependencies without a proposal that names the crate and why std can't do it.
- If the engineer's request conflicts with the golden rule or a decision, say so first.

### 2. Every iteration goes through the gate
- Run `scripts/check.sh` BEFORE starting (record the baseline) and AFTER the change.
- Never commit with a failing gate. Never weaken, skip, delete, or `#[ignore]` a test to
  make it pass. Never edit `tests/fixtures/*` without approval: they are copied from the spec.
- Every behavior change gets a test that fails before the change and passes after.
- After committing, append an entry to `ITERATIONS.md` (template inside) with the commit
  hash and the gate result.
- If something breaks and the cause is unclear, run `scripts/bisect.sh` from the last
  green commit in `ITERATIONS.md` and report the first bad commit.
- A Claude Code Stop hook (`.claude/settings.json`) re-runs the gate before a turn ends.

## The golden rule (every validation follows it)

> A command is valid iff the COMMITTED state allows it. A valid command replaces the
> vessel's pending command (last one wins). Cargo commands also check PLANNED inventory
> and dock throughput.

- committed = official state as of the last tick; planned = committed + this tick's
  accepted commands; `get_state` and every command response print PLANNED;
  `get_state_at(T)` prints COMMITTED at T.
- A tick is a turn, not wall-clock time. Nothing reads a clock.

## Decisions (left to the implementer by the spec)

| ID | Decision |
|----|----------|
| D1 | set_destination to current location → planned idle there (also cancels pending trip/dock) |
| D2 | dock twice same tick → no-op; dock when committed docked → "not idle" |
| D3 | undock after pending dock → "not docked" |
| D4 | undock twice same tick → no-op; undock when committed idle → "not docked" |
| D5 | dock after pending undock → "not idle" (pending undock can't be reversed in-tick) |
| D6 | undock after load/unload/swap → allowed; both commit on tick |
| D7 | load/unload/swap after pending undock → allowed; same result as reverse order |
| D8 | throughput counts NET change: per vessel max(L, U), summed per dock; load+unload same unit = 0 |
| D9 | inventory checked against PLANNED (no double loads; B may load what A unloaded) |
| D10 | no implicit "undo" rule; explicit `cancel` command is a future extension |
| D11 | malformed input → error line, one response per line, blank lines skipped |
| D12 | invalid init → error, world stays uninitialized (init can be retried) |
| D13 | every rejected command leaves state unchanged (tick with bad manifest doesn't advance) |
| D14 | output order: vessels in init order, cargo in insertion order (entry removed at 0) |
| D15 | cargo mode on iff init has `dock_throughput`; base mode prints no cargo fields |
| D16 | all messages in `SimError`'s `Display`; spec wording reproduced exactly |

## Architecture (one job per module)

- `src/main.rs` — I/O loop only: read line, `handle_line`, write, flush. Single thread.
- `src/protocol.rs` — serde `Command` enum (adjacently tagged) + output views with fixed key order.
- `src/sim.rs` — `Simulation` (uninitialized/running) and `World`. `execute` = `validate`
  (pure, returns `Effect` or `SimError`) then `apply` (infallible). Also `check_invariants`.
- `src/rules.rs` — pure helpers: `vessel_operations` (D8), `dock_operations`, range checks.
- `src/state.rs` — data: `Status` enum, `CargoList`, `DockInventory`, `WorldState`, `Topology`.
- `src/history.rs` — `HistoryStore` trait; v1 `SnapshotHistory` (full snapshot per tick).
- `src/error.rs` — `SimError` and every message.

Rules for code:
- Validation never mutates; `apply` never fails. Keep that split for every new command.
- New command = new `Command` variant + `validate` arm + `Effect` (if it mutates) + tests.
- New status = new `Status` variant; let the compiler find every `match` to update.
- Output order must be deterministic: never iterate a `HashMap` into output.
- No `unwrap`/`expect`/`panic` outside tests except for invariants already guaranteed by
  validation, each with a message saying why it can't fail.
- `serde_json::Value` sorts keys: never use it to check output ORDER in tests; compare raw lines.

## Tests (all run by `scripts/check.sh`)

- `tests/golden.rs` — both spec walkthroughs, byte for byte (`tests/fixtures`, see its README).
- `tests/decisions.rs` — one or more tests per D1–D16 and per spec rule; the harness asserts
  invariants after every command and unchanged state after every rejection.
- `tests/simulation.rs` — seeded random command sequences (300 seeds × 400 commands):
  invariants, rejection atomicity, immutable history, determinism.
- `tests/cli.rs` — the real binary: fixtures, blank lines, CRLF, invalid UTF-8.
- Unit tests in `state.rs` and `rules.rs`.

Invariants (`World::check_invariants`): planned/committed ticks equal; history has tick+1
entries and its last equals committed; cargo conserved (in world + delivered = manifested);
no vessel in transit past its arrival tick; cargo changes only on committed-docked vessels;
no dock over throughput; no cargo in base mode.

## Scaling notes

Every response prints the whole world, so output is O(vessels) per command and dominates
(see `examples/bench.rs`). Measure before optimizing. Planned seams: delta responses
(protocol), arrivals index + overlay planned state (sim), checkpoint + replay history
(history.rs only), sparse route graph (`Topology::travel_time` only), actor-per-simulation
server (main.rs only), command-log replication. The full rationale is in the design reference doc.

## Status and open questions

- Done: base spec, Follow-on 1 (cargo), Follow-on 2 (history), full test suite.
- Ask the recruiter (Tim): should follow-ons be in the submission? Does the grader compare
  JSON byte-for-byte / array order? Is auto-detecting cargo mode (D15) acceptable?
- Extension ready to propose if asked: `cancel(vessel_id)` — drop that vessel's entries
  from `pending`, replay the rest from committed, reject the cancel if any becomes invalid.
- The design doc for the submission is written by John, not the agent.
