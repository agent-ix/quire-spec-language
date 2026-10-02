---
id: TC-890
title: "S2 builds random, reward and workload declaration forms"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-311
    type: verifies
---
# TC-890: S2 builds random, reward and workload declaration forms

## Description

Verify that S2 builds the `random-decl`, `reward-decl` and `workload-decl` forms and dispatches each leading token to its one production.

Scope: FR-311-AC-1 to FR-311-AC-3.

## Test Procedure

Fixtures: ADR-024 §7.2's `Service` written in the shared grammar's productions; a unit with a zero weight; a unit whose workload omits `Server::reset`.

1. Build `Service` and read back each form, its order and its spans.
2. Dispatch units beginning `random`, `reward` and `workload`.
3. Build the zero-weight and incomplete-workload units.

Tag the tests `#[trace("TC-890", "FR-311-AC-n")]`.

## Expected Results

- Step 1: the forms, pairs and weights of FR-311-AC-1, each with its source span.
- Step 2: exactly one call each, to `random_decl`, `reward_decl` and `workload_decl`.
- Step 3: both build.
