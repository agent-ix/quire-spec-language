---
id: SR-179
title: "evidence review of standalone compiler export"
type: SpecReview
analysis: evidence
scope: "FR-027; TC-105; TM-007; Task-026"
review_set: all
---
## Summary

quoin advise --json again failed CLI-version detection with installed quire 0.31.0. Using the previously fetched installed method catalog, author judgment chooses contract-testing/integration-testing for AC-1/2 and negative-abuse-testing for AC-3; all are Test methods. The real CLI output is compared with the existing static producer and actually reread with supplied bindings. This establishes byte/consumer correspondence, not independent semantic correctness or full assurance.

Author PR-readiness review of `f0cfe7b` with the owner-selected all set.
No applicable AssuranceProfile was found; review timing follows the owner directive.

The corrected suite adds a fresh selected-source-only directory and actual /dev/full output failure. Named count failures assert category and remaining capacity before source I/O. These are focused contract controls, not an exhaustive assurance claim.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Advisor version detection is unavailable; method choices are author judgment. | FR-027; TC-105 |
