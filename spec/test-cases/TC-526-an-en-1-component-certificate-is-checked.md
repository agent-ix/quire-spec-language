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

Scope: FR-339-AC-1 to FR-339-AC-3 and FR-339-AC-5.

## Test Procedure

1. Check the certificate of FR-126-AC-1's weak `each` proof.
2. Check it with the witness of `{(*, 0, q1)}` naming `upd(a)`; with the
   two accepting components listed first; with one state in two
   components; with a `Trivial` witness over a state with a self-edge.
3. Check FR-339-AC-3's `UnfairVisibility` certificate for the synthesized SCC with `r` the only accepting state, then with the witness `UnfairWeak`, then with `q → r` not removed.
4. Over FR-339-AC-5's test-built checked package (the guarded
   `temporal_clause` node in QSpec FR-370's shape, the condition read by the
   test's own letters), check the certificate of the guarded TP-4 claim
   `eventually holds(c.versionNumber = 2)` `on each` when
   `c.versionNumber = 1` under a weak `each` constraint; then with the
   condition `c.versionNumber = 7` and no fairness; then the first
   certificate with the waiting component's witness `UnfairWeak` naming
   `upd(a)`, with the active component's witness `MissingAcceptance{set: 0}`,
   with binding `a`'s active `(1,0)` state removed, and with the condition
   undefined at version 1.

Tag the tests `#[trace("TC-526", "FR-339-AC-n")]`.

## Expected Results

- Step 1: accepted.
- Step 2: `WitnessFails` at `{(*, 0, q1)}`'s first state; `BackwardEdge`;
  `NotPartition`; `WitnessFails`.
- Step 3: accepted although `q → r` runs backward; `WitnessFails`; `BackwardEdge`.
- Step 4: accepted, settling `proved`; accepted; `WitnessFails`;
  `WitnessFails` at the active component's first state; `SuccessorMissing`
  at the removed state; `BadState` at binding `a`'s waiting `(0,0)` state.
