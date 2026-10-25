---
id: SR-17018
title: "Failure-domain analysis of QSL-1054 guard provenance"
type: SpecReview
analysis: failure-domain
scope: "spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md; spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md; spec/test-cases/TC-520-model-behaviours-read-as-temporal-traces.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-361
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-520
    type: reviews
---

## Summary

Failure-domain review focused on provenance leaks across aliases and helper
calls. The rule limits the exception to forwarding the bound reference and to
using that provenance-bearing reference as a `lookup` operand; it preserves
`absent-key` for evaluated field reads and leaves a false guard's consequent
unevaluated.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved failure-domain finding remains for the delta. | FR-125-AC-17; FR-361-AC-16 |
