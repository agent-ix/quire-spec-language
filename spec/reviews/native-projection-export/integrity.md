---
id: SR-197
title: "integrity review of standalone projection export"
type: SpecReview
analysis: integrity
scope: "FR-029; TC-107; TM-007; Task-028"
review_set: all
---
## Summary

FR-029 traces to US-004 and reuses FR-027 input ownership and FR-009 target semantics. It introduces no new wire version or implicit numeric/object support. Native package bytes and derived executable bytes remain distinct. Closed source-only decoding rejects runtime fields, and the fixed lowering target needs no configuration permutations. All scoped IDs and relationships validate.

Author PR-readiness review of `d1fcf16`, using the owner-selected all set.
No applicable AssuranceProfile was found; timing follows the owner directive.

The corrected error interface adds explicit coordinate status while retaining
existing code spellings and located span fields. The typed command parser admits
lower directly; shared fixture variants name their flag/frame payloads.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | FR-029; TC-107 |
