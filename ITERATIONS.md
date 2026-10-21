# Iteration log

One entry per commit, newest first. The agent appends an entry after every change;
`scripts/bisect.sh <last green commit>` finds the first bad one if the gate ever fails.

Template:

```
## <n>. <title> — <commit hash>
What:     <files / functions changed>
Why:      <reason; decision ID or spec section>
Fixes:    <problem resolved>
Behavior: <none | outputs that changed>
Gate:     scripts/check.sh PASS (<test count> tests) | FAIL (<what>)
Approved: <yes, by John on YYYY-MM-DD>
```

## 4. Context file, iteration log, benchmark — (this commit)
What:     CLAUDE.md, ITERATIONS.md, README.md, examples/bench.rs, .claude/settings.json Stop hook
Why:      hand-off context for the next agent or engineer; measurable performance claims
Fixes:    no shared working agreement; no evidence that output dominates cost
Behavior: none
Gate:     scripts/check.sh PASS
Approved: requested by John on 2026-10-21

## 3. Seeded randomized and CLI tests — 6e6ae77
What:     tests/simulation.rs (300 seeds x 400 commands), tests/cli.rs
Why:      catch bugs no hand-written case covers; prove determinism and atomicity
Fixes:    no coverage of random command interleavings or the real binary
Behavior: none (verified the suite catches a planted throughput bug, then reverted it)
Gate:     scripts/check.sh PASS; FLEET_SIM_SEEDS=5000 PASS
Approved: requested by John on 2026-10-21

## 2. Decision tests — afcdf9b
What:     tests/common/mod.rs harness, tests/decisions.rs (34 tests)
Why:      one executable check per State Matrix decision D1-D16
Fixes:    decisions documented but not enforced
Behavior: none
Gate:     scripts/check.sh PASS
Approved: requested by John on 2026-10-21

## 1. Engine and golden tests — be11448
What:     src/* (protocol, sim, rules, state, history, error, main), tests/golden.rs, scripts/
Why:      implement base spec + Follow-on 1 + Follow-on 2 under the strict golden rule
Fixes:    initial implementation
Behavior: both spec walkthroughs reproduced byte for byte
Gate:     scripts/check.sh PASS
Approved: requested by John on 2026-10-21
