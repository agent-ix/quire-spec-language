---
id: TC-530
title: "S3 checks strong fairness constraints and the unmarked fairness kind"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-129
    type: verifies
---
# TC-530: S3 checks strong fairness constraints and the unmarked fairness kind

## Description

Verify that S3 reads a constraint with no kind as weak, checks the strong
kind with FR-123's granularity rule, keeps weak and strong constraints on
one operation apart, puts the resolved kind into the clause's identity, and
refuses a strong constraint under a bounded profile.

Scope: FR-129-AC-1, FR-129-AC-2.

## Test Procedure

Use ADR-019 §6's mutex unit and `always eventually holds(m.owner = 1)`.

1. Check the clause with a constraint on `acquire` written with no kind and
   no granularity, with `weak whole`, with `strong`, with `strong each` and
   with `weak each`; compare the checked constraints and node identities.
2. Check the clause with `strong each` and `weak each` on `acquire`; with
   `strong` written twice; and a `strong` constraint on a clause under the
   event-position false-extension profile.

Tag the tests `#[trace("TC-530", "FR-129-AC-n")]`.

## Expected Results

- Step 1: `{Weak, acquire, Whole}` with the same node identity as the
  `weak whole` clause; `{Strong, acquire, Whole}`; `{Strong, acquire,
  Each}`; the `strong each` and `weak each` identities differ.
- Step 2: a two-member set in source order; a one-member set;
  `unsupported_construct`/`expression-form` at the constraint's span.
