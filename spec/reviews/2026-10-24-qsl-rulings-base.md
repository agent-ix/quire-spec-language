---
id: SR-16701
title: "Base review of the QSL ruling batch"
type: SpecReview
analysis: base
scope: "FR-125, FR-169, FR-183, FR-289, FR-290, ADR-023, TC-520, TC-594, TC-608, TC-774"
review_set: subset
---

## Summary

Reviewed the changed requirements and test specifications against the base checklist, including a scoped trace matrix over the touched artifacts and related source test files. The settlement clarifications are consistent; the newly specified absent-key and no-instance behavior still lack source test evidence.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The scoped matrix reports FR-125-AC-12 and FR-169-AC-16 as untagged. Existing FR-125 tests cover only part of AC-12 (and their comments state that two overlapping populations are refused); no state-graph test asserts a per-item `NoInstance` terminal record. Keep both criteria as requirements and schedule implementation/test follow-up before claiming coverage. | FR-125-AC-12, FR-169-AC-16, TC-520, TC-594 | correct-requirement-no-evidence |
