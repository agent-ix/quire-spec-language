---
id: SR-60323
title: "EARS conformance analysis of QSL-1183 accepted execution-point input"
type: SpecReview
analysis: ears-conformance
scope: "FR-104 input, behavior and AC-15"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: reviews
---

## Summary

The first draft used an untestable "existing applicability" reference and did
not state the accepted point/kind pairing. FR-104 now names S3 as subject and
gives concrete refusal behavior for inapplicable variants and mismatched
operations.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The original applicability statement did not specify the invalid condition, result or location. | FR-104-AC-15 |

## Resolution

The revised FR-104 SHALL statements name point variants by clause kind,
operation-name source, `invalid_model_binding`/`malformed-declaration`, and
the clause-name span. TC-475 makes those cases observable.
