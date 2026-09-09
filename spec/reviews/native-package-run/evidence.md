---
id: SR-189
title: "evidence review of selected package execution"
type: SpecReview
analysis: evidence
scope: "FR-028; TC-106; TM-007; Task-027"
review_set: all
---
## Summary

quoin advise --json failed CLI-version detection with installed quire 0.31.0. From the previously fetched installed method catalog, author judgment selects integration/contract testing for AC-1/2 and negative-abuse testing for AC-3/4. Tests export with the real compiler binary, execute selected bytes, and alter claims with recomputed digests. Alternate layout checks raw identity independently of semantic observations. These are local behavioral checks, not independent semantic assurance.

Author PR-readiness review of implementation `8db2292` and its final grammar correction.
The owner-selected review set is all; no applicable AssuranceProfile was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Advisor version detection is unavailable; method choices are author judgment. | FR-028; TC-106 |

