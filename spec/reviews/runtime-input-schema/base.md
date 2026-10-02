---
id: SR-276
title: "Base review of the runtime input schema"
type: SpecReview
analysis: base
scope: "FR-024-AC-5; TC-099/100; Task-036; f4679ef against 6d7de6a"
review_set: all
---
## Summary

PR-readiness review uses the owner-selected all set and the QUOIN contract.
FR-024 retains US-003 → StR-001 ownership; the new structural schema obligation
is covered by TC-099/100. No required AssuranceProfile applies.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Checks

IDs, relationships, inputs, outputs and refusal ownership are explicit. The six
coverage checks cover both envelope kinds, three observations, null/present
results, ten variants, required/unknown/mistyped fields, integer/index/revision
domains, identifier/digest spelling and duplicate vectors. There is no new state
transition or configuration option. Raw-byte and runtime validity are separately
limited. The selected stack in docs/matrix-status.md binds FR-024 5/5 and TM-007
16/16, with zero status lies; installed-stack adoption remains #28. Grammar
validation and actual run evidence are recorded by SR-284/285.
