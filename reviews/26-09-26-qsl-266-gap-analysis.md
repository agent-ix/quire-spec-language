---
id: SR-671
title: "QSL-266 gap analysis of per-occurrence requirement records"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ba8b2f82ae99bb898bb6b33d6b1dd9b8bdfb708d; spec/functional/FR-062-implement-checked-family-contract.md; spec/functional/FR-075-compute-candidates-from-registered-backends.md; spec/functional/FR-097-classify-claim-extent-and-write-bounded-requests.md; spec/test-cases/TC-160-checked-family-contract-shape.md; spec/test-cases/TC-440-qsl-extent-agrees-with-ir-requires-bound.md; spec/test-cases/TC-449-request-builder-writes-one-item-per-requirement-record.md; spec/tests.md; qsl-semantics/src/check/claims.rs; qsl-route/src/request.rs; qsl-package/src/emit/extent_agreement.rs; tests/it/request_builder.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: reviews
---

## Summary

Ticket: QSL-266 (PR quire-spec-language#459). There is no plan bundle
under `plan/` for QSL-266. The ticket and the TC steps it names are the
plan: TC-160 steps 5, 10 and 11, TC-440 step 4, and TC-449 steps 1-6.

Matrix: `quire coverage --scope . --json` at `ba8b2f82` reports no
unbacked row, status lie or no-symbol row for TC-160, TC-440, TC-449,
FR-062-AC-4, FR-062-AC-13, FR-075-AC-8 or FR-097-AC-6. The one unbacked
FR-062 row is FR-062-AC-10 (TC-166). It predates this PR and is out of
scope.

Step-by-step:

- TC-449 steps 1-6 each map to one test in `tests/it/request_builder.rs`,
  with the expected results the TC lists.
- TC-160 step 5 maps to
  `tc_160_the_requirements_function_yields_one_claim_per_scalar_application`.
- TC-160 step 10 is covered for RR-1 to RR-13 and RR-15 to RR-17, RR-15
  in both orders, RR-5 twice, and the S4 read through
  `tc_449_one_item_per_record_in_key_order`. RR-14 is excluded by leader
  decision (renamed `all_positive` and tested in the fix round).
- TC-440 step 4 is backed for the inner `+`. The outer `+` is an honest
  ignore naming IR-283: run with `--ignored`, it fails on `Lowered`.

Underspecified code: every new public item has an owning requirement.
`RequirementItem` and `items_from_requirements` belong to FR-075.
`ClaimSite`, `ValueClaim`, `RequirementRecord`, `ResultBound` and
`PathGuard` belong to FR-062 and ADR-012 §2/§13.5.
`KeyFault::UnclassifiedExtent` belongs to FR-062 and FR-097-AC-2.
`RequestItem::occurrence` belongs to FR-075's bounded follow-up rule.

Semantic review (intent↔test↔code) was done inline for the FR-062 claim
rules. Its correctness findings are in SR-670 (code-review).

## Verdict

**CONDITIONAL.** One medium finding: TC-160 step 11's text still
describes the deleted identity-based keying. The other two findings are
low status-text defects.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-160 step 11 and its expected result still describe the deleted identity-based `key_requirements` mechanism: two identities, an index whose identity is `None`, one identity shared by two indices. That mechanism and its tests were removed. The new tests tagged "TC-160 step 11" (`tc_160_an_unpaired_site_or_application_faults_instead_of_dropping`, `tc_160_two_sites_of_one_node_pair_by_location_not_order`) check location-based pairing of sites, `generated`-only occurrences and unclaimed scalar occurrences. The TC text no longer names what the tests verify, and no test can satisfy the `None`-identity clause as written. | spec/test-cases/TC-160-checked-family-contract-shape.md:77-86; spec/test-cases/TC-160-checked-family-contract-shape.md:131-136; qsl-semantics/src/check/claims/tests.rs:578-651 |
| FND-002 | low | FR-062's Status count sentence is garbled: "Six … are backed (AC-2, AC-5, AC-7, AC-8 and AC-12) and AC-4". AC-4 belongs inside the parenthesis. The TC-160 row in `spec/tests.md` is missing a separator: "AC-6 (first sentence only, QSL-246 owns the rest) AC-4 (step 5)". | spec/functional/FR-062-implement-checked-family-contract.md:545-547; spec/tests.md:47 |
| FND-003 | low | FR-062-AC-13's Status and the TC-160 row cite RR-14 as "does not parse". Under the leader's decision (rename to `all_positive`, test in this PR), both status lines must be rewritten in the fix round, or they will contradict the test. | spec/functional/FR-062-implement-checked-family-contract.md:537-543; spec/tests.md:47 |

## Coverage

- Plan tasks: no plan bundle. The ticket's TC steps were used, and all
  are implemented except RR-14 (leader-decided, fix round).
- Matrix: in-scope rows are all backed. `quire coverage` totals:
  966/1074 backed repo-wide.
- Untracked tests: none in the diff. Every new test carries `#[trace]`
  with TC and AC ids.
- Semantic review: done inline (see SR-670).

## Dispositions

Disposition pass at `agent-ix/quire-spec-language@27fa6f87c744677df61c0cfacfe0549425ec4800`
(renumbered from SR-642).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed 0872c86a | TC-160 step 11 and its expected result now describe location-based pairing: a paired site, a `generated`-only site, a site with no occurrence, an unclaimed application, and two sites of one node in both orders. Each clause maps to `tc_160_an_unpaired_site_or_application_faults_instead_of_dropping` or `tc_160_two_sites_of_one_node_pair_by_location_not_order`. The `None`-identity clause is gone. Step 10 adds the guard, scope and `fold`/`reduce`/`flatMap` units, each matching a test. |
| FND-002 | fixed 0872c86a | FR-062 now reads "Seven … are backed (AC-2, AC-4, AC-5, AC-7, AC-8, AC-12 and AC-13); three … partly". The TC-160 row in `spec/tests.md` is separated correctly. |
| FND-003 | fixed 0872c86a | RR-14 is `all_positive`, and `tc_160_rr_14_query_binder_roots` backs it with extents keyed by the binder `v` (not `s`), result bounds `Integer`/`Boolean` and no guard. The "does not parse" text is gone from FR-062 and `spec/tests.md`. FR-062-AC-13 is backed. |

Extra edits checked:

- ADR-014 §4 states the scoped roots and `fold`'s accumulator and element.
- FR-075's `bounded_item` signature matches `qsl-route/src/request.rs`.
- TC-160 step 10 matches the new tests.
- FR-062's ordinal wording (body before the `decreases` measure) matches
  RR-17's test.

The new ADR-012 row inconsistency is recorded as FND-007 in SR-670.
