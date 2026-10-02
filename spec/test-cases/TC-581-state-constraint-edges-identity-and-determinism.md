---
id: TC-581
title: "Undefined, absent and initially-false constraints, bounds, identity and determinism"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-158
    type: verifies
---
# TC-581: Undefined, absent and initially-false constraints, bounds, identity and determinism

## Description

Verify the constraint's edge cases, that it is no bound, that it stays out of the identity and is named in the method, and determinism.

Scope: FR-158-AC-3 to FR-158-AC-4.

## Test Procedure

Fixtures: ADR-021 §7.3's subject; a constraint whose body is undefined at one state.

1. Run with the undefined-at-one-state constraint, with an absent constraint name, with a constraint false at the initial state, and with no universe for `counters`.
2. Compare the obligation and request identities of the constrained and unconstrained requests; read the method; repeat TC-580 step 1.

Tag the tests `#[trace("TC-581", "FR-158-AC-n")]`.

## Expected Results

- Step 1: the item returns `Violated` with cause `UndefinedEvaluation` and a counterexample ending at that state, which replays, and that state is not a boundary state; `invalid_runtime_input`/`invalid-value` naming the constraint with no state expanded; one stored state; `RequiresBound`.
- Step 2: equal obligation identities and different request identities, so neither result settles the other; `Low` in the method; equal outcomes and boundary counts.
