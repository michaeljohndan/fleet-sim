# Iteration log

One entry per commit, newest first. The agent appends an entry after every change;
`scripts/bisect.sh <last green commit>` finds the first bad one if the gate ever fails.

Template:

```markdown
## <n>. <title> — `<commit hash>`

- **What:** <files / functions changed>
- **Why:** <reason; decision ID or spec section>
- **Fixes:** <problem resolved>
- **Behavior:** <none | outputs that changed>
- **Gate:** `scripts/check.sh` PASS (<test count> tests) | FAIL (<what>)
- **Approved:** <yes, by John on YYYY-MM-DD>
```

## 11. Design doc, D15/D16 decision text, pending proposals — `98ac8da`

- **What:** `docs/DESIGN.md` (all sections written), `CLAUDE.md` (D15 and D16 decision rows),
  `docs/PENDING_PROPOSALS.md` (new: Proposals 9b, 10A, 10B, 13; done table for 11 and 12)
- **Why:** submission design doc; D15/D16 wording updated by John on 2026-10-24; pinned proposals
  kept in the repo instead of only in agent memory
- **Fixes:** design doc was an unfinished template; D15/D16 text didn't state the empty-manifest and
  message-style rules; pending proposals had no record in the repo
- **Behavior:** none (docs only)
- **Gate:** `scripts/check.sh` PASS (64 tests; baseline 64 at `027bc54`)
- **Approved:** `DESIGN.md` written and committed by John; `CLAUDE.md` D15/D16 rows and `PENDING_PROPOSALS.md`
  requested by John on 2026-10-24

## 10. D15: base mode rejects any `cargo_manifest` field, even an empty one — `3b38d12`

- **What:** `src/sim.rs` (`World::new` and `validate` Tick-arm guards), `tests/decisions.rs`
  (`d15_base_mode_rejects_any_cargo_manifest_even_empty`)
- **Why:** D15 updated by John on 2026-10-24: base mode rejects any mention of cargo; `null` counts as absent
- **Fixes:** `DESIGN.md`/`CLAUDE.md` stated the rule but base mode still accepted `"cargo_manifest":[]`
- **Behavior:** base-mode init with `"cargo_manifest":[]` -> error `cargo operations require dock_throughput
  at init`, world stays uninitialized (D12); base-mode tick with `[]` -> same error, tick does not
  advance (D13). Unchanged: cargo mode, `null` manifests, both spec fixtures
- **Gate:** `scripts/check.sh` PASS (64 tests; baseline 63 at `670ad7f`). New test failed before the change
- **Approved:** Proposal 12, approved by John on 2026-10-24

## 9. Dedicated decision tests: D10, D16, D17 + gaps in D5, D7, D9, D15 — `8c48142`

- **What:** `tests/decisions.rs` (8 tests), `src/error.rs` (new test module: `d16_every_error_message_is_pinned`)
- **Why:** D10/D16 had only indirect coverage; D17 was asserted nowhere; D5, D7, D9, D15 had untested
  parts of their decision text (DESIGN 3.5)
- **Fixes:** regressions in D17 ordering, message wording, implicit undo, or per-vessel netting would
  have passed the gate
- **Behavior:** none (test-only; tests pin existing behavior)
- **Gate:** `scripts/check.sh` PASS (63 tests; baseline 54 at `3585ffe`). Mutation check: 7 targeted rule
  breaks in a scratch copy, each caught by its new test (D5/D7 not mutation-checked)
- **Approved:** Proposal 11, approved by John on 2026-10-24

## 8. Correct iteration 6 log entry — `661c396`

- **What:** `ITERATIONS.md` (entry 6 heading and Gate line)
- **Why:** `scripts/bisect.sh` relies on the log's hashes to find the last green commit
- **Fixes:** entry 6 hash was "100644" (a file mode from a diff header), "sscripts" typo,
  baseline recorded as 51 tests at 100644 instead of 54 at `c32464f`
- **Behavior:** none (docs only)
- **Gate:** `scripts/check.sh` PASS before and after (baseline at `bd59a71`)
- **Approved:** Proposal 7 (hash, typo, baseline; What line left as written), approved by John on 2026-10-22

## 7. README: LinkedIn link in Authors — `13a71db`

- **What:** `README.md` (Authors line)
- **Why:** author contact info requested by John
- **Fixes:** README had no LinkedIn link
- **Behavior:** none (docs only)
- **Gate:** `scripts/check.sh` PASS before and after (baseline at `273957a`)
- **Approved:** Proposal 6, approved by John on 2026-10-22

## 6. Cross platform support: LF line endings, OS ignores, CI on Linux/macOS/Windows — `839340d`

- **What:** `.gitattributes`, `.github/workflows/ci.yml`, `docs/DESIGN.md`
- **Why:** Good practice to include design decision for future reader. CI/CD pipelines for os-agnostic systems.
- **Fixes:** lack of design documentation and CI/CD testing
- **Behavior:** none (no output changes for any input)
- **Gate:** `scripts/check.sh` PASS (54 tests; baseline 54 at `c32464f`)
- **Approved:** approved by John on 2026-10-21

## 5. D18: zero-cost same-destination swap — `3c02b67`

- **What:** `tests/decisions.rs` (+3 D18 tests), `src/sim.rs` (comment on Swap arm), `CLAUDE.md` (D18 row)
- **Why:** D18 — a swap with load == unload changes no cargo, so it costs 0 like
  load+unload of the same destination; cost depends only on the result
- **Fixes:** behavior existed but was untested and undocumented; a change to "count
  every swap as 1" could have broken it silently
- **Behavior:** none (no output changes for any input)
- **Gate:** `scripts/check.sh` PASS (54 tests; baseline 51 at `4433ffb`)
- **Approved:** Proposal 1, approved by John on 2026-10-21

## 4. Context file, iteration log, benchmark — `cf270a6`

- **What:** `CLAUDE.md`, `ITERATIONS.md`, `README.md`, `examples/bench.rs`, `.claude/settings.json` Stop hook
- **Why:** hand-off context for the next agent or engineer; measurable performance claims
- **Fixes:** no shared working agreement; no evidence that output dominates cost
- **Behavior:** none
- **Gate:** `scripts/check.sh` PASS
- **Approved:** requested by John on 2026-10-21

## 3. Seeded randomized and CLI tests — `6e6ae77`

- **What:** `tests/simulation.rs` (300 seeds x 400 commands), `tests/cli.rs`
- **Why:** catch bugs no hand-written case covers; prove determinism and atomicity
- **Fixes:** no coverage of random command interleavings or the real binary
- **Behavior:** none (verified the suite catches a planted throughput bug, then reverted it)
- **Gate:** `scripts/check.sh` PASS; `FLEET_SIM_SEEDS=5000` PASS
- **Approved:** requested by John on 2026-10-21

## 2. Decision tests — `afcdf9b`

- **What:** `tests/common/mod.rs` harness, `tests/decisions.rs` (34 tests)
- **Why:** one executable check per State Matrix decision D1-D16
- **Fixes:** decisions documented but not enforced
- **Behavior:** none
- **Gate:** `scripts/check.sh` PASS
- **Approved:** requested by John on 2026-10-21

## 1. Engine and golden tests — `be11448`

- **What:** `src/*` (protocol, sim, rules, state, history, error, main), `tests/golden.rs`, `scripts/`
- **Why:** implement base spec + Follow-on 1 + Follow-on 2 under the strict golden rule
- **Fixes:** initial implementation
- **Behavior:** both spec walkthroughs reproduced byte for byte
- **Gate:** `scripts/check.sh` PASS
- **Approved:** requested by John on 2026-10-21
