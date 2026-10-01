---
id: US-026
title: "Check timing claims over a timed model"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-231
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-232
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-233
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-234
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-235
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-236
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-237
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-238
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-239
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-240
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-241
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-242
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-243
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-244
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-245
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-246
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-247
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-248
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-249
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-251
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-252
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-253
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-254
    type: exercises
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-026: Check timing claims over a timed model

## Story

**As a** verification operator building a robot controller or an embedded
real-time system in QSL
**I want** to give my state model real-valued clocks, guards, deadlines and
urgency, state claims such as "a reply comes within 3 ms", check a task set's
schedulability, measure timing probabilities, and carry the claims onto the
target as monitors and code obligations
**So that** I get `proved` with a certificate a small checker accepts, a
counterexample with exact delays that I can replay, or a stated reason when
the check cannot decide, and so that a model that stops time by mistake is
reported to me.

## Context

ADR-026 designs dense time. A timed model declares a `time` member and
clocks; its behaviours alternate real delays and discrete steps, and only
behaviours whose time diverges count. A claim over it binds `model-time` by
default, so an interval counts time units. QSL's native zone engine checks
timed claims and returns a zone certificate with every proof; a native
closed-form engine checks schedulability; hybrid dynamics go to external
solvers under QSpec FR-193; stochastic delays are sampled by the statistical
engine; and the claim reaches an embedded target as a tick-based monitor and
Kani obligations.

## Acceptance Examples (Illustrative)

### US-026-EX-1: A deadline race is refuted with exact delays

- **Given** ADR-026 §11's `Rpc` model with `T = 3 ms` and the claim
  `NoLateReply`.
- **When** the operator requests the claim.
- **Then** it settles `refuted` with the steps `send` at 0, `timeout` after
  a delay of 3 and `reply` after a delay of 0, and replaying the
  counterexample through the model reproduces the refutation.

### US-026-EX-2: A proof carries a certificate

- **Given** the same model with `T = 4 ms` and its claim `Settles` under
  the timed profile.
- **When** the operator requests it.
- **Then** it settles `proved`, and the proof counts only after the
  certificate checker accepts its zone certificate.

### US-026-EX-3: A model that stops time is reported

- **Given** the `Rpc` variant whose `reply` and `timeout` guards are
  strict, with `T = 3 ms`.
- **When** the operator requests any claim over it.
- **Then** the request also carries the subject's time-lock-freedom item,
  which settles `refuted` with the stem `send` at 0 and a final delay of 3.

### US-026-EX-4: A task set is checked in closed form

- **Given** the task set `Ctl` of ADR-026 §12.
- **When** the operator requests `schedulable Ctl under fixed-priority`.
- **Then** it settles `proved` with the response times 1, 3 and 12 as
  evidence, recomputed before the verdict settles, and with the third
  task's WCET raised to 6 it settles `refuted`.

### US-026-EX-5: A timing probability is measured

- **Given** ADR-026 §11's stochastic variant and the claim `RareLate`.
- **When** the operator requests it with statistical evidence.
- **Then** it settles `measured`, `Rejected`, and every sampled run replays
  with exact rational delays.

## Priority and Risk (Informative)

Priority: High. Robots and embedded real-time systems state their
requirements as deadlines, response bounds and schedulability, and those
need time units, not steps.

## Traceability (Informative)

FR-230 to FR-254.
