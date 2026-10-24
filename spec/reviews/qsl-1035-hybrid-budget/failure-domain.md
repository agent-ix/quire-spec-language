---
id: SR-1311
title: "Failure-domain analysis of hybrid budget coverage"
type: SpecReview
analysis: failure-domain
scope: "FR-250-AC-3/5, TC-705"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: reviews
---

## Summary

Checked malformed hybrid limits, elapsed-time exhaustion, cancellation, timed-out storage, and NoInitialState handling on both provider paths. The existing criteria retain these distinct failures and the added process cases exercise the v2 request/result boundary.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No additional failure-domain gap found in the changed scope. | FR-250-AC-3/5; TC-705 |
