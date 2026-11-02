---
id: SR-202611021112
title: "Code review — QSL-1461 updated IR and vector coverage"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-cst/src/grammar.rs, qsl-forms/src/value.rs, qsl-forms/tests/it/value_forms.rs, qsl-semantics/src/check/, qsl-semantics/tests/it/native_literals.rs, qsl-package/src/emit/tests.rs, qsl-replay/src/spine/refinement.rs, qsl-replay/tests/it/compile_text_literals.rs, Cargo.lock"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/spec/spec
    type: references
---

## Summary

Reviewed PR #2084 at head `3d85b5e65c0b2fa870b97b06f4dbfdcbf7922fb1` against current `origin/main` `f2ce9b52c39803e9be1b10faad42bb04159a5ca5`. The updated IR relock and checked-package TX vectors are covered by focused tests; one explicit FR-091 acceptance path conflicts with the parser behavior, and TC-1052's binary-utf8 evaluation vector remains absent.

## Verdict

**CONDITIONAL** — the new source, evaluation, and strict-reader coverage is substantial, but the named function conflict and binary-profile evaluation requirements remain unresolved.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium | FR-091-AC-69 requires `function Text` to parse and make a Text constructor use ambiguous, but the new test requires the declaration to be rejected as syntax. | qsl-semantics/tests/it/native_literals.rs:381 |
| FND-002 | medium | The new source evaluation test covers nfc and unicode-scalars but omits TC-1052 step 7's binary-utf8 evaluation case. | qsl-replay/tests/it/compile_text_literals.rs:96 |

## Review Evidence

- Inspected the full `origin/main...HEAD` diff (17 changed files), including the IR lock update, parser/forms changes, checker and lowering, replay limit classification, emission, refusal and readback vectors, and new source evaluation tests.
- `git diff --check origin/main...HEAD` and `cargo fmt --all -- --check` passed.
- Locked QSL package tests with `QSPEC_DIR=/home/peter/dev/worktrees/qspec-ci-current` passed: 3 Text literal tests, including all 15 source vectors and all 15 strict-reader vectors.
- The semantic, forms and replay test sessions produced no final output, and their subprocesses were no longer visible; their status is unavailable. No further builds were started so the parent lane can run its focused checks.
- No CI workflow files changed. The PR's GitHub status checks currently show only CLA checks.
- Rust implementation review found the `TextLiteralOutsideBounds` replay mapping correctly returns no `ReachedLimit`; it stays an ordinary typed refusal.

## Acceptance Criteria Examined

- **FR-056-AC-51** — contextual Text values, profile normalization/equality, and scalar versus UTF-8 byte bounds.
- **FR-056-AC-52** — typed bounds refusal, empty input below minimum, non-Text result/operand positions, and no implicit conversion.
- **FR-091-AC-69** — source Call shape, String argument restriction, arity, invalid escape, Text-name ambiguity, and bounds refusal.
- **FR-093-AC-50** — retained value, body/semantic type, equality law/mode/arguments, and shared literal node.
- **TC-1052 steps 7–9** — source evaluation under nfc, unicode-scalars, and binary-utf8; refusal vectors; lowering and reader behavior.
- **QSpec FR-322-AC-304, TX-01..TX-14 and TX-13s** — source normalization/bounds plus strict checked-package reader decisions.

## Dispositions

Original verdict: **CONDITIONAL**. Round 1 verified both findings fixed at PR head `4d8d436007ef8c19e4f3ad4c527eff6916af2943` against `origin/main` `1a4b6f3dec85ce571573762afc5e9cec5b3baff7`.

| FND | outcome | reason |
| --- | ------- | ------ |
| FND-001 | fixed | The new regression test now parses `function Text` and asserts `CheckCause::AmbiguousName`, name `Text`, two loci, and code `ambiguous_declaration`. |
| FND-002 | fixed | The replay test now checks and evaluates composed and decomposed source Text bodies and profile-aware equality under `binary-utf8`, as well as the other two profiles. |
| FND-001 | fixed | Round 2 verified the follow-up forms mapper change preserves the reserved `Text` token as a declaration identifier; the focused forms test passed locally and the lead reports semantics text-filter integration 20/20, including the ambiguity case. |
| FND-002 | fixed | Round 2 rechecked binary composed/decomposed value and equality cases at the current head; the lead reports replay evaluator 1/1. |
| FND-001 | fixed | Round 3 confirms the declaration-name mapper and ambiguity assertion are unchanged from the reviewed passing head. |
| FND-002 | fixed | Round 3 confirms binary-utf8 evaluator cases remain unchanged from the reviewed passing head. |
| FND-001 | fixed | Round 4 confirms the declaration mapper and typed ambiguity assertion at the rebased PR head. |
| FND-002 | fixed | Round 4 confirms binary-utf8 source value and equality assertions remain present at the rebased PR head. |
| FND-001 | fixed | Round 5 confirms the declaration mapper and ambiguity regression test are unchanged after rebasing onto QSL-608. |
| FND-002 | fixed | Round 5 confirms binary-utf8 evaluation and equality coverage is unchanged after rebasing onto QSL-608. |

Round 2 verification target: PR #2084 HEAD `f26190307e787b405e7cc769ca77e9331ad91f22`, against latest `origin/main` `ac5d048b3a848ad18012f116ae0daef285087989`. The lead reports package vectors 5/5 and affected-package all-features Clippy passed. Local forms focused test passed (1/1). A local semantics rerun was cancelled after the locked build made no further output while another lane was active.

Round 3 verification target: PR #2084 HEAD `ff07ffaf12bcc4635915c5e6d31c970dab698787`; the currently fetched `origin/main` is `b81c0ca482ff34add347c09540088c06f63b3336`. The review-relevant Text tests and mapper implementation are unchanged since round 2. The branch merge base is `ac5d048b3a848ad18012f116ae0daef285087989`; origin/main has since advanced by QSL-1465 commits that are not in this head and touch nearby semantics/package code. The lead reports all-features Clippy and focused Text tests passing on this head: package 5/5, forms 4/4, semantics 20/20, plus filtered CST and replay suites.

Round 4 verification target: PR #2084 HEAD `87638411ac6606748c9e0a78f807768887714507`, based on `b81c0ca482ff34add347c09540088c06f63b3336`; that was the requested current main revision. During this review fetch, `origin/main` advanced to `ece188a937805a15ce355b10ccfac3445bea3aad` (QSL-608), so the branch is based on b81 rather than the newly fetched ece188. The final diff retains both Text fixes and their tests. The lead reports affected-package all-features Clippy and focused Text tests passed on this head: package 5/5, forms 4/4, semantics 20/20, and filtered CST/replay suites.

Round 5 verification target: PR #2084 HEAD `ebc00d70afd03de54bfa453a38888a9c953cc0b1`, based on latest `origin/main` `ece188a937805a15ce355b10ccfac3445bea3aad` (merge-base equals ece188). Compared the review-specific mapper and test files against round 4; no changes affect either finding. The lead reports affected-package all-features Clippy and QSpec-backed filtered tests passed after this rebase: package 5/5, forms 4/4, semantics 20/20, and filtered CST/replay suites.
