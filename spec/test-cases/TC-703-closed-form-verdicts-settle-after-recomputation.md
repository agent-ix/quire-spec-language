---
id: TC-703
title: "Closed-form verdicts settle only after check_closed_form recomputes their evidence"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-248
    type: verifies
---
# TC-703: Closed-form verdicts settle only after check_closed_form recomputes their evidence

## Description

Verify the closed-form verdict map and that tampered evidence settles inconclusive.

Scope: FR-248-AC-1 to FR-248-AC-3.

## Test Procedure

Fixtures: the outcomes of TC-702 and their tampered copies.

1. Settle `Ctl` and its WCET-6 variant under both policies.
2. Settle the outcome with fixpoint 11 and the miss with demand 7 at point 8.
3. Settle `Mc` and its `C(HI) = 6` variant.

Tag the tests `#[trace("TC-703", "FR-248-AC-n")]`.

## Expected Results

- Step 1: `Proved{ClosedForm{FixedPriorityRta}}` with the fixpoints, `Proved{ClosedForm{EdfQpa}}`; `refuted` with evidence for both.
- Step 2: `inconclusive`, `ReplayParity`, both.
- Step 3: `Proved{ClosedForm{AmcRtb}}`; `inconclusive`, `SufficientTestFailed`.
