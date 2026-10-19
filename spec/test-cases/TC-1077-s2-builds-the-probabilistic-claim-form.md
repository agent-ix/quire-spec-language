---
id: TC-1077
title: "S2 builds the probabilistic claim form"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-392
    type: verifies
---
# TC-1077: S2 builds the probabilistic claim form

## Description

Verify that S2 builds the `probabilistic-clause` form for each of ADR-024 §7's claims, dispatches the leading token to its one production, and leaves every range, unit and completeness check to S3.

Scope: FR-392-AC-1 to FR-392-AC-3.

## Test Procedure

Fixtures: ADR-024 §7's models and claims `NoFault`, `P95`, `LongRun`, `Monthly` written in the shared grammar's productions; `NoFault` with only `alpha`; `P95` at `0.005 s`; a claim with threshold `1`; a claim over a trace subject; a claim `under every scheduler fair weak Node::tick`.

1. Build the four claims and read back each form, its scheduler, confidence items and spans.
2. Dispatch a unit beginning `probabilistic`; build the `every scheduler` claim.
3. Build the partial-confidence, `0.005 s`, threshold-`1` and trace-subject claims.

Tag the tests `#[trace("TC-1077", "FR-392-AC-n")]`.

## Expected Results

- Step 1: the bodies, schedulers and confidence items of FR-392-AC-1, each with its source span.
- Step 2: exactly one call, to `probabilistic_clause`; `Every` with one fairness item.
- Step 3: all four build.
