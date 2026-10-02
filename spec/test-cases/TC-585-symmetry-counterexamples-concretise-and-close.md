---
id: TC-585
title: "Symmetry counterexamples concretise to subject traces, closing a loop in one or more passes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-161
    type: verifies
---
# TC-585: Symmetry counterexamples concretise to subject traces, closing a loop in one or more passes

## Description

Verify concretisation of ADR-021 §7.1's lasso and of a quotient loop that closes only after two passes.

Scope: FR-161-AC-1 to FR-161-AC-2.

## Test Procedure

Fixtures: ADR-021 §7.1's annotated subject; the `Token` unit of FR-161-AC-2.

1. Concretise the §7.1 `fair weak whole` lasso and replay it.
2. Run the `Token` claim with symmetry, concretise its lasso and replay it.

Tag the tests `#[trace("TC-585", "FR-161-AC-n")]`.

## Expected Results

- Step 1: `upd(b)` three times through `(0,1,0)`, `(0,2,0)`, `(0,0,0)`, one pass; `reproduced-with-evaluated-witness`.
- Step 2: empty stem, loop `pass(b)`, `pass(a)`; it replays.
