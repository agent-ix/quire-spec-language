---
id: SR-1285
title: "Gap analysis of PR #625: FR-056-AC-2 numbers with no finite double (QSL-219 part 2)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@25610e30b8e443aa7b665d3880145b1b9c2d858a; PR #625 diff against origin/main: qsl-semantics/src/model/intake.rs, qsl-semantics/src/model/observation.rs, spec/functional/FR-056-admit-domain-package-model-declarations.md, spec/test-cases/TC-145-admit-ir-2-domain-package.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
---
# Gap analysis of PR #625

## Summary

Ticket: QSL-219 (part 2). PR: quire-spec-language#625. A manual check of
each AC against its tests. There is no plan bundle.

FR-056-AC-2 (amended) claims, each traced to the test that covers it:

- **`1e400`, `-1e400` give `inexact-integer` with `document_pointer`, at any
  depth, through intake.** Covered by
  `refuses_a_number_with_no_finite_double_by_its_lexeme` (top level,
  `/package/count`, and `/a~1b/1/c~0d`). Binding correct.
- **`1e-400` gives `inexact-number` through intake.** Covered by the same
  test, by way of the #617 tree path. Binding correct.
- **The same through admission under any digest, never `stale_dependency`.**
  Covered for FR-106 observation admission by
  `a_number_with_no_finite_double_refuses_noncanonical_wire_under_any_digest`
  (raw digest and zero digest). Not covered for FR-154 `admit()`
  (intake.rs:996), which is FR-056's own admission (FND-001).
- **No longer malformed.** `refuses_a_number_serde_json_cannot_represent_at_the_one_parse`
  was updated. Binding correct.

TC-145 step 3 names "admission under their raw digest and under another
digest". The digest tests' `#[trace("TC-145", "FR-056-AC-2")]` bindings are
correct for what they exercise.

## Verdict

Approve with findings. One coverage gap against AC-2's admission clause.
The non-whole-overflow gap is recorded in SR-1284 FND-002.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-056-AC-2 says `1e400`/`-1e400`/`1e-400` refuse "through admission under any digest, never `stale_dependency`". No test calls FR-154 `admit()` with these numbers. The tests cover `PackageDocument::parse` and observation's `check_document_digest` only. `admit` passes `NoncanonicalNumber` through (intake.rs:1054-1065), and AC-13's u64-bound test already covers that pass-through, so the risk is small. Fix: add `admit()` cases for `1e400` under its raw digest and under another digest, asserting the `noncanonical_wire` refusal. | qsl-semantics/src/model/intake.rs:1054-1065, spec/functional/FR-056-admit-domain-package-model-declarations.md:378 |
