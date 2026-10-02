---
id: TC-533
title: "A fairness constraint over a supplied trace settles as a missing premise"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-132
    type: verifies
---
# TC-533: A fairness constraint over a supplied trace settles as a missing premise

## Description

Verify that a clause with any fairness constraint over a supplied trace
settles `unsupported`, `MissingFairnessPremise`, naming the first
constraint, and that an empty fairness set evaluates normally.

Scope: FR-132-AC-1, FR-132-AC-2.

## Test Procedure

Supply the lasso `0 2 0` with steps `acq(2)`, `rel` over the mutex's states,
with no model subject.

1. Evaluate `always eventually holds(m.owner = 1)` with `weak each` on
   `acquire`; with `strong`; with `strong each` on `acquire` then `weak
   whole` on `release`.
2. Evaluate the clause with an empty fairness set.

Tag the tests `#[trace("TC-533", "FR-132-AC-n")]`.

## Expected Results

- Step 1: `Unsupported(MissingFairnessPremise)` naming `{Weak, acquire,
  Each}`, then `{Strong, acquire, Whole}`, then `{Strong, acquire, Each}`;
  label `unsupported`, basis `unavailable`, category unsupported.
- Step 2: `false`, a violation for that lasso.
