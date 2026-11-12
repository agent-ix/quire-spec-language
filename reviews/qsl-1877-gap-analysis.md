---
id: SR-2026111117463800000002
title: "gap analysis of PR 2681 typed FR-290 capability Cause"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; spec/functional/FR-057-admit-shared-capability-kinds.md, spec/functional/FR-075-compute-candidates-from-registered-backends.md, spec/test-cases/TC-155-keep-admission-backend-independent.md, spec/test-cases/TC-195-unregistered-named-backend-marker.md, qsl-route/src/cause.rs, qsl-route/tests/it/cause.rs"
review_set: subset
---

## Summary

Ticket: QSL-1877. Planless gap analysis of PR 2681 at 86b647a77 against main 91ada09a7, which holds the ticket's spec-reviewed criterion FR-057-AC-15 (#2679). Plan completion: not assessed. Checked whether the ticket's criterion has code and a tagged test, and whether the PR's new criterion is traced correctly.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-057-AC-15, the criterion #2679 wrote for this ticket, has no implementation and no tagged test after this PR. `NegotiationOutcome`, `NegotiationCause`, `NegotiationSettlement` and `qsl_route::settle_candidates` do not exist, and nothing carries `#[trace("TC-155", "FR-057-AC-15")]`. TC-155 step 8, including the named two-candidate `Inconsistent` case, is not exercised. The PR instead adds and traces its own FR-075-AC-23. | spec/functional/FR-057-admit-shared-capability-kinds.md:459; spec/test-cases/TC-155-keep-admission-backend-independent.md:52-57 |
| FND-002 | low | The new test is tagged `#[trace("TC-195", "FR-075-AC-23")]`, but TC-195's scope line names FR-075-AC-3 only, and TC-195 was not changed to cover AC-23. | qsl-route/tests/it/cause.rs:38; spec/test-cases/TC-195-unregistered-named-backend-marker.md:16 |

## Verdict

FAIL. The ticket's governing criterion (FR-057-AC-15) is unbacked. The PR's own FR-075-AC-23 is backed by one test for its per-outcome mapping. No test asserts its single-source clause, and that clause is false (code-review SR-2026111117463800000001 FND-002). Binding `qsl-route/tests/it/cause.rs::each_outcome_settles_its_typed_cause_and_catalog_code` to FR-075-AC-23 is correct for the mapping it asserts.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified at 80f74e3ec: qsl-route/tests/it/negotiation_cause.rs has two `#[trace("TC-155", "FR-057-AC-15")]` tests that cover every code, `outcome()`, and every settlement row, including two candidates with a named backend giving `Inconsistent`. |
| FND-002 | fixed | Verified at 80f74e3ec: the TC-195/FR-075-AC-23 test and AC-23 are removed. |
