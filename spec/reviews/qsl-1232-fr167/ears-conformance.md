---
id: SR-117432
title: "QSL-1232 EARS conformance analysis"
type: SpecReview
analysis: ears-conformance
scope: "Changed normative requirements in FR-167, FR-168 and FR-169"
review_set: all
---

## Summary

The new normative behaviors are expressed with explicit SHALL statements and concrete acceptance outcomes. Scoped grammar validation reports no EARS finding in the changed requirements.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved EARS conformance issue found in the changed requirements. | FR-167; FR-168; FR-169 |

## Disposition

- FR-167 states when the graph is canonicalized, how symmetry permutations and POR edges are retained, and when reduced expansions are complete.
- FR-168 defines the reduced outcome, its reduction list, certificate absence and completion conditions.
- FR-169 states the settlement category and certification for reduced and exhaustive results.
- Repository-wide grammar findings are outside the changed requirements; the scoped validation reports only existing catalog advisories.
