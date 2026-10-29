---
id: SR-271
title: "Scope review of owned runtime decoding"
type: SpecReview
analysis: scope-boundary
scope: "FR-024 amendment; TC-099/100; Task-035; implementation 3c6a0e6"
review_set: all
---
## Summary

A owns FR-024's native runtime input boundary. The change consumes pinned Rust Serde and IR constructors without modifying B/C repositories or native language admission.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | - |

## Allocation

| Requirement | Owner | Class |
| --- | --- | --- |
| FR-024 amendment | Compiler runtime input module | core |

Callers → public type Deserialize → closed native wire types → existing IR/identity constructors; selected artifact readers additionally apply byte selection and structural admission. Serde object/tag behavior is guaranteed for the named positive/adverse controls by TC-099/100; broader library correctness remains assumed. IR identifier constructors are consumed at pinned 690bde7 with invalid-identifier controls. SHA-256 stays in the existing sha2 dependency; only text encoding is refactored. The standalone schema and producer resource predicate remain follow-ups under #27, with no implication that foreign tools are qualification dependencies.

