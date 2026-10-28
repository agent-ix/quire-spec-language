---
id: SR-60321
title: "Integrity analysis of QSL-1183 accepted execution-point input"
type: SpecReview
analysis: integrity
scope: "FR-104 input, output, behavior and AC-15; TC-475"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: reviews
---

## Summary

The review found missing validation order and error locations, an untestable
invalid-point path, and an underspecified operation-name source. FR-104 and
TC-475 now specify each case before lowering and use the authored, resolved
`on M::T::op` member as the operation-name source.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Unknown/duplicate/non-clause binding failures and clause/function ambiguity ordering lacked defined locations and precedence. | FR-104-AC-15, TC-475 |
| FND-002 | medium | Applicability and operation mismatch had no criterion-level refusal or test cases. | FR-104-AC-15, TC-475 |

## Resolution

FND-001: map and native duplicate errors now name their source-start regions;
existing clause/function ambiguity runs first at both declaration spans.
FND-002: the criterion and TC-475 enumerate allowed point/kind pairs, the
mismatch code, and the clause-name span.
