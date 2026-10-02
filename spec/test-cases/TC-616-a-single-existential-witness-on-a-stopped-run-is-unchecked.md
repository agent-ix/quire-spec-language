---
id: TC-616
title: "A single-existential witness on a stopped run settles well-definedness unchecked"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-181
    type: verifies
---
# TC-616: A single-existential witness on a stopped run settles well-definedness unchecked

## Description

Verify that when a limit stops the product exploration after phase 0 found
a witness lasso for every initial state, the HP-5 item settles
`inconclusive`, `WellDefinednessUnchecked`, never `proved`, and that the
same run with sampling off settles as a stopped run.

Scope: FR-181-AC-6.

## Test Procedure

Fixture: ADR-023 §8's secure vault and
`exists trace b of V { eventually always holds(v.l @ b = 1) }`.

1. Run the item with the default `witness_samples` and `max_states` 2, and
   settle it.
2. Run the item with `witness_samples` 0 and `max_states` 2, and settle it.

Tag the tests `#[trace("TC-616", "FR-181-AC-6")]`.

## Expected Results

- Step 1: a `Sampled` witness lasso per initial state; `WitnessedUnchecked`
  with `end` `Stopped(ResourceExhausted, MaxStates)`; `inconclusive`,
  `unsettled`, `WellDefinednessUnchecked`, naming `max_states`; not
  `proved`.
- Step 2: `NoDecision`; `incomplete`, `LimitReached`, naming
  `max_states`.
