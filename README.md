# Fleet Simulation Engine

A deterministic, turn-based fleet simulator written in Rust. It reads one JSON command per line on stdin and writes exactly one JSON response per line on stdout.

Implements the base specification, Follow-on 1 (cargo and dock operations), and Follow-on 2 (historical state queries).

## Authors
- John Lee - [GitHub](https://github.com/michaeljohndan) - 2003jjl@gmail.com

## Quick start

```bash
cargo build --release
cargo run --release < tests/fixtures/base.in.jsonl     # the spec's base example
scripts/check.sh                                       # format, lint, and every test
```

## Requirements

- **Rust** (stable) with `rustfmt` and `clippy`. Install via [rustup](https://rustup.rs).
- **Bash** for the helper scripts:
  - macOS and Linux: N/A (built in).
  - NixOS: works as-is (scripts use `#!/usr/bin/env bash`). Install Rust via rustup or Nix.
  - Windows: use `git bash`. Plain `cargo` commands work in any shell.

Runtime dependencies: 
- `serde`
- `serde_json`

## Usage

The program is driven by another program or by hand, one JSON command per line:

```bash
cargo run --release                     # interactive: paste a line, press Enter; Ctrl+D to quit
cargo run --release < commands.jsonl    # from a file
```

Example:

```text
→ {"command":"init","parameters":{"travel_times":[[0,4,7],[4,0,3],[7,3,0]],"vessels":[{"id":"v1","location":0},{"id":"v2","location":2}]}}
← {"status":"ok","payload":{"tick":0,"vessels":{"v1":{"state":"idle","location":0},"v2":{"state":"idle","location":2}}}}
→ {"command":"set_destination","parameters":{"vessel_id":"v1","destination":1}}
← {"status":"ok","payload":{"tick":0,"vessels":{"v1":{"state":"transit","from":0,"to":1,"arrives_at_tick":4},"v2":{"state":"idle","location":2}}}}
```

Every response is either `{"status":"ok","payload":<world state>}` or `{"status":"error","payload":{"message":"<reason>"}}`. Blank lines are ignored.

### Commands

| Command | Parameters | Effect |
|---|---|---|
| `init` | `travel_times`, `vessels`, optional `dock_throughput`, `cargo_manifest` | Creates the world at tick 0 (must be first) |
| `tick` | optional `cargo_manifest` | Advances time one tick. Pending commands take effect, and the planned state becomes the commited state. |
| `set_destination` | `vessel_id`, `destination` | Sends an idle vessel to a location |
| `dock` / `undock` | `vessel_id` | Parks / releases a vessel at its location |
| `load` / `unload` | `vessel_id`, `destination` | Moves one unit of cargo (identified by destination) |
| `swap` | `vessel_id`, `load_destination`, `unload_destination` | One load plus one unload |
| `get_state` | none | Returns the planned state, which means the current state, including commands issued this tick |
| `get_state_at` | `tick` | Committed state recorded at the end of a past tick. Commands still pending in the current tick are never included |

**Cargo mode:** cargo commands and cargo fields in the output are enabled when `init` includes `dock_throughput`. Without it, the program behaves exactly as the base spec.

## Testing

```bash
scripts/check.sh            # the full gate: cargo fmt --check, clippy (warnings = errors), all tests
cargo test                  # tests only
scripts/install-hooks.sh    # once per clone: every git commit runs scripts/check.sh first
```

| Suite | What it checks |
|---|---|
| `tests/golden.rs` | Both spec walkthroughs reproduced byte for byte |
| `tests/decisions.rs` | Every implementation decision and spec rule, invariants after every command |
| `tests/simulation.rs` | Seeded random command sequences: invariants, rejected commands change nothing, history never changes, same seed gives identical output |
| `tests/cli.rs` | The real binary over stdin/stdout, including blank lines, CRLF, invalid UTF-8 |

Randomized testing:

```bash
FLEET_SIM_SEEDS=5000 cargo test --release --test simulation   # runs 5000 random sequences instead of 300, testing interleavings and more cases.
FLEET_SIM_SEED=1234 cargo test --test simulation              # reproduce one failing seed
```

Find the commit that broke the build:

```bash
scripts/bisect.sh <last-known-good-commit>
```

**CI** (GitHub Actions) runs the full gate on Linux and Windows on every push, plus a 2,000-seed randomized run; the macOS job runs on demand ("Run workflow").

## Demo scenarios

`scenarios/` contains ready-to-run command files that exercise specific behavior:

```bash
cargo run --release < scenarios/history.jsonl
```

## Benchmark

```bash
cargo run --release --example bench -- <vessels> <ticks>
```

## Project layout

```text
src/
  main.rs        I/O loop: one line in, one line out
  protocol.rs    JSON commands and output format
  sim.rs         engine: validate each command, then apply it
  rules.rs       pure rule helpers (throughput counting, range checks)
  state.rs       data model: vessels, cargo, docks, map
  history.rs     committed snapshots for get_state_at
  error.rs       every error message
tests/           golden, decision, randomized, and end-to-end tests
scripts/         check, bisect, and git-hook scripts
scenarios/       demo command files
examples/        benchmark
docs/DESIGN.md   design document
```

## Documentation

- [`README.md`](README.md): **(This document.)** Written for anyone cloning the repo, installation, execution, tests, CI/CD, and a high-level overview of the project layout.
- [`docs/DESIGN.md`](docs/DESIGN.md): Design document that features architecture, decisions regarding program behavior, and verification. This must be written by the engineer, which is why AI agents are strictly told not to write in this file.
- [`CLAUDE.md`](CLAUDE.md): context for AI-assisted changes. Made to specify workflows for engineers and agents to pick up where the previous development left off. Future changes should follow the guideline set here. Specifically, it includes:
    -  The working agreement that specifies that the `scripts/check.sh` is run before every commit, all changes are logged, and behavior is not changed without approval.
    -  The decisions table (D1 - D18) to highlight the command behavior of everything. 
- [`ITERATIONS.md`](ITERATIONS.md): log of every change and its test result. Used to record development and help with debugging. 
    - If something breaks, this file with `scripts/bisect.sh` tells the engineer which change caused it. 
    - Every AI-made change is recorded with a reason and a passing check, so nothing slips past unreviewed.