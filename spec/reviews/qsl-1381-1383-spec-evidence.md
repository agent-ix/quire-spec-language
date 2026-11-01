---
id: SR-1957021
title: "Evidence review — QSL-1381/1382/1383 checked successors"
type: SpecReview
analysis: evidence
scope: "agent-ix/quire-spec-language; FR-043, FR-044, FR-045, FR-046-AC-5, TC-1277, TC-1278, TC-1279, TC-1280; checked temporal and query-proof integration tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-044
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-045
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-046
    type: reviews
---

## Summary

Compared each authored checked successor procedure with the executable checked-route vectors and trace tags. Test is the appropriate declared verification method, but TC-1277 currently claims several edge-case assertions that are absent from its executable cases.

## Verdict

**FAIL** — TC-1277's procedure and FR-043 successor coverage overstate the executable evidence for required subcases.

## Coverage

- Reviewed all retained FR-043/044/045 ACs and FR-046-AC-5 against named checked-route tests and current code.
- Reviewed TC-1277 through TC-1280 procedures and expected results against tests/it/checked_temporal_successors.rs, tests/it/compiled_protocol_v2.rs and qsl-replay/tests/it/query_proof.rs.
- All authors retain Test verification. `quoin advise --json` on Quoin 0.29.1 ran for approximately two minutes without stdout or stderr and was interrupted (exit 130); manual verification-method review confirms the checked cases are unit/integration tests, but no advisor catalog recommendation is claimed.
- The gap is evidence completeness, not an argument to remove behavior or change verification method.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-1277 describes interior-history, completeness-conflict and identity-premise vectors that are not asserted by the named checked tests; FR-043 successor evidence therefore does not yet demonstrate every claim in the case procedure. | spec/test-cases/TC-1277-evaluate-checked-package-temporal-obligations.md:44 |

## Evidence detail

The TC-1277 procedure says `checked_past_operator_requires_authoritative_history` also omits an interior history interval, but that function only compares authoritative origin with a history cutoff. Its progress step covers changed decision-scope and watermark assertions, but not a changed completeness assertion or changed clock/parameter premises. The corresponding FR-043 ACs remain tagged to TC-1277, so tags alone do not discharge those obligations.
