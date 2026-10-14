---
id: SR-9122
title: "Gap analysis of quire-spec-language PR #843: adapt to QSV fallible TypeEnvironment::admits"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@6426347a1c5709993e094bda667c6ff5890168e6; QSL-773 QSL half (lock bump + adapt every caller so a storage failure is never false or a type refusal); PR #843 diff"
review_set: subset
---
# Gap analysis of quire-spec-language PR #843

## Summary

Ticket: QSL-773, the QSL half. This is a planless gap analysis. The ticket's
own ACs cover the QSV seam (QSV #22, merged). The QSL half's criteria come
from the lead's brief, and are checked against the diff and its tests:

- C1. The lock moves to QSV 60107c5f and Cargo.toml stays branch=main (met).
- C2. Every `TypeEnvironment::admits` caller is adapted so that
  Allocation/Capacity is never `false` (met at all five sites).
- C3. Every consumer of the new `ConstructionCause` storage variants is
  adapted (not met at the evaluator's tuple and record construction; FND-001).
- C4. Tests are typed and fail on a wrong mapping. They are met for the
  mapping functions: `membership_storage_failures_are_resource_exhausted`,
  `storage_failure_settles_resource_exhausted_with_its_cause`,
  `storage_failure_converts_to_the_resource_exhausted_run_refusal`,
  `storage_failure_is_incomplete_not_a_refused_argument` and
  `storage_failure_is_not_a_false_membership_answer`. Each would fail if
  Allocation were mapped to invalid_runtime_input or `Ok(false)`. The
  call-site wiring cannot be reached from QSL, because QSV's
  ReservationPolicy is private. This is recorded.
- C5. The gate ran on the final head: make ci exit 0, log
  qsladapt-ci.log written after the commit (met).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Criterion C3 is unmet. Tuple and record construction discard ConstructionRefusal, so StorageDenied/StorageCapacity read as `*-construction-rejected`. This is the same defect as SR-9121 FND-001 | qsl-eval/src/value/expression/evaluate.rs:1571, qsl-eval/src/value/expression/evaluate.rs:1605 |

## Verdict

**FAIL on C3 only (FND-001, the same defect as SR-9121 FND-001).** C1, C2, C4
for the mappings, and C5 are met.
