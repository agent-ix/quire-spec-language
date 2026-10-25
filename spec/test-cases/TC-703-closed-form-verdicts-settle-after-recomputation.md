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

Scope: FR-248-AC-1 to FR-248-AC-5.

## Test Procedure

Fixtures: the outcomes of TC-702 and their tampered copies. Carry each outcome
in a `ClosedFormResult` whose `originating_obligation` is the identity of the
obligation analyzed to produce it.

1. Settle `Ctl` and its WCET-6 variant under both policies.
2. Settle the outcome with fixpoint 11 and the miss with its last iterate 13 replaced by 12; settle the two-task set of FR-248-AC-2 with evidence naming job 2 and with evidence naming job 0.
3. Settle `Mc` and its `C(HI) = 6` variant.
4. Check `CtlFp`'s `ClosedFormResult` against `CtlEdf`'s item and claim triple, then check that same result against the original request with its source bytes edited to raise a WCET and its expected package identity unchanged.
5. Check `CtlFp`'s `ClosedFormResult` with `claim` its own FR-454 triple, with an unknown NodeId, with another ordinal, with a `claim`/`state_graph` node's triple, and with `CtlEdf`'s triple; settle each.

Tag the tests `#[trace("TC-703", "FR-248-AC-n")]`.

## Expected Results

- Step 1: `Proved{ClosedForm{FixedPriorityRta}, Certified}` with the fixpoints, `Proved{ClosedForm{EdfQpa}, Certified}`; `refuted` with evidence for both.
- Step 2: `inconclusive`, `ReplayParity`, both. The job-2 evidence: `inconclusive`, `ReplayParity`, naming job 1; the job-0 evidence: `refuted`.
- Step 3: `Proved{ClosedForm{AmcRtb}, Certified}`; `inconclusive`, `SufficientTestFailed`.
- Step 4: `content-mismatch` naming the originating `CtlFp` identity and the expected `CtlEdf` identity; FR-098's package-identity refusal after the source edit.
- Step 5: AC-1's settlement with `Ctl` read from the recompiled package; `clause-root-unresolved` and the item `failed`, `Failed`, with no evidence checked, for each of the three bad triples; `content-mismatch` naming the identities of `CtlFp` and `CtlEdf`.
