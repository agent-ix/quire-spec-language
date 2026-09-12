---
id: SR-382
title: "Dependency review of reconciled native temporal subsystem"
type: SpecReview
analysis: dependency
scope: "FR-043, FR-044, FR-045, NFR-008, TC-122-125, TM-008"
review_set: all
---
## Summary

The native evaluation and activation requirements are implemented on PR #70's
admitted temporal body; the classifier is deliberately classification-only.
The dependency order is acyclic: shared semantics → A's `/2` contract and F's
observation authority → Contract IR predicate projection → exact TL
correspondence → end-to-end qualification.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-045 must remain classification-only until Contract IR #63/#64 and the recorded closure-axis decision are available. Treating a supported classification as a formula or correspondence implementation would violate the dependency boundary. | FR-045, TC-125 |
