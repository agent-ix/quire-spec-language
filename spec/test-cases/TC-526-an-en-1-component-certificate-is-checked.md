---
id: TC-526
title: "An EN-1 component certificate is accepted or rejected by the core checker"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-339
    type: verifies
---
# TC-526: An EN-1 component certificate is accepted or rejected by the core checker

## Description

Verify that `check_components` accepts a weak-fairness liveness proof's
partition and rejects a failing witness, a backward edge and a
non-partition.

Scope: FR-339-AC-1 and FR-339-AC-2.

## Test Procedure

1. Check the certificate of FR-126-AC-1's weak `each` proof.
2. Check it with the witness of `{(*, 0, q1)}` naming `upd(a)`; with the
   two accepting components listed first; with one state in two
   components; with a `Trivial` witness over a state with a self-edge.

Tag the tests `#[trace("TC-526", "FR-339-AC-n")]`.

## Expected Results

- Step 1: accepted.
- Step 2: `WitnessFails` at `{(*, 0, q1)}`'s first state; `BackwardEdge`;
  `NotPartition`; `WitnessFails`.
