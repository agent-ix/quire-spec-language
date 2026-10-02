---
id: TC-576
title: "Ample sets reduce ADR-021 §7.2 to one interleaving and keep the deadlock"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-156
    type: verifies
---
# TC-576: Ample sets reduce ADR-021 §7.2 to one interleaving and keep the deadlock

## Description

Verify visibility, the ample set at each state, the counts and verdicts of ADR-021 §7.2, and the deadlock found by the reduced search.

Scope: FR-156-AC-1 to FR-156-AC-2.

## Test Procedure

Fixtures: ADR-021 §7.2's subject, instance `x = a`, empty fairness set.

1. Run `ReachesTwo` with partial-order reduction, recording each state's ample set; run it unreduced.
2. Run the deadlock-freedom item with partial-order reduction.

Tag the tests `#[trace("TC-576", "FR-156-AC-n")]`.

## Expected Results

- Step 1: the ample sets of FR-156-AC-1; 7 model states, 6 product states, `Holds`; unreduced 27 model states, `Holds`.
- Step 2: `Violated`, `kind: Deadlock`, the six-step prefix of the table.
