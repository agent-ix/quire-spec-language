---
id: SR-404
title: "Failure-domain review of protocol-role refusal loci"
type: SpecReview
analysis: failure-domain
scope: "FR-042-AC-8; TC-121; issue #68"
review_set: subset
---

## Summary

The amendment closes the relevant identity-confusion failure domain: a mutable diagnostic locus can
no longer combine one declaration's source identity with another declaration's span. The exact
multi-source negative fixture makes that defect distinguishable from a correct typed refusal.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unaddressed extension, identity, purity, or topology failure domain remains in the issue #68 slice. | FR-042-AC-8; TC-121 |

The compiler owns the source/span pair and must establish both components from the protocol role
before any fallible model/type admission. No callback, graph traversal, user-supplied evaluator, or
new entity key is introduced. Existing work limits and typed refusal causes remain unchanged.
