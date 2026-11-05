---
id: SR-20261105101011
title: "QSL-1731 integrity analysis"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md, spec/test-cases/TC-1285-a-quantity-negation-lowers-to-quantity-negate.md, spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
---
## Summary

The new accepted and refused cases link to concrete test vectors in TC-1285 and TC-483, including nested negation, zero, parameter, `let`, and call operands. A preexisting AC-34 rationale is now inconsistent with the changed quantity-divisor rule.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-093-AC-34 states that any quantity divisor not itself a nonzero literal is unproved, while FR-093-AC-37 admits a divisor formed by negating a nonzero literal. The AC-34 example is specifically a parameter divisor; scope its explanation to that case or restate the complete proof rule. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:1038 | wrong-requirement |
