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

1. Check `always eventually holds(c.versionNumber = 2)` with a weak
   constraint on `attemptUpdate` written with no granularity, with `whole`
   and with `each`; compare fairness sets and node identities.
2. Check a weak constraint on `Absent`; a fairness constraint on a clause
   under the event-position false-extension profile; one constraint written
   twice.
3. Under infinite-trace, check the recovery-stability formula over
   `Health`, `eventually[3,*] holds(p)` and `eventually[5,3] holds(p)`;
   check `eventually[3,*] holds(p)` under event-position; compare the
   `[0,2]` interval's key with `[0,2]` under event-position.
4. Check the five clauses of FR-123-AC-4 and read their property forms and
   requirement records; check a clause over the ConfigVersion subject under
   the fixed-sample profile.

Tag the tests `#[trace("TC-518", "FR-123-AC-n")]`.

## Expected Results

- Step 1: the first two fairness sets are equal, granularity `Whole`; the
  third is `Each`; the clause node identities differ between `Whole` and
  `Each`.
- Step 2: `missing_declaration`/`missing-name` at `Absent`;
  `unsupported_construct`/`expression-form` at the constraint; a one-member
  fairness set.
- Step 3: the formula checks with `Some([0,2])` on the inner `always` and
  `None` on the outer operators; `eventually[3,*]` checks with
  `Some([3,Open])`; `eventually[5,3]` refuses at its span; `eventually[3,*]`
  under event-position refuses at its span; the two `[0,2]` keys differ.
- Step 4: `ReachableInvariant`, `BoundedMltl`, `Safety`, `Liveness`,
  `Liveness`; each infinite-trace clause records (`temporal-satisfaction`,
  `Unbounded`); the fixed-sample clause has no property form and keeps its
  profile in its requirement record.
