---
id: SR-16277
title: "IR-586 S2 PR 1384 gap analysis: FR-400 ACs touched by the DeclarationKey and AdmittedIntegerDivision move"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ae001482e28a98797037954c417fdaebc50bd6ca; diff ir-588-s4-qsl...ir-586-s2-qsl; spec/functional/FR-400-separate-checked-terms-from-the-checker.md, FR-060, FR-059, FR-401, FR-403, NFR-015"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-400
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: references
---

## Summary

Ticket: IR-586. Planless gap analysis over this PR's diff only. Plan
completion: not assessed.

FR-400 criteria this PR touches:

- **AC-5** (`qsl_semantics::model::key::DeclarationKey` fails to compile):
  covered. The `compile_fail` doctest is in `model/key.rs`, with its
  `no_run` pair. The `AdmittedIntegerDivision` old paths (`value::` and
  `value::definition::`) have the same pairs in `value/definition.rs`.
- **AC-1** (TC-390: only `qsl-semantics` enables `construct`): the manifest
  state is correct, but no test checks it.
- **AC-3** (T12-G passes on the QSL tree and refuses a fixture
  `construct` call): absent. This PR adds the first `construct` call site.
- **AC-9** (no normal dependency enables `quire-checked/test-support`): the
  manifest state is correct, but the existing guard does not see
  `quire-checked`.

FR-059 is unaffected: `graph::classify` ignores `quire-checked`, and the
backend `construct` rule (AC-3, second clause) has no RT, IR or CG edge to
check yet. FR-401, FR-403 and NFR-015 were read for context only.

## Verdict

CONDITIONAL. AC-5 is met. AC-1, AC-3 and AC-9 have no test. The first two
matter now, because this PR creates the `construct` call site they guard
(see SR-16276 FND-001). If PR B is planned to deliver T12-G and TC-390,
the ticket must say so explicitly.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-400-AC-3 and AC-1 have no test, although this PR adds the first `quire_checked::construct` call. No T12-G rule exists in `tools/arch-lint/api_surface.rs`, and no TC-390 test asserts that only `qsl-semantics` enables `construct`. | qsl-semantics/src/value/definition.rs:697; tools/arch-lint/api_surface.rs | correct-requirement-no-evidence |
| FND-002 | medium | FR-400-AC-9's QSL clause is untested for `quire-checked`. `no_shipped_dependency_enables_test_support` only checks workspace crates. | tests/it/family_outcome_layering.rs:639-645 | correct-requirement-no-evidence |
