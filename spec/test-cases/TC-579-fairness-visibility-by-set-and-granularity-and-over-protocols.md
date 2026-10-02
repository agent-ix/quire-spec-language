---
id: TC-579
title: "Fairness visibility applies only under a non-empty fairness set, at either granularity, and over protocols"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-157
    type: verifies
---
# TC-579: Fairness visibility applies only under a non-empty fairness set, at either granularity, and over protocols

## Description

Verify the empty-set run on `Gate`, the `each` granularity, and ADR-021 §7.4 over `ProtocolSystem` under default and adversarial scheduling.

Scope: FR-157-AC-3 to FR-157-AC-4.

## Test Procedure

Fixtures: The `Gate` unit; ADR-021 §7.4's `Mark` protocol subject.

1. Run `Opens` with the empty fairness set, reduced and unreduced; replay both counterexamples.
2. Run `Opens` under `fair weak each go` with partial-order reduction.
3. Run §7.4's `Marked` with partial-order reduction under the default scheduler constraints and under `scheduling adversarial`, and unreduced.

Tag the tests `#[trace("TC-579", "FR-157-AC-n")]`.

## Expected Results

- Step 1: ample sets `{arm}` then `{arm2}`; `Violated` with stem `arm, arm2` and loop `toggle, toggle`; unreduced `Violated` with empty stem; both replay.
- Step 2: AC-1's visibility and verdict.
- Step 3: `fork`, `send(S)`, `attempt(D)` visible and `duplicate`, `lose`, `join`, `finish` invisible; `Holds`, settling `Proved{Reduced{[PartialOrder{BreadthFirstRevisit}]}}`; `Violated` with `fork`, `send(S)`, loop `duplicate`, `lose`; each equal to the unreduced verdict.
