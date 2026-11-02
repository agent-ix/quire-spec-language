---
id: SR-202611020540
title: "Gap analysis — QSL-1461 explicit Text literals"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; repository Test Matrix, code-to-spec reverse coverage, and QSL-1461 changed paths"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/spec/spec
    type: references
---

## Summary

Attempted the repository's computed Test Matrix and inspected the QSL-1461 changed paths for unowned behavior and stubs. The optional semantic review was not requested and was skipped; the planless audit does not assess plan completion.

## Verdict

**CONDITIONAL** — the Quoin matrix did not return in the review window; manual criteria-to-test inspection found missing evidence for Text evaluation and required refusal vectors.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium | Text evaluation and required refusal vectors lack tests; the Quoin matrix could not complete in this review window. | FR-056-AC-51, FR-056-AC-52, TC-1052 |

## Coverage

- Matrix: `quoin matrix --repo . --json` and retry with `--no-project-config` both produced no output through the review window; interrupted at lead handoff, so matrix status is unavailable. Manual acceptance-to-test check completed: the traced integration test checks the checked graph but does not evaluate Text; the bound-refusal cases omit `Text("")` below a positive minimum and `Text(5)`.
- Reverse coverage and stubs: inspected changed production code and its requirement references; no unowned behavior or stub found.
- Semantic review: skipped (optional; no opt-in).
- Plan completion: not assessed.


## Dispositions

| FND | Outcome | Reason |
| --- | ------- | ------ |
| FND-001 | fixed | New replay evaluation coverage and both omitted refusal vectors directly address the manual criteria-to-test gap. |

### Round 1 evidence

- `qsl-replay/tests/it/compile_text_literals.rs::explicit_text_bodies_evaluate_with_profile_normalization_and_equality` now compiles source Text calls and evaluates retained values and equality for nfc and unicode-scalars.
- `qsl-semantics/tests/it/native_literals.rs` adds the empty literal below a positive minimum and `Text(5)`, checking the typed refusal and body location.
- Focused locked replay evaluation test passed (1/1); focused locked `native_literals` tests passed (12/12). Formatting and diff checks passed.
- Historical limitation: Quoin matrix did not return during the initial review window; the disposition is based on the specific manual acceptance-to-test finding, not a claim that the original matrix completed.
