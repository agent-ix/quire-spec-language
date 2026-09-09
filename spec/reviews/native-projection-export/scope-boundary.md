---
id: SR-201
title: "scope-boundary review of standalone projection export"
type: SpecReview
analysis: scope-boundary
scope: "FR-029; TC-107; TM-007; Task-028"
review_set: all
---
## Summary

A owns standalone native compilation and export. Existing IR binding and codegen retain semantic and generation authority; the command invokes neither an external process nor a new evidence store. C retains producer expansion and Quire adoption; B retains portable verification. Native model/runtime obligations remain in the separately exported native package, not silently promoted into a proof claim.

Author PR-readiness review of `4f2c20f`, using the owner-selected all set.
No applicable AssuranceProfile was found; timing follows the owner directive.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | FR-029; TC-107 |

