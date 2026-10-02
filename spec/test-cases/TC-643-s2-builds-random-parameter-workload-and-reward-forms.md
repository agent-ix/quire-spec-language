---
id: TC-643
title: "S2 builds random-parameter, workload and reward forms"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-274
    type: verifies
---
# TC-643: S2 builds random-parameter, workload and reward forms

## Description

Verify that S1 parses and S2 builds the random-parameter, reward and workload forms, the `workload` dispatch entry, and the one S2 refusal.

Scope: FR-274-AC-1 to FR-274-AC-3.

## Test Procedure

Fixtures: ADR-024 §7.2's `Service` unit; a unit with an empty support list; a unit with a zero weight; a unit whose workload omits `Server::reset`.

1. Build `Service` and read back each form, its order and its spans.
2. Dispatch a unit beginning `workload`.
3. Build the empty-support, zero-weight and incomplete-workload units.

Tag the tests `#[trace("TC-643", "FR-274-AC-n")]`.

## Expected Results

- Step 1: the forms, pairs and weights of FR-274-AC-1, each with its source span.
- Step 2: exactly one call, to `probabilistic::workload`.
- Step 3: `EmptySupport` at the braces; the other two build.
