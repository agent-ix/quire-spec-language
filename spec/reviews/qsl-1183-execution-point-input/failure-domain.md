---
id: SR-60322
title: "Failure-domain analysis of QSL-1183 accepted execution-point input"
type: SpecReview
analysis: failure-domain
scope: "FR-104 accepted-point input and identity handoff; TC-475"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: references
---

## Summary

The review identified missing locations for unmatched input and missing
coverage for equal-body clauses bound to different points. The revised input
uses exact authored names, defines a source-start location for map failures,
and TC-475 asserts distinct identities for initialization and handler points.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | An unmatched external key had no source region, so a refusal could not be located. | FR-104-AC-15 |
| FND-002 | medium | Equal-body clauses with different accepted points lacked an explicit identity assertion. | FR-104-AC-15, TC-475 |

## Resolution

FND-001: map failures use the check unit's source-start region; native
duplicates use the native check source-start region. FND-002: TC-475
asserts distinct checked node identities for equal-body initialization and
handler clauses, and FR-105-AC-12 continues to bind the point into identity.
