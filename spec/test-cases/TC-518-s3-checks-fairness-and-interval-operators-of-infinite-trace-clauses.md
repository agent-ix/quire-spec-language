---
id: TC-518
title: "S3 checks fairness constraints and interval operators of infinite-trace clauses"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: verifies
---
# TC-518: S3 checks fairness constraints and interval operators of infinite-trace clauses

## Description

Verify that the S3 TemporalTrace check reads unmarked fairness as `whole`,
refuses an unresolved or misplaced constraint, admits interval operators
nested under unbounded ones under infinite-trace, and records each clause's
property form.

Scope: FR-123-AC-1 to FR-123-AC-4.

## Test Procedure

Use ADR-018 §6's ConfigVersion example unit (post clause `Cycles`) and the
`Health` unit of FR-126-AC-4.

1. Check `always eventually holds(c.versionNumber = 2)` with `fair weak
   attemptUpdate`, `fair weak whole attemptUpdate` and `fair weak each
   attemptUpdate`; compare fairness sets and node identities.
2. Check `fair weak Absent`; a fairness constraint on a clause under the
   event-position false-extension profile; `fair weak attemptUpdate`
   written twice.
3. Under infinite-trace, check the recovery-stability formula over
   `Health`, `eventually[3,*] holds(p)` and `eventually[5,3] holds(p)`;
   compare the `[0,2]` interval's key with `[0,2]` under event-position.
4. Check the five clauses of FR-123-AC-4 and read their property forms and
   requirement records.

Tag the tests `#[trace("TC-518", "FR-123-AC-n")]`.

## Expected Results

- Step 1: the first two fairness sets are equal, granularity `Whole`; the
  third is `Each`; the clause node identities differ between `Whole` and
  `Each`.
- Step 2: `missing_declaration`/`missing-name` at `Absent`;
  `unsupported_construct`/`expression-form` at the constraint; a one-member
  fairness set.
- Step 3: the formula checks with `Some([0,2])` on the inner `always` and
  `None` on the outer operators; both malformed intervals refuse at their
  spans; the two `[0,2]` keys differ.
- Step 4: `ReachableInvariant`, `BoundedMltl`, `Safety`, `Liveness`,
  `Liveness`; each infinite-trace clause records (`temporal-satisfaction`,
  `Unbounded`).
