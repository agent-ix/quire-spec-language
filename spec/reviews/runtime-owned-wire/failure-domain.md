---
id: SR-266
title: "Failure-domain review of owned runtime decoding"
type: SpecReview
analysis: failure-domain
scope: "FR-024 amendment; TC-099/100; Task-035; implementation 3c6a0e6"
review_set: all
---
## Summary

FR-024 retains strict refusal across direct and nested decoding. No new callback, object resolution, graph traversal or I/O is introduced.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | - |

## Checks

Snapshot and invocation selectors remain distinct types with exact identity/revision/digest. Duplicate vector entries remain available to later model-aware validation; duplicate object keys refuse. Required nullable results have one explicit null representation. Byte preflight, default Serde recursion limits and structural construction remain the artifact admission boundary. Direct Serde decoding creates a draft and is not a resource-bounded artifact admission API.

