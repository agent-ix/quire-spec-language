---
id: SR-16710
title: "Failure-domain analysis of QSL-1032 timed model identity binding"
type: SpecReview
analysis: failure-domain
scope: "spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md (time member behavior, AC-32, AC-38)"
review_set: subset
---

## Summary

The identity boundary now requires one checked node and explicitly refuses absence and ambiguity at the alias span. The no-order/no-fallback rule prevents declaration order or spelling from silently selecting a different model identity.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No additional failure-domain gap found in the touched resolution rule. | FR-230-AC-32, FR-230-AC-38 |
