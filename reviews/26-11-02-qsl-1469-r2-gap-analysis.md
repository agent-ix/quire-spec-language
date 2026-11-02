---
id: SR-2026110216372
title: "Gap analysis — QSL-1469 round 2: root only the nodes FR-496 rows name"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/lowering/model.rs; qsl-package/src/emit/tests/owners.rs; qsl-package/src/emit/tests/receiver_frames.rs; FR-094; FR-496-AC-1; FR-496-AC-5"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-496
    type: reviews
---

## Summary

Ticket: QSL-1469. PR agent-ix/quire-spec-language#2125, head 4de61267d. This is a planless gap analysis scoped to the PR diff. Plan completion: not assessed.

## Verdict

**CHANGES REQUESTED** (FND-001).

Mutation results (qsl-package lib, deep stress tests skipped):

- Drop the declaring owner (`once(&operation.owner)`): 28 tests fail, including the new `owners::an_aliasless_models_rows_name_emitted_declaring_nodes`. Covered.
- Drop the written-field chain: 510 pass. Not covered.
- Drop the `creates` and `deletes` chains: 510 pass. Not covered.

The restored variant_types assertion (one `variant_type` node) fails under whole-view retention, so it guards the over-retention.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium   | The frame-subject half of the retained set (written field owners, `creates`, `deletes`) has no test. With it removed, every qsl-package lib test still passes. The removal is a real regression: scratch probes of an operation whose frame modifies `Gauge/d`, creates `Gauge` or deletes `Gauge`, in a unit with no clause naming `Gauge`, read back `Verified` at the PR head but are refused `MissingDeclaration` (`MissingName`) by IR's v2 reader with the chains removed. That is the same refusal class #2119 fixed. Add a TC-1286 test for such an operation. | qsl-semantics/src/check/lowering/model.rs:436-452 |
