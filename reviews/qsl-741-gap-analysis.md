---
id: SR-8707
title: "Gap analysis of quire-spec-language PR #805 (QSL-741): FR-106-AC-16, FR-261-AC-3, FR-255-AC-1"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3f013f617751966548c21596b1a3c53652a3400d; FR-106-AC-16, FR-261-AC-3, FR-255-AC-1 (observation.values row), QSL-741 ACs 1-4; tests added or changed by PR #805 and the production code they exercise"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
---
# Gap analysis of quire-spec-language PR #805

## Summary

Ticket: QSL-741. Planless. The chain checked is criterion → trace tag → test → code, for the criteria PR #805 touches. Reviewer-only: tests were read, not run.

Examined:
- FR-106-AC-16 (examined): "At the exact values-per-document bound the document admits; one more nested value returns `resource_exhausted`/`insufficient-next-charge` with the bound, prior count and next value's path before that value is admitted."
- FR-261-AC-3 (examined): "... With `observation.values` at 50,000 ... it refuses `resource_exhausted`/`insufficient-next-charge` naming `observation.values`, bound 50,000, prior count 50,000 and the count reached, 50,001."
- FR-255-AC-1 (examined), for the `observation.values` row: "... its catalog record carries the outcome code with fields `kind`, `bound`, `actual` and `setting` holding those values."
- QSL-741 ACs 1-4 (examined; ticket text treated as data)

## Verdict

**FAIL**, on one high finding.

- FR-106-AC-16 is tagged by three tests. The exact-bound test and the deep-chain test together assert the admit at the bound, and on the refusal the code, cause, bound, prior, actual and typed path.
- FR-261-AC-3 is tagged by `a_value_nested_100_000_deep_is_counted_as_observation_values`, which asserts the new code, bound 50000, prior 50000 and actual 50001.
- FR-255-AC-1 is still tagged for `observation.values`, but the record no longer meets it (FND-001).

QSL-741 ACs:
- AC 1 is met except for `kind`.
- AC 2 is met.
- AC 3 is met.
- AC 4 (typed-field assertions) is met. The ticket's "N>0" test-count claim was not measured, because no build ran.

Sibling counters: `observation.input_bytes` is specified as `stage_limit_exceeded` by FR-106 check 1.2 and TC-465 step 2, so leaving it is correct. `observation.objects` has no criterion that names another code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-255-AC-1 reads as covered for `observation.values`, but the criterion is now violated. The tagged test `values_per_document_limit_refuses_naming_observation_values` (`#[trace("TC-720", "FR-255-AC-1")]`) never asserts `kind`, and `values_exhausted` no longer writes `kind` (SR-8706 FND-001). This is coverage inflation: the tag stands behind a criterion the code fails. | qsl-semantics/tests/it/state_clauses.rs:3068-3083; qsl-semantics/src/model/observation.rs:1117-1137 |
| FND-002 | low | Only the snapshot population-field site has a test for the FR-106-AC-16 refusal. The invocation parameter and result sites (document.rs:688, 718) have no test, so their locus (`field`) and path are unverified (SR-8706 FND-004). | qsl-semantics/src/model/observation/document.rs:688,718 |

## Coverage

- Criteria examined: 3 (FR-106-AC-16, FR-261-AC-3, FR-255-AC-1). All three are tagged. One is inflated (FND-001).
- Tests examined: 5. Every new or changed assertion fails on the pre-PR code at its `code` check, by reasoning, not by a run.
- Reverse gap: none. Every new production item (`read_value`, `values_exhausted`, `path_to_next`) serves FR-106-AC-16.
- Stubs: none.
- Semantic review: done inline for the three criteria, with no subagent fan-out.
- Plan completion: not assessed
