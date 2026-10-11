---
id: SR-7806
title: "Code review of QSL-671 PR 705 specification delta"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@d9d3a43352c09239e8fc2ecc89c3f26d7a963d6b; spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md, spec/test-cases/TC-917-evaluator-invariant-producers-have-explicit-classifications.md, spec/tests.md, qsl-eval/src/value/expression/evaluate.rs, Cargo.lock"
review_set: subset
---

## Summary

Reviewed the exact spec delta and traced its new caller and dependency-source claims. The Rational caller proof matches the locked kernel source, but the new normative verification still anchors itself to a pinned upstream revision through Cargo.lock.

## Verdict

**FAIL** — the changed specification newly directs verification through a locked upstream git revision, which the repository policy prohibits.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new inspection obligation anchors its proof to the `quire-exact` git revision selected by Cargo.lock, preserving a pin-based source check after replacing the explicit revision URL. | spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md:531 |
