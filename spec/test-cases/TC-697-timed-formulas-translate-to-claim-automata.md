---
id: TC-697
title: "Timed formulas translate to claim automata that agree with the evaluator"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-242
    type: verifies
---
# TC-697: Timed formulas translate to claim automata that agree with the evaluator

## Description

Verify that MITL and bounded translations accept exactly the timed traces on which the evaluator returns false, that punctual bounded intervals are decided, and that TT-1 builds no automaton.

Scope: FR-242-AC-1 to FR-242-AC-3.

## Test Procedure

Fixtures: a seeded generator of timed lassos with stamps in steps of `1/4` and length at most 8; the two punctual models of FR-242-AC-2; ADR-026 §11's `Rpc` unit with `T = 3 ms` and `T = 4 ms`, universe `{c}`.

1. For each of the four formulas of FR-242-AC-1, compare automaton acceptance with evaluator values on 1,000 generated lassos.
2. Check `eventually[2 ms, 2 ms] holds(q)` over the two punctual models.
3. Check `always holds(not c.late)` under the timed profile over `Rpc`.

Tag the tests `#[trace("TC-697", "FR-242-AC-n")]`.

## Expected Results

- Step 1: acceptance equals evaluator `false` on every lasso.
- Step 2: `Holds`; `Violated`.
- Step 3: no automaton built, zero automaton states.
