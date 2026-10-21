---
id: SR-13661
title: "integrity review of STD-177 FR-150 fold precedence and its relatives (PR #1143)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@5a71fd301e44e5072238efd8a3308780b5279b55; spec/functional/FR-150-check-the-symmetric-population-annotation.md, spec/test-cases/TC-565-s3-accepts-and-refuses-the-symmetric-annotation.md, spec/decisions/ADR-021-state-space-reduction.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-150
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-565
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: references
---
# SR-13661: integrity review of STD-177 (PR #1143)

## Summary

Ticket: STD-177. This review checks consistency between FR-150's new
precedence bullet, FR-150's own form 1 definition, and ADR-021 SYM-3, which
FR-150 cites as the source of the forms. Probe outcomes are in SR-13659's
Summary.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | ADR-021 SYM-3 lists as a breaking form "a `fold`, `reduce` or other order-sensitive traversal of a `Set` or `Bag` whose element type contains a reference into the population". It has no catalog exception. One reading makes every fold or reduce over references break transparency. That contradicts FR-150's new rule that a catalog fold is not form 1 and checks in an annotated unit. FR-150 depends_on ADR-021 and cites SYM-3 as the source of the forms. Add the FR-145 catalog exception to SYM-3 in this PR. | spec/decisions/ADR-021-state-space-reduction.md:131 |
| FND-002 | low | Form 1 is defined by meaning: a traversal "whose result depends on element order" (lines 67-68). The new bullet decides it by syntax: a step outside the catalog is reported as form 1. These two disagree. `fold<Tally>(a, q in self.peers: 1, identity: 0)` is outside the catalog but does not depend on order, and it is refused `identity-observing-form` with `IdentityForm::OrderSensitiveTraversal`. Define form 1 as "a fold or reduce over a Set or Bag whose step is outside FR-145's catalog, or another traversal that depends on element order". | spec/functional/FR-150-check-the-symmetric-population-annotation.md:67-68 |

## Verdict

Mergeable on integrity grounds only with FND-001 fixed or deferred to a
ticket. FND-001 is the ADR half of the same rule and leaves the source text
for FR-150's forms out of step. FND-002 is a definitional nit.

FR-150 and TC-565 agree with each other: TC-565 step 2 and its expected
results match AC-2 clause by clause. No AC id collides. TC-565's scope line
(AC-1 to AC-2) is unchanged and correct.
