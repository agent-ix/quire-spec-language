---
id: SR-117437
title: "QSL-1232 scope-boundary analysis"
type: SpecReview
analysis: scope-boundary
scope: "QSL model-check, replay and settlement ownership for reduced state graphs"
review_set: all
---

## Summary

The change remains within QSL's EN-1 state-graph exploration and settlement path. It reuses the existing evaluator transition system, replay facade and reduced-proof wire rather than moving reduction behavior into source admission or changing replay semantics.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved ownership or boundary issue found in the changed scope. | FR-101; FR-120; FR-167; FR-168; FR-169; FR-170; ADR-021; ADR-022 |

## Boundary disposition

- `qsl-eval` owns the model transitions and reduction hooks; `qsl-analyze` owns state-graph expansion and claim outcomes.
- `qsl-replay` continues to re-establish concrete evidence and owns the terminal proof vocabulary. `StateGraphOutcome::HoldsReduced` is distinct from FR-143's `ModelCheckOutcome` inside `LivenessHalf::Checked` and the hyper result types.
- Reduced proofs are explicitly uncertified because the current core checker does not check quotient or ample-set closure.
- No parser, package wire, provider negotiation or external repository contract is changed by this QSL specification batch.
