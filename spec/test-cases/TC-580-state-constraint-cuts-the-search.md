---
id: TC-580
title: "A state constraint stores boundary states, reports real violations and never proves"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-158
    type: verifies
---
# TC-580: A state constraint stores boundary states, reports real violations and never proves

## Description

Verify ADR-021 §7.3: the stored and expanded counts, the boundary violation, and `ConstraintReached` for safety, liveness and deadlock-freedom.

Scope: FR-158-AC-1 to FR-158-AC-2.

## Test Procedure

Fixtures: ADR-021 §7.3's subject, constraint `Low`, instance `x = a`.

1. Run `always holds(x.v <= 1)` constrained and unconstrained.
2. Run `always holds(x.v <= 2)`, `eventually holds(x.v = 3)` and the deadlock-freedom item, constrained and unconstrained.

Tag the tests `#[trace("TC-580", "FR-158-AC-n")]`.

## Expected Results

- Step 1: 20 stored, 8 expanded; `Violated` at `(2,0,0)` with prefix `bump(a), bump(a)` in both runs.
- Step 2: constrained, `ConstraintReached{12}` for each; unconstrained, `Violated` at `(3,0,0)`, `Holds`, and `Violated` at `(3,3,3)`.
