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

Fixtures: a braking model with `v` of rate `-a` and a clock `t`; a fixture provider returning each `SolverOutcome`, and one that overruns its `deadline_ms`, which the host ends at `deadline_ms` and settles as `TimedOut`.

1. Classify the variables and route the claim, with the fixture provider registered and with none.
2. Map each fixture result.
3. Admit requests with `max_solver_time` 1.5 s, 2 s, 0.2 s and unset, and with `max_solver_steps` unset and 1,000; stop the fixture provider on time with no `Cancel` raised, again with a provider `deadline_ms` overrun, and again with an FR-276 frontend deadline that fires; stop it on steps; write 1.5 as a wire `max_solver_time`, a zero budget, an unknown `hybrid.limits` member, an empty `hybrid`, and a `hybrid` with an empty `limits`; write a request that sets no budget; compare the canonical key bytes of two requests that differ only in `max_solver_time`.

Tag the tests `#[trace("TC-705", "FR-250-AC-n")]`.

## Expected Results

- Step 1: `v` hybrid, `t` a clock; routed to the fixture; `unsupported`, `unsupported-requested-capability` with none.
- Step 2: the rows of FR-250's table, with no `refuted`.
- Step 3: solver budgets 2, 2, 1 and 60 s, the run using the rounded value; the time stop and the `deadline_ms` overrun are `incomplete` with cause `TimedOut`, the frontend deadline stop `Cancelled{source: deadline}`, the wire 1.5, the zero budget, the unknown member, the empty `hybrid` and the empty `limits` are each refused by the provider before consumption, the unset budgets are omitted from the request, and the stored record holds no wall-time member; the key bytes are equal; the step stop is `LimitReached{max_solver_steps, 1000, hybrid.max_solver_steps}`, the default 1,000,000.
