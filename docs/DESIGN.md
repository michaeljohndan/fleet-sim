<!--
DESIGN DOC TEMPLATE: Fleet Simulation Engine
Structure adapted from MIL-STD-498 Software Design Description (DI-IPSC-81435),
trimmed for a take-home; decisions use the Architecture Decision Record (ADR) format.

HOW TO USE
- Save as docs/DESIGN.md in the repo. Every sentence must be your own writing: the spec
  forbids AI for the design doc. These hidden comments are questions to answer; delete
  each one once its section is written.
- Target length: about 5-8 pages. Short, specific sentences beat long ones.
- Use the spec's words for spec facts and your own words for everything else.
-->

# Fleet Simulation Engine: Software Design Description

| | |
|---|---|
| Author | |
| Version / commit | |
| Date | |
| Status | Draft / Final |

## 1. Scope

### 1.1 Identification
<!-- What is this software called, what version or commit does this doc describe,
     and which parts of the spec does it implement (base, Follow-on 1, Follow-on 2)? -->

### 1.2 System overview
<!-- In 3-5 sentences: what problem does the simulator solve, who or what drives it
     (a person? another program?), and what are its inputs and outputs? -->

### 1.3 Document overview
<!-- One or two sentences: what this document covers and how it is organized. -->

### 1.4 Goals and non-goals
<!-- Goals in priority order (e.g. correctness, determinism, ...). Why that order?
     Non-goals: what did you deliberately leave out, and why is that the right call
     for this scope? -->

## 2. Referenced documents
<!-- The spec PDF (title, version/date), plus anything you relied on (e.g. Rust or serde
     docs). Only list what you actually used. -->

## 3. System-wide design decisions

### 3.1 Core model: committed vs. planned state
<!-- Explain the two states in your own words. What does get_state show, and what does
     get_state_at show? What is a tick, and what is a tick boundary? -->

### 3.2 The golden rule
<!-- State the one rule every validation follows. Why one rule instead of case-by-case
     logic? Which of the spec's own statements does it explain for free? -->

### 3.3 Behavior and error handling
<!-- What does a caller get back when something is wrong? What guarantee holds for a
     rejected command? Why does that matter to a caller? -->

### 3.4 Determinism
<!-- What does "same input, same output" require of the implementation? What did you
     avoid to guarantee it, and what does it make possible? -->

### 3.5 Implementation decisions (ADR format)
<!-- One entry per decision D1-D18. Keep each to 4-8 lines. Format:

#### Dn: <short title>
- **Context:** the situation, and what the spec says or leaves open.
- **Decision:** what the program does (one sentence).
- **Rationale:** why: cite the golden rule, a spec quote, or a consequence you avoided.
- **Alternatives rejected:** the other option(s), and why they lose.
- **Consequences / tradeoffs:** what this costs or limits, honestly stated.
- **Verified by:** test name(s).
-->

## 4. Architectural design

### 4.1 Components
<!-- Table: module | single responsibility | why it is separate.
     Then a diagram you draw yourself showing how they depend on each other. -->

| Module | Responsibility | Why separate |
|---|---|---|
| | | |

### 4.2 Concept of execution
<!-- Walk one command from stdin to stdout. Where does parsing happen, where is it
     validated, where does state change, where is output written? Then walk one tick. -->

### 4.3 Interface design
<!-- The stdin/stdout protocol: input shape, the two response shapes, one-line-per-command
     rule, ordering of output fields. What did you do about the base vs. cargo-mode
     protocol conflict? -->

### 4.4 Resource usage
<!-- Time per command and memory, in terms of vessels / locations / ticks. What does the
     benchmark show, and what is the real bottleneck? -->

## 5. Detailed design

### 5.1 Data structures
<!-- For each key structure (vessel status, cargo lists, dock inventory, travel times,
     vessel ids, history): what you chose, why, and what you rejected. -->

### 5.2 Throughput accounting
<!-- Explain how operations are counted, with a worked example. Why does the cost depend
     only on the resulting state? What exploit does that prevent? -->

### 5.3 History
<!-- How get_state_at works, what it costs in memory, and how you would change it at
     scale without touching other modules. -->

## 6. Requirements traceability
<!-- Map each spec requirement to the code that implements it and the test that proves
     it. Fill every blank cell; add rows for anything missing. -->

| Spec requirement | Module / function | Test(s) |
|---|---|---|
| init must be first; second init errors | | |
| Commands validated against committed state | | |
| set_destination: arrival tick, override before tick | | |
| Arriving vessel reports idle at its arrival tick | | |
| get_state returns planned state | | |
| One output line per command, in order | | |
| dock / undock rules and overrides | | |
| load / unload / swap validity | | |
| Delivery at destination removes cargo | | |
| Dock throughput shared across vessels per tick | | |
| cargo_manifest validation (init and tick) | | |
| get_state_at returns committed state; future/negative errors | | |

## 7. Verification
<!-- The test layers you have, what kind of bug each one catches, and how to run them.
     How do you know the random tests can actually catch bugs? How is a breaking commit
     found? What does the CI / pre-commit gate enforce? -->

## 8. Known limitations and future work
<!-- Be honest: what is imperfect today (e.g. the D17 ordering quirk, history memory),
     and what you would build next (e.g. cancel, capacity, docking time, delta output).
     For each: why it is not done yet, and where in the code it would land. -->

## 9. Notes

### 9.1 Open questions
<!-- Anything you would confirm with the team (e.g. strict array order in grading). -->

### 9.2 Glossary
<!-- Terms a new reader needs: tick, committed state, planned state, pending log,
     operation, delivery, cargo mode. Define them in your own words. -->