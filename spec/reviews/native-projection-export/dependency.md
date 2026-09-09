---
id: SR-198
title: "dependency review of standalone projection export"
type: SpecReview
analysis: dependency
scope: "FR-029; TC-107; TM-007; Task-028"
review_set: all
---
## Summary

The existing source frontend/static pipeline creates a NativePackage, then existing Boolean lowering invokes the strict IR binder. Both pinned IR readers and the current code generator consume the resulting bytes in the real integration test. Backend activation assurance and Quire extraction adoption remain separate. This command requires no new producer API or runtime input artifact.

Author PR-readiness review of `4f2c20f`, using the owner-selected all set.
No applicable AssuranceProfile was found; timing follows the owner directive.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | FR-029; TC-107 |

