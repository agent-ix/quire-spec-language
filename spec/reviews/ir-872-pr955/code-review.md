---
id: SR-12804
title: "Changed artifact review of QSL PR 955"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #955; FR-123 Outputs/Behavior/AC-1..4, TC-518 Steps/Expected Results, QSV FR-121 and QSL FR-129 context, five changed self-review artifacts"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: reviews
---

## Summary

The FR and TC changes are internally consistent and the merged-tree spec gate passes. Three changed author self-review artifacts put affirmative no-defect statements in Findings rows, which the reviewer data contract treats as defects.

## Verdict

CONDITIONAL — correct the three affirmative Findings rows in the changed self-review artifacts.

## Scope and evidence

| Unit | Role | Evidence |
| --- | --- | --- |
| FR-123 Outputs and Behavior | examined | QSV-owned shared types and EffectiveId/name operation pair; QSL resolution, defaults, first-occurrence order and classification remain explicit |
| FR-123-AC-1 through AC-4 | examined | AC-1 adds distinct declaring-type identities; AC-2 through AC-4 are unchanged |
| TC-518 | examined | Step 1 and expected result compare same-named operations across two model types |
| QSV FR-121 | context_only | QSV PR #44 defines FairnessOperation as declaring-type EffectiveId and name, plus the other shared values |
| QSL FR-129 and ADR-011 X-11 | context_only | QSL owns strong-kind defaults and checker behavior; ADR places shared values below QSL |
| Changed author self-review artifacts | examined | Five artifacts checked for alignment with the changed requirement and reviewer data contract |

Merged-tree validation: `make check-spec-validation` exited 0 on scratch merge of QSL main `33066f7656e7091eaafe015f0a8b645aecd09508` and PR head `ab18f7e3cdc8567a90810fd28860aa6ceedb2b1f`. Existing warnings remain. Separate `cargo deny --workspace check bans --config deny.toml` exits with “unexpected argument '--config'”; this is the unchanged QSL Makefile command, so the cargo-deny lane is not established as passing by this review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Three changed author self-review artifacts record “No ... found” as Findings instead of clean placeholders; this makes positive results look like defects to review-data consumers. | spec/reviews/qsv-fairness-reference/dependency.md:25; integrity.md:28; scope-boundary.md:25 |
