---
id: TC-525
title: "An EN-1 closure certificate is accepted or rejected by the core checker"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: verifies
---
# TC-525: An EN-1 closure certificate is accepted or rejected by the core checker

## Description

Verify that `check_closure` accepts a safety proof's closure and rejects a
missing initial state, a missing successor and a bad state.

Scope: FR-338-AC-1 to FR-338-AC-3.

## Test Procedure

1. Check the certificate of FR-126-AC-2's TP-1 proof over ADR-018 §6's
   subject.
2. Check it with the product state for model state `(1, 0)` removed; with
   the initial product state removed.
3. Check a certificate holding every reachable `Counter` state for the
   deadlock-freedom item, with no `terminal` member; then the same states
   for `always holds(c.value <= 3)`.
4. Check a certificate holding `t1` for `always holds(true)` over
   `test/tallies`.

Tag the tests `#[trace("TC-525", "FR-338-AC-n")]`.

## Expected Results

- Step 1: accepted.
- Step 2: `SuccessorMissing` at `(1, 0)`; `InitialMissing` at the initial
  state.
- Step 3: `BadState` at value 3; accepted.
- Step 4: `BadState` at `t1`.

