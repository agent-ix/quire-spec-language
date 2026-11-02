---
id: SR-202611020539
title: "Code review — QSL-1461 explicit Text literals"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-cst/src/grammar.rs, qsl-forms/src/value.rs, qsl-package/src/emit/tests.rs, qsl-semantics/src/check/, qsl-semantics/tests/it/native_literals.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/spec/spec
    type: references
---

## Summary

Reviewed PR #2084 against QSL-1461's Text literal acceptance criteria, the Rust implementation and the test evidence. Parsing, checking, typed refusal, lowering and checked-package readback are covered; the required Text evaluation path and one specified lower-bound refusal vector are not covered by the added tests.

## Verdict

**CONDITIONAL** — the checked value and wire path look sound, but the PR lacks evidence for required evaluation and refusal cases.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium | No test evaluates a source `Text(...)` body, so TC-1052's required evaluation behavior remains unverified. | qsl-semantics/tests/it/native_literals.rs:355; spec/test-cases/TC-1052-native-string-uuid-and-timestamp-literals.md:46 |
| FND-002 | medium | Required Text refusal vectors are incomplete: the tests omit the specified empty literal below a positive minimum and `Text(5)`. | qsl-semantics/tests/it/native_literals.rs:381; spec/functional/FR-056-admit-domain-package-model-declarations.md:977 |

## Review Evidence

- Independently inspected every changed Rust file and the relevant callers, including the native constructor resolution, type hinting, normalization, bounds refusal, lowering, protocol cause mapping, and checked-package v2 readback.
- `git diff --check origin/main...HEAD` and `cargo fmt --all -- --check` passed.
- The changed Rust behavior is represented by one checker/lowering integration test and one emitter/readback test. No `Text("...")` source evaluation test exists under `qsl-replay`.
- The required `Text[1, 8; nfc]` with `Text("")` lower-bound refusal and `Text(5)` argument refusal are stated in the reviewed acceptance criteria but absent from the added refusal vectors.
- Rust lane: reviewed with the repository's Rust style and review checks. The PR changes no CI workflow and introduces no public API surface.
- Focused locked `native_literals` tests passed (12/12). The locked checked-package readback test was started but remained silent during its long first compile and was interrupted at the lead’s handoff; no result is claimed. `cargo clippy` and `cargo deny` were not run independently. Quoin matrix likewise returned no output during the available review window and was interrupted; the gap-analysis artifact records a manual criteria-to-test check and this limitation.

## Acceptance Criteria Examined

- **FR-056-AC-51** — “Check and evaluate” the Text bodies; both nfc spellings retain U+00E9, Unicode-scalars keeps composed/decomposed distinct, retained length controls bounds, binary-utf8 counts bytes.
- **FR-056-AC-52** — typed `TextLiteralOutsideBounds` cause and call location; empty text below minimum; refusal in non-Text positions and no implicit conversion.
- **FR-091-AC-69** — Text call parsing and checking, String argument rejection, arity, invalid escape, ambiguity and bounds refusal.
- **FR-093-AC-50** — typed Text literal lowering, retained value and Text-profile equality lowering.
- **TC-1052 steps 7–9** — Text evaluation, refusal vectors and checked IR nodes.

## Dispositions

Not applicable on the initial review pass.
