# Pending proposals

Proposals that were written but not yet approved for implementation. They were drafted by the
agent (Claude Code) under the working agreement in `CLAUDE.md` and reviewed by John. Nothing here
is implemented. Each one still needs explicit approval before any code changes. Once approved,
it follows the gate in `CLAUDE.md` (baseline → change → gate → one commit → `ITERATIONS.md` entry),
and its entry here moves to the "Done" table at the bottom.

This is an engineering record, not part of the design document (`docs/DESIGN.md`).

| # | Title | Area | Behavior change | Status |
|---|-------|------|-----------------|--------|
| 9b | Batch-indexed manifest application | `state.rs`, `sim.rs` | none | pinned 2026-10-24 |
| 10A | History: checkpoints + state diffs | `history.rs` (+3 lines in `sim.rs`) | none | pinned 2026-10-24 |
| 10B | History: checkpoints + command replay | `history.rs`, `sim.rs` | none | pinned 2026-10-24 |

Measurements quoted below come from one release-build run on a Windows 11 laptop (2026-10-24),
using a scratch benchmark outside the repo. Re-measure before and after implementing.

---

## Proposal 9b: Batch-indexed manifest application

```
PROPOSAL 9b: Batch-indexed manifest application
What changes:   src/state.rs: add DockInventory::add_many(&mut self, items: &[(Loc, Loc, u64)]).
                  - Manifests with <= 256 entries: loop over the existing `add` (unchanged path).
                  - Larger: build a temporary HashMap<(Loc, Loc), usize> of current positions,
                    apply each entry in O(1) via the Entry API (Occupied -> add quantity;
                    Vacant -> push to the end and record its position), then drop the map.
                src/sim.rs add_manifest: call state.docks.add_many(manifest); `manifested` gets
                  the sum of the manifest's quantities.
                Unit test in state.rs.
Why:            Benchmark: applying a manifest is O(E^2) because DockInventory::add does a
                linear scan (64k entries ~ 905 ms for one tick).
Problem fixed:  The large-manifest cliff, without slowing anything else.
Behavior change: none. A manifest only adds, never removes, so positions stay valid while the
                temporary index lives; contents and insertion order are identical to repeated
                `add` (D14/D17). The index is only looked up, never iterated into output.
Tests:          New unit test: random manifests applied via add_many vs a loop of add, compared
                with iter(). Sizes 0, 1, 256, 257, ~2000; duplicate keys, zero quantities,
                pairs already present, re-add after removal. Golden, D14/D17 and simulation
                tests guard order and conservation.
Risk:           Low. A wrong threshold only costs speed. Not fixed: load/unload lookups stay
                O(E) (~33 us at 64k entries).
```

Measured (scratch prototype, equivalence-checked on 200 random seeds):

| Manifest entries | Today | With 9b | Speedup |
|---:|---:|---:|---:|
| <= 256 | unchanged | unchanged | same code path |
| 1,000 | 326 us | 155 us | 2x |
| 4,000 | 3.9 ms | 0.66 ms | 6x |
| 16,000 | 53 ms | 2.5 ms | 21x |
| 64,000 | 905 ms | 12 ms | 73x |

**Threshold:** the index wins once a manifest has more than ~200 entries, whatever the
inventory size (building the index costs ~100 ns per existing entry; a scan costs ~1 ns per
compare).

**Rejected alternative (Proposal 9): a permanent two-map store** (HashMap + BTreeMap). It
fixed manifests (45x at 64k) but made per-tick copies 5–40x slower, render iteration 10–40x
slower and each history snapshot ~5x bigger at every inventory size. Net loss.

---

## Proposal 10A: History with checkpoints + state diffs

```
PROPOSAL 10A: History with checkpoints + state diffs
What changes:   src/history.rs: a new HistoryStore implementation that keeps a full WorldState
                  every K ticks and, for the ticks in between, only what changed since the
                  previous tick (vessels whose status or cargo changed; dock entries that changed).
                  get(T) = nearest checkpoint <= T + up to K-1 diffs, returned as Cow::Owned.
                  len() and last() move into the HistoryStore trait.
                src/sim.rs: 3 lines. The `history` field type, its constructor, and the
                  `use` line name the new type. (Alternative with zero sim.rs changes: replace
                  SnapshotHistory's internals in place, at the cost of a misleading name.)
Why:            SnapshotHistory stores 56 B per vessel per tick forever (10k vessels x 100k
                ticks ~ 56 GB), mostly duplicates: most vessels don't change on most ticks.
Problem fixed:  Unbounded history memory.
Behavior change: none. get_state_at(T) returns the same state for every T.
Tests:          New equivalence test: random runs record into both stores; get(t) must match
                for every t. Existing: get_state_at decision tests, golden fixture lines
                22-24, simulation.rs immutable-history check, check_invariants
                (history length = tick + 1, last = committed).
Risk:           A diff bug would return a wrong past state; the equivalence test and the
                simulation history check catch it. Lookup becomes O(V + K x changes) instead
                of O(1), comparable to the render that follows.
```

**Why choose 10A over 10B for history alone:** it stores results, not commands, so a later rule
change or bug fix can never rewrite past history. It also stays almost entirely inside
`history.rs`.

**Correction to earlier notes:** CLAUDE.md and DESIGN 5.3 describe this as "history.rs only".
That's true only if `SnapshotHistory` is rewritten in place. A new type needs the 3-line `sim.rs`
change above, because `sim.rs` names the concrete type and `check_invariants` uses `len()`/`last()`,
which aren't in the trait today.

---

## Proposal 10B: History with checkpoints + command replay

```
PROPOSAL 10B: History with checkpoints + command replay
What changes:   src/history.rs: store a full WorldState every K ticks plus each tick's accepted
                  commands and manifest. HistoryStore::record gains those inputs.
                src/sim.rs: advance passes `pending` and the manifest to record before clearing
                  them. A replay function rebuilds tick T by running the stored commands through
                  World's validate/apply/advance from the nearest checkpoint.
Why:            Same memory problem as 10A. In addition, a per-tick command log is the shared
                foundation for crash recovery (replay after restart), replicas (ship the log),
                an audit trail, and cancel (D10).
Problem fixed:  Unbounded history memory; no command log.
Behavior change: none. get_state_at(T) returns the same state for every T.
Tests:          Same equivalence test as 10A, plus: replaying a full run's log from tick 0
                reproduces every recorded state byte for byte (relies on determinism, DESIGN 3.4).
Risk:           Larger change across two modules. Replaying old logs with newer rules can
                change the past, so logs are tied to an engine version. Lookup re-runs up to
                K ticks of commands (slower than 10A's diffs).
```

**When to prefer 10B:** only if the command log is wanted for its own sake (crash recovery,
replicas, `cancel`). For history alone, 10A is smaller and safer.

---

## Proposal 13: Decouple protocol from sim, and rules from protocol

> **Implemented in `5b2234b` (2026-10-25).** Kept here until John makes the DESIGN.md edits
> listed at the end of this section. As built, `protocol` also gained `parse_command` (the
> `serde_json` call that `handle_line` used to make inline), and README's project layout and
> `lib.rs`'s module doc were updated in the same commit.

```
PROPOSAL 13: Decouple protocol from sim, and rules from protocol
What changes:   src/lib.rs: handle_line moves here from protocol.rs, unchanged in behavior; it
                  calls sim.execute, then world.state_for(view), then protocol::render_state.
                  `pub use` lines updated so fleet_sim::handle_line and render_error keep the
                  same paths.
                src/protocol.rs: drops `use crate::sim`. render_view(&World, View) becomes
                  render_state(names: &[String], state: &WorldState, cargo_mode: bool);
                  StateView holds those fields instead of &World. Parsing and the other views
                  are unchanged.
                src/rules.rs: check_manifest takes `impl IntoIterator<Item = (i64, i64, i64)>`
                  instead of &[protocol::ManifestEntry]; same checks, same messages.
                src/sim.rs: the two check_manifest call sites map entries to tuples.
                CLAUDE.md "Architecture" list: add lib.rs (handle_line); protocol described as
                  parse/format only.
                One new unit test in protocol.rs.
Why:            Removes both module cycles (protocol <-> sim, rules -> protocol), so DESIGN 4.1
                shows a strict layering: lib orchestrates, protocol only parses and formats,
                rules only depends on state. From John's review of the 4.1 dependency list.
Problem fixed:  protocol, which should be a pure wire layer, depends on the engine it sits in
                front of; rules depends on a serde wire type to validate plain numbers.
Behavior change: none. Every response is byte-identical. Public API paths (fleet_sim::handle_line,
                render_error, Simulation) unchanged.
Tests:          No existing test changes: golden, CLI, decision and random tests all go through
                fleet_sim::handle_line and compare output exactly.
                + protocol unit test render_state_needs_no_world: renders a hand-built WorldState
                  in base and cargo mode and checks the raw line (key order; cargo fields only in
                  cargo mode), showing protocol is now testable without a World.
Risk:           Low. A render or ordering slip fails the golden tests byte for byte. A manifest
                validation slip fails d13_rejected_tick_does_not_advance_time,
                d12_invalid_init_is_rejected_and_init_can_be_retried and the D15 manifest tests.
                Cost: churn in four files for no user-visible change.
```

**Why protocol depends on sim today:** two reasons, and both must change.
1. `handle_line` calls `sim.execute` and `sim.world()`. That's orchestration and belongs in `lib.rs`.
2. The renderer takes `&World` to call `state_for`, `cargo_mode` and `vessel_names`. It must take
   plain data instead.

Moving only `handle_line` would leave the dependency in place.

**Why not move `ManifestEntry` into `state.rs`:** it is a serde wire type (`Deserialize`,
`deny_unknown_fields`). `state.rs` has no dependencies today; moving it would add serde to the
data model. Passing plain numbers to `check_manifest` keeps the wire type in `protocol`.

**Dependency graph:**

```
BEFORE (two cycles)                    AFTER (acyclic)
main     → lib                         main     → lib
protocol → sim, state, error           lib      → protocol, sim
sim      → protocol, rules, state,     protocol → state, error
           history, error              sim      → protocol (Command types only), rules,
rules    → protocol, state, error                 state, history, error
history  → state                       rules    → state, error
                                       history  → state
```

**DESIGN.md edits it would require** (John's document):

| Where | Change |
|---|---|
| 4.1 table, `lib.rs` row | Also owns `handle_line` (parse → execute → render) |
| 4.1 table, `protocol.rs` row | Parse and format only; no engine calls |
| 4.1 Mermaid diagram | Remove `protocol --> sim` and `rules --> protocol`; add `lib --> protocol` and `lib --> sim`; `sim --> protocol` means "Command types" only |
| 4.1 note ("Later on, might move…") | Obsolete: delete, or state that the layering is strict |
| 4.2 command trace, steps 2 and 7 | `handle_line` is in `lib.rs`; it passes names, state and cargo mode to `protocol::render_state` |
| 6, row "One output line per command" | `protocol.rs handle_line` → `lib.rs handle_line` |
| 7, unit tests bullet | Add `protocol.rs` (`render_state_needs_no_world`) |

---

## Done

| # | Title | Commits |
|---|-------|---------|
| 11 | Dedicated decision tests (D10, D16, D17 + gaps in D5, D7, D9, D15) | 8c48142, log 670ad7f |
| 12 | Base mode rejects any cargo_manifest field, even an empty one (D15) | 3b38d12, log 027bc54 |
| 13 | Decouple protocol from sim, and rules from protocol | 5b2234b |
