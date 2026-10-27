---
id: SR-60304
title: "Dependency review of QSL conformance input adapters"
type: SpecReview
analysis: dependency
scope: "FR-350 and TC-875"
review_set: all
---

## Summary

Reviewed the replay adapter dependency graph: QSL consumes QSpec FR-336 harness forms, FR-323 reused envelopes, and FR-071/072 request/result contracts without changing those owners.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No dependency cycle or missing owner edge found; QSpec owns corpus serialization and QSL owns the adapter binding. | FR-350; FR-336; FR-323; FR-071; FR-072; FR-098 |
