---
id: SR-7808
title: "Integrity review of QSL-671 PR 705 spec delta"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@d9d3a43352c09239e8fc2ecc89c3f26d7a963d6b; spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md, spec/test-cases/TC-917-evaluator-invariant-producers-have-explicit-classifications.md, spec/tests.md"
review_set: subset
---

## Summary

The new criteria distinguish present caller inspection from future API-adoption cleanup, and TC-917 carries both obligations. Integrity review found a high-severity policy conflict: the present inspection proof is tied to the exact upstream git source revision in Cargo.lock.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new inspection obligation anchors its proof to the `quire-exact` git revision selected by Cargo.lock, preserving a pin-based source check after replacing the explicit revision URL. | FR-090-AC-19 |
