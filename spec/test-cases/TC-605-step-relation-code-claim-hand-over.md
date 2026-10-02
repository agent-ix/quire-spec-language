---
id: TC-605
title: "A bound step relation hands over its code claim, and its Kani counterexample replays"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-180
    type: verifies
---
# TC-605: A bound step relation hands over its code claim, and its Kani counterexample replays

## Description

Verify that a relation over bound operations records a separate `operation-contract` code claim with the checked-package members the harness reads, and that QSL's half of a Kani counterexample replay reproduces, disagrees and refuses.

Scope: FR-180-AC-1 to FR-180-AC-3.

## Test Procedure

Fixtures: `Det` over a vault whose `step` is bound to an implementation, and over one with no binding; the vault variant with invariant `self.l <= self.h`; hand-built code-claim counterexamples.

1. Check and emit `Det` over the bound and the unbound vault.
2. Replay the counterexample with unequal post-states, and the one with equal post-states.
3. Replay the counterexample with pre-state `(0, 1)` over the invariant variant, and one whose pre-state fails admission.

Tag the tests `#[trace("TC-605", "FR-180-AC-n")]`.

## Expected Results

- Step 1: two requirement records with distinct identities and the relation node's members; the model claim alone.
- Step 2: `reproduced-with-evaluated-witness`; `inconclusive`, `ReplayParity`.
- Step 3: `invalid_runtime_input`/`invalid-value` naming the execution and invariant; FR-106's record.
