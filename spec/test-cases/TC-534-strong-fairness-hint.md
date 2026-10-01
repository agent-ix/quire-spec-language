---
id: TC-534
title: "A refuted liveness record names the strong constraint that would exclude its lasso"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-133
    type: verifies
---
# TC-534: A refuted liveness record names the strong constraint that would exclude its lasso

## Description

Verify the `fairness.strong-would-exclude` hint: its suggested constraint,
identity and position; its absence when the clause already carries the
constraint, for non-liveness items and for a stutter loop; and that it
leaves the verdict, counterexample and identities unchanged.

Scope: FR-133-AC-1 to FR-133-AC-3.

## Test Procedure

1. Settle the mutex refutations under `weak each` and under `strong`
   (whole) on `acquire`; compare each record's verdict, counterexample and
   obligation identity with a settlement that skips the hint computation.
2. Settle ADR-018 §6's ConfigVersion refutation under weak `attemptUpdate`
   with no granularity.
3. Settle FR-126-AC-3's stutter refutation and deadlock-freedom refutation,
   and FR-126-AC-2's bounded refutation.

Tag the tests `#[trace("TC-534", "FR-133-AC-n")]`.

## Expected Results

- Step 1: each record carries one hint `{Strong, acquire, Each}`, identity
  `acq(1)`, position 0, warning severity; verdict `refuted`, counterexample
  and obligation identity equal to the settlement without the hint.
- Step 2: one hint `{Strong, attemptUpdate, Each}`, identity `upd(b)`,
  position 0.
- Step 3: no hint on any record.
