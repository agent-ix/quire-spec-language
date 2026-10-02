---
id: TC-575
title: "Footprints are enforced: a read outside the footprint is an internal fault, a write outside the frame a violation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-155
    type: verifies
---
# TC-575: Footprints are enforced: a read outside the footprint is an internal fault, a write outside the frame a violation

## Description

Verify read enforcement through the restricted observation, write enforcement through `check_frame`, and determinism.

Scope: FR-155-AC-3 to FR-155-AC-4.

## Test Procedure

Fixtures: ADR-021 §7.2's subject with partial-order reduction selected.

1. Evaluate `bump(a)`'s post clause over an observation restricted to a footprint without `Field{a, v}`, then with the derived footprint.
2. Run the reduced check and record each step's frame check; check a hand-built successor of `bump(a)` that also changes `b.v`; derive footprints twice.

Tag the tests `#[trace("TC-575", "FR-155-AC-n")]`.

## Expected Results

- Step 1: the expansion stops `runtime_invariant`/`established-invariant-broken` naming `bump(a)` and `Field{a, v}` and the item settles `failed`, internal failure; then a value.
- Step 2: every step passes; `frame_violation`; equal footprints.
