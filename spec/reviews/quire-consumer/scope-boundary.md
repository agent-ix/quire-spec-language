---
id: SR-211
title: "scope-boundary review of the actual Quire consumer"
type: SpecReview
analysis: scope-boundary
scope: "FR-030; FR-011; IT-003; TC-108; TM-007; Task-029"
review_set: all
---
## Summary

FR-030 belongs to A's compiler integration (core); FR-011 describes the compiler-side join (core). Quire owns Markdown recognition, extraction availability and its schemas. Its selected body/coordinate contract is checked by actual IT-003 tests; the caller's original-source and context selection remain explicit assumptions. C retains producer and existing-repository adoption, B retains portable verification. No C/B repository or new wire schema is changed.

Author PR-readiness review of `55649b3`, using the owner-selected all set.
No applicable AssuranceProfile was found; timing follows the owner directive.

ClausesOutcome and SemanticContext are deliberate pinned upstream re-exports.
Protocol versions and hard input limits have named consumer constants. Pairing
native/formal identity fields with the command wire records remains PR23 work.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | FR-030; FR-011; TC-108 |
