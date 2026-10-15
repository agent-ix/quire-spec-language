---
id: SR-11186
title: "gap analysis of quire-spec-language PR 861 (same-identity collisions refuse conflicting-binding)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ea401b9cb7d94ff86c5ba9bf20dbeacbb40ef75a; FR-056-AC-26, FR-056-AC-30, TC-996, TC-1003, QSpec FR-154 (row: two nodes share one identity), QSpec FR-154-AC-12, QSpec TC-235; qsl-semantics/src/model/intake.rs; qsl-semantics/tests/it/tc757_same_identity_collisions.rs, variant_type.rs"
review_set: subset
---

## Summary

Ticket: QSL-757. Planless gap analysis. Plan completion: not assessed.

- Key choice. The PR keys fields, operations, parameters and cases on the
  shared identity. That is how QSpec FR-154's shared-identity row keys its
  member cases (only a step names its process), so the key is right. The row
  is not scoped to one owner, so a cross-owner duplicate refusing
  `conflicting-binding` is correct too (compare QSpec FR-154-AC-10).
- FR-056-AC-26 and TC-996 step 4 are met by
  `two_cases_sharing_one_identity_are_refused_by_the_reader`, which asserts the
  full cause. The variant half of FR-056-AC-30 and TC-1003 step 5 is met by
  `two_cases_sharing_an_identity_are_refused_at_the_second_listed`, in both
  orders.
- The QSpec gap is routed by the lead and not counted here. The QSpec FR-154
  row and TC-235 do not list these collision kinds, and one QSpec TC-235 row
  for a same-identity case still states the old outcome. The PR body records
  only the TC-235 half of this gap.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Wrong trace, and the amended AC has no trace for its new obligation. The four tc757 collision tests are tagged `QSpec-FR-154-AC-12`, but that AC does not state field, operation or parameter collisions. Those are the routed QSpec gap. The PR adds exactly that obligation to QSL FR-056-AC-30 ("Two fields, two operations, a field and an operation, or two parameters of one operation that share one identity refuse the same cause..."), but no test carries `FR-056-AC-30` for it. AC-30's Verification cell also says `Test (TC-1003)`, and TC-1003 has no step for those collisions. Fix: tag the tc757 tests `FR-056-AC-30`, drop the AC-12 claim or keep it only as context, and add the field/operation/parameter step to TC-1003, or point AC-30 at a TC that has one. | qsl-semantics/tests/it/tc757_same_identity_collisions.rs:38,55,72,89,104; spec/functional/FR-056-admit-domain-package-model-declarations.md:800; spec/test-cases/TC-1003-variant-type-effective-identity.md |

## Verdict

CHANGES REQUESTED (one medium trace finding). The behaviour matches FR-154. Each
collision kind has a vector that fails on the pre-change code. The shared-
identity key is the right reading of the spec.

## Dispositions

Round 1, reviewed at bd32608341bda0788418f2cdde5a90371a5b7310.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | bd3260834. The tc757 tests now carry `FR-056-AC-30`, `TC-1003` and `QSpec-FR-154-AC-13`, and the AC-12 claim is gone. TC-1003 gains step 6 (field, operation and parameter collisions) and step 7 (distinct identities, and one member name under two owners admitted), and each step has a test. |
