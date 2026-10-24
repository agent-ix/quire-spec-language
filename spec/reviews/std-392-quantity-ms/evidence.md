---
id: SR-16369
title: "STD-392 evidence review"
type: SpecReview
analysis: evidence
scope: "FR-187-AC-1, FR-189-AC-15 and TC-622/TC-624"
review_set: all
---

## Summary

The changed acceptance criterion names TC-624, and the existing source tests carry the FR-189-AC-15 trace. TC-624 step 13 now states the exact-rational and unit/refusal cases for the follow-on code work; those added cases are planned evidence and have not been implemented or executed in this spec lane.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `quoin advise` recommends performance-benchmarking from the words “latency” and “quantified threshold”; this criterion sets no runtime performance bound, so the unit/BDD test recommendation is the applicable evidence. The authored Test method remains appropriate. | FR-189-AC-15 |

## Evidence mapping

| Obligation | Authored method | Evidence artifact |
| --- | --- | --- |
| FR-187-AC-1 | Test | TC-622 |
| FR-189-AC-15 | Test | TC-624; `qsl-analyze/src/statistical/tests.rs`; `qsl-analyze/tests/it/statistical_model.rs` |

The advisor reports `authored: Test`, `mismatch: false`, and no inconclusive recommendation for both scoped obligations. The new TC-624 step defines the follow-on checks; no implementation tests were run as part of this specification change.
