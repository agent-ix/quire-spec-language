---
id: SR-067
title: "Native model and checker failure-domain review"
type: SpecReview
analysis: failure-domain
scope: "FR-015/016, FR-006 judgments, docs/native-model-checking.md, IT-005, TC-025–029/040–053 and TM-003"
review_set: all
evaluated_revision: "ceccabb564b61742597ad356eb1019ab0c8d1544"
review_date: "2026-09-08"
---

## Summary

The corrected contract distinguishes model identity from runtime population and bounds proof work before expansion. No unresolved specification finding remains in this scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Resolved in 6d58113: enumerating object instances in the model would couple artifact identity to population contents; the selected carrier now contains a bounded opaque text ID and runtime membership stays separate. | FR-015; FR-016-AC-9; TC-040; TC-052 |
| FND-002 | high | Resolved in the reviewed contract: shared Boolean aliases and alternative joins need bounded, sound fact derivation before IR materialization; TC-053 independently checks the new presence rules. | FR-016-AC-1; FR-016-AC-7; FR-016-AC-8; TC-051; TC-053 |

## Failure domains

Trust boundaries are typed caller-supplied models, source identities and authored clause mappings. Admission is atomic and strict; no callbacks, external CLI, executable model hooks or runtime samples participate. FormalSource establishes exact coordinates and byte correspondence, while the caller remains responsible for identity assignment and the future Quire consumer for manifest authority.

Entity keys are owner-qualified declarations and native roles, qualified clause identity, object type/universe, and structured proof keys with observation or lexical identity. Duplicate sites/frames refuse before sorting. Conflicting source bindings or owner artifacts refuse; identical candidates retain ambiguity. Reference carrier fields cannot bypass native reference semantics.

All inputs are immutable. Generated proof inputs are symbolic assumptions conditional on later valid populations, never observed truth. Opaque nonnumeric payload equality cannot manufacture facts. Every initializer has its earlier proof obligations before body guards; all branches still receive name/type checks.

IR model closure is acyclic. Native expression and proof DAG traversal has explicit node/depth budgets; repeated DAG expansion is charged before allocation. Presence union/intersection work is separately bounded. Runtime graph traversal and cycle termination remain FR-007/evaluation work; reaches here records a typed obligation, not a traversal claim.

## Verdict and provenance

PASS for implementation of this specified scope. Agent A applied the actual
QUOIN base and all seven analysis skills serially, following the owner's
selected all review set. No subagents or builds were started for this review.
No applicable required AssuranceProfile was found. Catalog schema and existing
public interfaces were inspected. Implementation/test completion is not claimed.
The owner-declined optional gap-analysis semantic comparison remains excluded.

