---
id: TC-234
title: "Connection admission checks all three conditions independently and reports every failure"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-086
    type: verifies
---
# TC-234: Connection admission checks all three conditions independently and reports every failure

## Description

Verify that `check_connection` reports both the port-direction failure and
the per-end multiplicity failure when both fail simultaneously, correctly
reports the interface-type condition as passing, and admits a Connection
whose three conditions all hold. Scope: FR-086-AC-2.

Catches an implementation that stops at the first failing condition (the
sequential-check shape) instead of accumulating all three independently —
undetectable by any test that violates only one condition at a time, since a
stop-at-first implementation and an exhaustive one report the same single
failure in that case.

## Test Procedure

1. Declare two Port-classified endpoints whose declared directions are
   mutually incompatible for the relationship's declared direction (for
   example, both declared `In`, for a `SourceToTarget` relationship, which
   the direction rule requires the source to be `Out`/`InOut`).
2. Give the relationship's source end a multiplicity that does not conform
   to its port's declared multiplicity, while the two ports' interface
   types are declared to conform to each other correctly.
3. Classify and admit this relationship as a Connection candidate, then run
   `check_connection` over it.
4. Separately construct a Connection whose three conditions all hold, and
   run `check_connection` over it.

## Expected Results

Step 3's outcome reports two failures — `port-direction` and
`multiplicity` — and does not report `interface-type` as failing. Step 4
admits. A mutant that returns after the first failing condition reports only
`port-direction` in step 3, failing the two-failures assertion.
