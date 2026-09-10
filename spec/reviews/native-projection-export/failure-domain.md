---
id: SR-196
title: "failure-domain review of standalone projection export"
type: SpecReview
analysis: failure-domain
scope: "FR-029; TC-107; TM-007; Task-028"
review_set: all
---
## Summary

Static compilation and complete strict lowering finish before output begins. A later unsupported clause therefore cannot publish an earlier clause as a successful partial artifact. Errors retain request identity, the native package reference and original source; the test changes the later clause and checks its exact source slice. Output I/O limitations remain those of FR-027. Requests share no mutable runtime state.

Author PR-readiness review of `d1fcf16`, using the owner-selected all set.
No applicable AssuranceProfile was found; timing follows the owner directive.

Failure construction now resolves coordinates while the program is available
and retains metadata without source text. Missing and invalid spans have distinct
span_status values; invalid coordinates retain unmapped_span. Lowering categories
map exhaustively to the native Code catalog and existing exit classification.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | FR-029; TC-107 |
