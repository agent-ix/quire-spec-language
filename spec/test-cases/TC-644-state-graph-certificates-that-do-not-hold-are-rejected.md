---
id: TC-644
title: "State-graph certificates that do not hold are rejected and never prove"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: verifies
---
# TC-644: State-graph certificates that do not hold are rejected and never prove

## Description

Verify each rejection rule of `check_state_graph`: a missing successor, a
broken or missing rank, an undefined predicate, an undefined contract
conjunction, a broken count and a broken order, each settling `CertificateRejected{rule, at}`, and a check that a
limit stops.

Scope: FR-169-AC-10, FR-169-AC-11.

## Test Procedure

Fixtures: TC-643's `ReachesTwo` and sequenced `InOneWay` certificates;
FR-168-AC-5's item; §7.3's unsequenced job; FR-167-AC-2's guarded subject.

1. Check `ReachesTwo`'s certificate with `(2, 2)` removed, with the rank of
   `(1, 0)` set to 2, and with `(0, 0)` unranked.
2. Check a hand-built certificate for FR-168-AC-5's item that lists all 9
   states and ranks every state with `va <= 1`.
3. Check an `InOneWay` certificate over the unsequenced job with every
   count 1, and the sequenced certificate with its order reversed.
4. Check `ReachesTwo`'s certificate with `max_states` 2.
5. Check a hand-built certificate of FR-167-AC-2's 3 states, ranked 2, 1
   and 0 for `ReachesTwo`, over the guarded subject, and over the same
   subject without the guard.
6. Settle each step's result.

Tag the tests `#[trace("TC-644", "FR-169-AC-10")]` or
`#[trace("TC-644", "FR-169-AC-11")]`.

## Expected Results

- Step 1: `SuccessorMissing` at the first, in certificate order, of
  `(1, 2)` and `(2, 1)`; `RankBroken` at `(0, 0)`; `RankMissing` at
  `(0, 0)`.
- Step 2: `PredicateUndefined` at the first, in certificate order, of the
  states where `va = 2`.
- Step 3: `CountBroken` at the node with two paths; `OrderBroken` at the
  first, in certificate order, of the nodes other than the `to` node.
- Step 4: stopped, naming `max_states` and 2.
- Step 5: over the guarded subject, `BadState` at the state where
  `va = 2`; over the unguarded subject, accepted.
- Step 6: each rejection settles `inconclusive`,
  `CertificateRejected{rule, at}`, never `proved`; step 4 settles
  `failed`, naming `max_states`.
