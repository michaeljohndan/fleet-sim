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

## 6. Cross platform support: LF line endings, OS ignores, CI on Linux/macOS/Windows — 100644
What:     .gitattributes, .github/workflows/ci.yml, docs/DESIGN.md
Why:      Good practice to include design decision for future reader. CI/CD        pipelines for os-agnostic systems.
Fixes:    lack of design documentation and CI/CD testing
Behavior: none (no output changes for any input)
Gate:     sscripts/check.sh PASS (54 tests; baseline 51 at 100644)
Approved: approved by John on 2026-10-21

## 5. D18: zero-cost same-destination swap — 3c02b67
What:     tests/decisions.rs (+3 D18 tests), src/sim.rs (comment on Swap arm), CLAUDE.md (D18 row)
Why:      D18 — a swap with load == unload changes no cargo, so it costs 0 like
          load+unload of the same destination; cost depends only on the result
Fixes:    behavior existed but was untested and undocumented; a change to "count
          every swap as 1" could have broken it silently
Behavior: none (no output changes for any input)
Gate:     scripts/check.sh PASS (54 tests; baseline 51 at 4433ffb)
Approved: Proposal 1, approved by John on 2026-10-21

## 4. Context file, iteration log, benchmark — cf270a6
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
