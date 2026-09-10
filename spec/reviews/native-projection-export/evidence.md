---
id: SR-199
title: "evidence review of standalone projection export"
type: SpecReview
analysis: evidence
scope: "FR-029; TC-107; TM-007; Task-028"
review_set: all
---
## Summary

quoin advise --json remains unavailable because CLI-version detection fails with installed quire 0.31.0. Author judgment from the installed method catalog selects integration/contract testing for AC-1 and negative-abuse testing for AC-2/3. Actual command bytes pass both pinned IR readers and complete backend generation. The tests inspect generated Rust/source maps but do not claim new generated execution or activation assurance.

Author PR-readiness review of `d1fcf16`, using the owner-selected all set.
No applicable AssuranceProfile was found; timing follows the owner directive.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Advisor version detection is unavailable; method choices are author judgment. | FR-029; TC-107 |
