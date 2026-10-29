---
id: SR-117431
title: "QSL-1232 dependency analysis"
type: SpecReview
analysis: dependency
scope: "FR-167/168/169 state-graph reduction dependencies"
review_set: all
---

## Summary

The new behavior depends on contracts already assigned to the model system, EN-1 reduction machinery, and replay/settlement boundaries. The direct requirements name those dependencies and preserve the existing ownership split.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved dependency or ownership gap found in the reviewed change. | FR-120; FR-167; FR-168; FR-170; ADR-021; ADR-022 |

## Dependency disposition

- FR-167 consumes FR-120 states and transitions; symmetry canonicalization and POR expansion defer to ADR-021.
- FR-168 consumes FR-167's reduced graph and FR-166 witness samples; it emits concrete counterexample evidence through the existing FR-170 replay contract.
- FR-169 maps outcomes through the existing FR-127/FR-331 verdict wire and uses certificate checking only for exhaustive results.
- No dependency is silently changed to a different owner or treated as completed by this spec change.
