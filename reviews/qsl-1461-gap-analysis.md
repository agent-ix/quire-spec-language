---
id: SR-202611021113
title: "Gap analysis — QSL-1461 updated IR and vector coverage"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL Text literal requirements, trace tags, implementation and focused tests in qsl-cst, qsl-forms, qsl-semantics, qsl-package, and qsl-replay"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/spec/spec
    type: references
---

## Summary

Audited QSL-1461's changed source and tests against the Text-literal criteria and checked reverse coverage and stubs in the touched paths. The scoped `quire matrix` produced no output through the review window and was interrupted, so the findings below come from manual acceptance-to-test inspection.

## Verdict

**CONDITIONAL** — focused coverage exists for source parsing/checking, normalization, lowering, evaluation, and strict readback, but one specified conflict outcome is contradicted and one required evaluation profile is untested.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium | The `function Text` case is traced to FR-091-AC-69 but asserts syntax rejection where the criterion requires an ambiguous-name refusal. | qsl-semantics/tests/it/native_literals.rs:381 |
| FND-002 | medium | TC-1052 step 7 requires binary-utf8 source evaluation alongside nfc and unicode-scalars; the replay evaluator test covers only the latter two. | qsl-replay/tests/it/compile_text_literals.rs:96 |

## Coverage

- Static matrix: attempted `quire matrix --scope . --format tsv`; it remained silent and was interrupted. Matrix status is unavailable. The semantic, forms and replay test sessions also produced no final output, with no visible subprocess remaining; their status is unavailable.
- Manual criterion mapping: FR-056-AC-51/52, FR-091-AC-69 and FR-093-AC-50 have trace tags and focused tests. FR-091-AC-69's tagged `a_function_cannot_be_named_text` test proves behavior opposite to its ambiguity requirement. TC-1052 step 7 lacks binary-utf8 evaluation.
- QSpec TX vectors: source emission and strict checked-package reader tests cover all 15 vectors; focused package tests passed.
- Reverse coverage and stubs: inspected the changed production paths; no unrelated unowned behavior or live stub found.
- Semantic review beyond the listed acceptance mismatches: skipped as optional and not requested.
- Plan completion: not assessed.

## Dispositions

Original verdict: **CONDITIONAL**. Round 1 verified both findings fixed at PR head `4d8d436007ef8c19e4f3ad4c527eff6916af2943` against `origin/main` `1a4b6f3dec85ce571573762afc5e9cec5b3baff7`.

| FND | outcome | reason |
| --- | ------- | ------ |
| FND-001 | fixed | `a_function_named_text_makes_the_constructor_call_ambiguous` parses the declaration and asserts the typed ambiguous-name cause and both loci. |
| FND-002 | fixed | `explicit_text_bodies_evaluate_with_profile_normalization_and_equality` now covers binary-utf8 composed/decomposed values and both equality outcomes. |
| FND-001 | fixed | Round 2 confirmed `function_like` preserves the reserved `Text` token as a declaration name, enabling the tagged semantic ambiguity case. The focused forms case passed locally; the lead reports semantics integration 20/20, including this case. |
| FND-002 | fixed | Round 2 confirmed binary-utf8 source value and equality cases are present; the lead reports the replay evaluator test passed 1/1. |
| FND-001 | fixed | Round 3 confirms the mapper fix and ambiguous-name test are unchanged at the requested head. |
| FND-002 | fixed | Round 3 confirms binary-utf8 evaluator coverage is unchanged at the requested head. |
| FND-001 | fixed | Round 4 confirms the rebased forms mapper retains the `Text` declaration and the ambiguity test remains. |
| FND-002 | fixed | Round 4 confirms the binary-utf8 evaluator vectors remain in the final diff. |
| FND-001 | fixed | Round 5 confirms the rebased mapper and ambiguity test still cover the required branch. |
| FND-002 | fixed | Round 5 confirms the binary-utf8 evaluator vectors remain in the rebased diff. |

Round 2 verification target: PR #2084 HEAD `f26190307e787b405e7cc769ca77e9331ad91f22`, against latest `origin/main` `ac5d048b3a848ad18012f116ae0daef285087989`. The lead reports package vectors 5/5 and affected-package all-features Clippy passed. Local forms focused test passed (1/1). The static Quire matrix remains unavailable from the prior pass; scoped findings were manually rechecked at this head.

Round 3 verification target: PR #2084 HEAD `ff07ffaf12bcc4635915c5e6d31c970dab698787`; the currently fetched `origin/main` is `b81c0ca482ff34add347c09540088c06f63b3336`. The review-relevant Text tests and mapper implementation are unchanged since round 2. The branch merge base is `ac5d048b3a848ad18012f116ae0daef285087989`; origin/main has since advanced by QSL-1465 commits that are not in this head and touch nearby semantics/package code. The lead reports all-features Clippy and focused Text tests passing on this head: package 5/5, forms 4/4, semantics 20/20, plus filtered CST and replay suites.

Round 4 verification target: PR #2084 HEAD `87638411ac6606748c9e0a78f807768887714507`, based on `b81c0ca482ff34add347c09540088c06f63b3336`; that was the requested current main revision. During this review fetch, `origin/main` advanced to `ece188a937805a15ce355b10ccfac3445bea3aad` (QSL-608), so the branch is based on b81 rather than the newly fetched ece188. The final diff retains both Text fixes and their tests. The lead reports affected-package all-features Clippy and focused Text tests passed on this head: package 5/5, forms 4/4, semantics 20/20, and filtered CST/replay suites.

Round 5 verification target: PR #2084 HEAD `ebc00d70afd03de54bfa453a38888a9c953cc0b1`, based on latest `origin/main` `ece188a937805a15ce355b10ccfac3445bea3aad` (merge-base equals ece188). Compared the review-specific mapper and test files against round 4; no changes affect either finding. The lead reports affected-package all-features Clippy and QSpec-backed filtered tests passed after this rebase: package 5/5, forms 4/4, semantics 20/20, and filtered CST/replay suites.
