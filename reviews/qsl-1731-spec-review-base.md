---
id: SR-20261105101010
title: "QSL-1731 base requirements review"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md, spec/test-cases/TC-1285-a-quantity-negation-lowers-to-quantity-negate.md, spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
---
## Summary

The scoped IDs, links and changed test vectors are coherent, and all four changed documents validate. One unchanged acceptance-criterion explanation now contradicts the new accepted negated-literal divisor cases and should be narrowed to its parameter example. The repository-wide static matrix did not return within 30 seconds, so full-corpus AC-to-test coverage is not claimed.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-093-AC-34 says every quantity divisor that is not itself a nonzero literal is unproved, contradicting FR-093-AC-37's accepted direct and nested negations of nonzero literals. Clarify that the refusal in this AC is for its parameter divisor (`e`), or qualify the rule to include negation chains rooted in nonzero literals. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:1038 | wrong-requirement |
