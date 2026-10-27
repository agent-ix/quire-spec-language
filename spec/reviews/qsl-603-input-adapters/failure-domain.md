---
id: SR-60302
title: "Failure-domain review of QSL conformance input adapters"
type: SpecReview
analysis: failure-domain
scope: "FR-350 corpus input adapters and TC-875"
review_set: all
---

## Summary

Reviewed malformed tagged values, population identity resolution, request construction and path-based byte provision. Each boundary has a rejecting owner and replay paths remain outside the typed product request.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No additional failure-domain issue found; malformed call values, unmatched population names, unsafe or unlisted byte paths and FR-071 digest failures are separately assigned. | FR-350-AC-8, FR-350-AC-9, FR-350-AC-10 |
