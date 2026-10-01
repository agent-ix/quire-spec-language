---
id: TC-614
title: "A witness on a stopped run settles inconclusive with well-definedness unchecked"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-166
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-168
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: verifies
---
# TC-614: A witness on a stopped run settles inconclusive with well-definedness unchecked

## Description

Verify that when a limit stops the exploration after phase 0 found a
witness for every initial state, the item settles `inconclusive` with cause
`WellDefinednessUnchecked`, never `proved`, and that the same run with
sampling off settles as a stopped run.

Scope: FR-166-AC-6, FR-168-AC-8, FR-169-AC-8.

## Test Procedure

Fixture: ADR-022 §7.1's subject and `ReachesTwo`.

1. Run the item with default `witness_samples` and `max_states` 2.
2. Settle step 1's outcome with its witnesses' replay results.
3. Run the item with `witness_samples` 0 and `max_states` 2, and settle it.

Tag the tests `#[trace("TC-614", "<AC id>")]`.

## Expected Results

- Step 1: a `Sampled` witness per instance; `WitnessedUnchecked` with
  `end` `Stopped(ResourceExhausted, MaxStates)`; not `Witnessed`.
- Step 2: `inconclusive`, `unsettled`,
  `Inconclusive(WellDefinednessUnchecked{sources})` with `Sampled` sources,
  naming `max_states` and its value; not `proved`.
- Step 3: `NoDecision` with the same `end`; `failed`, `resource-incomplete`,
  naming `max_states`.
