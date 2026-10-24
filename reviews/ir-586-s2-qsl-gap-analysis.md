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

## New findings (disposition pass 1)

Re-review at `182875cb5810b62451c4be2643bcda0e8893810b`. `quire matrix` binds FR-400-AC-1 to TC-390's two `construct` tests, AC-3 to four arch-lint tests, AC-9 to `no_shipped_dependency_enables_test_support`, and AC-5 to one xtask test.

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-003 | medium | FR-400-AC-5 shows as `tagged` only through `tc_747_quire_checked_declaration_key_copy_is_flagged`. That test checks the canonical-types `copy` rule (FR-272-AC-2), not AC-5's claim that `qsl_semantics::model::key::DeclarationKey` fails to compile from another crate. The `compile_fail` doctests in `model/key.rs` that do verify AC-5 carry no binding, so the matrix overstates coverage. Fix: drop `FR-400-AC-5` from the xtask test's `#[trace]`, and bind the doctest evidence through a test the matrix can see, for example a trybuild or `compile_fail` wrapper test tagged FR-400-AC-5. | xtask/src/canonical_types.rs:1188-1189; qsl-semantics/src/model/key.rs:16-25 | correct-requirement-no-evidence |

## Dispositions

Disposition pass 1 at head `182875cb5810b62451c4be2643bcda0e8893810b`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-400-AC-1: `only_qsl_semantics_enables_the_construct_feature` and `a_crate_other_than_qsl_semantics_enabling_construct_is_named` (tests/it/family_outcome_layering.rs). FR-400-AC-3: `tc_arch_lint_api_surface_029`/`_030`, `tc_arch_lint_direction_010` and `tc_arch_lint_metadata_010`. All pass locally. AC-3's `PackageId`-from-`model` clause waits for the `PackageId` move, which is a later X-13 item. |
| FND-002 | fixed | `no_shipped_dependency_enables_test_support` now guards `quire-checked` and carries `FR-400-AC-9`. It passes locally. |
