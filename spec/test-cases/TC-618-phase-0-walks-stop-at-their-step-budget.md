---
id: TC-618
title: "Phase 0 walks stop at their step budget, and the horizon is not a limit"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-166
    type: verifies
---
# TC-618: Phase 0 walks stop at their step budget, and the horizon is not a limit

## Description

Verify that every phase 0 walk ends within `max_walk_steps`, that a walk the
budget stops decides nothing and is counted in the terminal record with the
budget's name and value, and that the search horizon bounds walks without
being reported as a limit.

Scope: FR-166-AC-7.

## Test Procedure

Fixture: ADR-022 §7.1's subject.

1. Run `ReachesThree` with the default limits and record each walk's length
   and the terminal record.
2. Run `ReachesTwo` with `max_walk_steps` 1 and `witness_samples` 64.
3. Run `ReachesTwo` with horizon `max_depth` 1.

Tag the tests `#[trace("TC-618", "FR-166-AC-7")]`.

## Expected Results

- Step 1: 64 walks per instance, each of 4,096 steps, no witness; the record
  names `max_walk_steps`, 4,096 and a count of 64 per instance.
- Step 2: no sampled witness; every witness is `Explored`.
- Step 3: every walk takes at most 1 step; the record states horizon 1 as a
  method parameter and counts no walk stopped by a limit.
