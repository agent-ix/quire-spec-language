---
id: TC-577
title: "The breadth-first proviso prevents ignoring, the closure reaches through disabled members, and choice is deterministic"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-156
    type: verifies
---
# TC-577: The breadth-first proviso prevents ignoring, the closure reaches through disabled members, and choice is deterministic

## Description

Verify C3's full expansion on a cycle that would ignore a visible transition, the necessary-enabling closure, and determinism.

Scope: FR-156-AC-3 to FR-156-AC-4.

## Test Procedure

Fixtures: The `Ring` unit of FR-156-AC-3; the `Gate` unit of FR-157 for the closure.

1. Run `always holds(q.v < 2)` over `Ring` with partial-order reduction, recording ample sets; run it unreduced; replay the counterexample.
2. Compute the candidate seeded by `arm` at the `Gate` initial state; run TC-576 step 1 twice.

Tag the tests `#[trace("TC-577", "FR-156-AC-n")]`.

## Expected Results

- Step 1: `{step(r)}` at `(0, 0)` and `(1, 0)`; full expansion at `(2, 0)`; `Violated` with the six-step prefix, which replays; unreduced `Violated`.
- Step 2: the candidate holds `arm`, `arm2` and `go`, with enabled part `{arm}`; equal ample sets and outcomes.
