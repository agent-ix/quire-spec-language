---
id: SR-16712
title: "Evidence analysis of QSL-1032 timed model identity binding"
type: SpecReview
analysis: evidence
scope: "spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md (AC-32, AC-38); scoped matrix for QSL-1032 timed-source tests"
review_set: subset
---

## Summary

`Test` can observe the selected NodeKey, the typed missing/ambiguous S3 refusals, read-back equality, and the producer/read-back package identity. The scoped static matrix currently reports AC-32 and AC-38 untagged; QSL-1032 must add direct test bindings.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | No test currently binds AC-32 or AC-38; the implementation PR must add tests that fail for incorrect model-node mapping and both refusal shapes. | FR-230-AC-32, FR-230-AC-38 | correct-requirement-no-evidence |
