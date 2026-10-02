---
id: SR-267
title: "Integrity review of owned runtime decoding"
type: SpecReview
analysis: integrity
scope: "FR-024 amendment; TC-099/100; Task-035; implementation 3c6a0e6"
review_set: all
---
## Summary

The amendment makes existing closed-wire rules apply at each public type boundary without changing native-state-input/1 or public draft field types.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | - |

## Traceability

| Story | Requirement | Stakeholder need | Verification |
| --- | --- | --- | --- |
| US-003 | FR-024 | StR-001 | TC-099/100; Rust integration tests |

## Checks

The requirement distinguishes decoding, structural construction and model validity. NFR-006 limits still attach to construction and execution; no unbounded direct-decoder admission guarantee is invented. Missing fields, explicit null and canonical digest text have one interpretation. The unchanged native profile ruling is tracked under #30; this wire correction does not resolve its semantic questions. No CLI, registry precedence, pagination, authentication or new dependency assumption enters this amendment.

