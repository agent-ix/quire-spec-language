---
id: SR-405
title: "Integrity review of protocol-role refusal loci"
type: SpecReview
analysis: integrity
scope: "FR-042-AC-8; TC-121; TM-003; issue #68"
review_set: subset
---

## Summary

The added obligation is complete, consistent, atomic at the changed behavior, and testable through
the public native-admission report. Its owning story, verification method, test case, matrix row,
and concrete trace binding are present.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved completeness, consistency, ambiguity, or testability defect was found in the issue #68 amendment. | FR-042-AC-8; TC-121; TM-003 |

The requirement distinguishes the correct `Invalid::Type` verdict from the independent diagnostic
identity invariant, so a correct refusal cannot mask a wrong locus. It neither changes package
admission nor conflicts with FR-042's existing partial-result behavior. The fixture fixes declaration
order and checks both the selected source and exact slice, avoiding an assumption that a matching
span alone proves source ownership.
