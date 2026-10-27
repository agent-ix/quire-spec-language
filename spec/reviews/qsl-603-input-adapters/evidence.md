---
id: SR-60305
title: "Evidence review of QSL conformance input adapters"
type: SpecReview
analysis: evidence
scope: "FR-350-AC-8 through FR-350-AC-10 and TC-875"
review_set: all
---

## Summary

TC-970 specifies schema mutations and byte-path refusal cases; QSL replay adapter tests remain pending the implementation slice, as intended by spec-before-code order.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new replay adapter mapping is test-verifiable; implementation must bind every case before the ticket can close. | FR-350-AC-10; FR-336-AC-7/8; TC-970 |
