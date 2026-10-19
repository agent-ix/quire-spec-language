---
id: TC-705
title: "Hybrid solver results map to proved or inconclusive, never refuted"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: verifies
---
# TC-705: Hybrid solver results map to proved or inconclusive, never refuted

## Description

Verify classification of clocks and hybrid variables, routing to an FR-193 provider, and the verdict map.

Scope: FR-250-AC-1 to FR-250-AC-3.

## Test Procedure

Fixtures: a braking model with `v` of rate `-a` and a clock `t`; a fixture provider returning each `SolverOutcome`.

1. Classify the variables and route the claim, with the fixture provider registered and with none.
2. Map each fixture result.
3. Admit requests with deadline durations of 1.5 s, 2 s and 0.2 s and with none, and with `max_solver_steps` unset and 1,000; stop the fixture provider on time and on steps.

Tag the tests `#[trace("TC-705", "FR-250-AC-n")]`.

## Expected Results

- Step 1: `v` hybrid, `t` a clock; routed to the fixture; `unsupported`, `unsupported-requested-capability` with none.
- Step 2: the rows of FR-250's table, with no `refuted`.
- Step 3: `max_solver_time` 2, 2, 1 and 60, the solver given the rounded budget; the time stop is `incomplete` with cause `TimedOut` and no wall-time member in the stored record; the step stop is `LimitReached{solver-steps, 1000, max_solver_steps}`, the default 1,000,000.
