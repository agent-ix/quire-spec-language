---
id: SR-917
title: "QSL-345 spec review of PR 545, items 1 and 2 (FR-121 package bytes and re-exports)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@17895e2ca8666075a7b8d9ec9884fcc1e089f48a; spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md (re-export paragraph, Outputs, AC-14, AC-15); spec/test-cases/TC-516-locate-a-function-call-site.md (summary, steps 14-15); spec/spec.md (FR-121 row); spec/tests.md (TC-516 row). ADR-012 §15.4 amendment and FR-121 AC-12/AC-13/field Behavior excluded (item 4 leaves this PR)."
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: reviews
---
## Summary

Ticket: QSL-345 (PR 1 of 2). PR: quire-spec-language#545 at 17895e2c.

FR-121 Outputs now say `package` is the `quire.checked-package/v2` bytes,
"exactly as the S4 emitter wrote them when it minted `package_id`". That matches
the code, and AC-14 can be tested and is tested. The re-export paragraph lists exactly
the eight types lib.rs re-exports. AC-15 is testable. TC-516 steps 14 and 15 point at
the right fixtures: the step 2 unit `f` and the step 1 unit's `x`. The spec states
what is, without naming alternatives.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean for items 1 and 2. Removing item 4 leaves item-4 text in these places, all of
which must go: `FieldName` in the FR-121 re-export sentence, the Inputs "four
implementors" sentence, the Outputs `FieldSite` bullet, the field Behavior bullet,
the ADR-012 Dependencies entry, the spec.md FR-121 row's "state field's ADR-012
§15.4 domain key" clause, the TC-516 summary's first clause, and the ADR-012
amendment line and §15.4 bullet.
