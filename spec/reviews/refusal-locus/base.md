---
id: SR-403
title: "Base review of protocol-role refusal loci"
type: SpecReview
analysis: base
scope: "FR-042-AC-8; TC-121; TM-003; issue #68"
review_set: subset
---

## Summary

Reviewed the refusal-locus amendment against the base checklist and the owner-selected six-lens
subset. The changed requirement and test case are ready: identifiers and relationships are valid,
the failure is externally observable, and no applicable AssuranceProfile overrides the selection.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The changed documents validate and FR-042 remains 10/10 backed; repository-wide structural validation still reports the pre-existing TestMatrix column assertion tracked as compiler #28. | FR-042-AC-8; TM-003; compiler #28 |

## Checks

FR-042 implements US-004, AC-8 names Test verification through TC-121, and TC-121 verifies
FR-042. TM-003 maps AC-8 to TC-121 as passed, while the concrete Rust symbol carries both trace
identifiers. The option-permutation, constraint-boundary, and state-transition rules introduce no
new cases for this diagnostic-only change. The error-path and edge-case rules are covered by a
two-unit package whose invalid protocol role must report `Invalid::Type`, source index 1, and the
exact refused declaration rather than the same byte range in source 0. The matrix supplies TC-121's
Integration/P1 classification and trace relationship.
