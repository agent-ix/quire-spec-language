---
id: SR-20261105101013
title: "QSL-1731 failure-domain analysis"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md, spec/test-cases/TC-1285-a-quantity-negation-lowers-to-quantity-negate.md, spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
---
## Summary

Reviewed the changed quantity proof forms for missing refusal boundaries and pathological nesting. The spec distinguishes zero literals and negated zero from nonzero roots, and excludes parameter, `let`, call and other computed roots; no failure-domain defect was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
