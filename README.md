# fleet_sim

A deterministic, turn-based fleet simulation. It reads one JSON command per line on stdin
and writes exactly one JSON response per line on stdout. Implements the base spec,
Follow-on 1 (cargo and docks), and Follow-on 2 (historical queries).

```bash
cargo build --release
./target/release/fleet_sim < tests/fixtures/base.in.jsonl
scripts/check.sh          # format, lint, and every test
```

Cargo features turn on when `init` includes `dock_throughput`; without it the program
behaves exactly like the base spec.

See `CLAUDE.md` for the architecture, the implementation decisions (D1–D16), the test
suite, and the working agreement for changes.
