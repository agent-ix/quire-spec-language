---
id: SR-236
title: "failure-domain review of bounded integer IR lowering"
type: SpecReview
analysis: failure-domain
scope: "FR-033; TC-111; TM-006; Task-032"
review_set: all
---
## Summary

The target admits only checked primitive expressions and direct model declarations. Calls, locals, fields, references and graph forms refuse the complete package, including a later unsupported clause. Original nominal identity remains in the native model and direct-read correspondence. Counters and bounded serialization stay per request. Unknown CLI targets refuse before file intake; runtime values do not enter source-only lowering.

Author PR-readiness review of `5a7e5db`, using the owner-selected all set.
No applicable AssuranceProfile exists; timing follows the owner directive.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | FR-033; TC-111 |

