---
id: SR-164202
title: "Failure-domain review — QSL-1642 timed population scope"
type: SpecReview
analysis: failure-domain
scope: "spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md; spec/functional/FR-391-name-a-population-in-a-state-clause.md; spec/test-cases/TC-1331-refuse-named-populations-in-timed-model-predicates.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: references
---

## Summary

The refusal boundary distinguishes a declared population outside timed-predicate scope from an unknown name, and keeps model-state identity with `self`. The requirement prevents silent fallback to an arbitrary or model-context population.

## Verdict

**PASS** — no additional failure-domain requirement is needed for this scoped decision.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found. | - |

## Analysis

- The refusal applies to declared named populations used by `allInstances`, `lookup`, a quantifier domain, or a population form. Unknown declarations retain the ordinary missing-name behavior.
- The first population reference is the refusal locus, and no timed model is emitted after refusal.
- A self-only timed predicate remains valid. Population reads in unit-level clauses and `holds` atoms remain governed by FR-391.
- No callback, graph traversal, or new state identity is introduced.
