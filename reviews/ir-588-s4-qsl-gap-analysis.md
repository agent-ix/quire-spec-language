---
id: SR-16277
title: "Gap analysis of quire-spec-language PR #1382: FR-403 S4 dispositions (IR-588)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@571dfa7005fd8d75134224e48fdb8f58291a16cf; FR-403-AC-2, AC-8, AC-9, AC-11 and the S4 disposition table (spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md); TC-1121 context; tests qsl-eval/tests/it/source_call.rs, qsl-eval/src/lib.rs doctests, qsl-semantics/tests/it/model_population.rs; manual AC-to-test check"
review_set: subset
date: '2026-10-23'
---
# Gap analysis of quire-spec-language PR #1382

## Summary

Ticket: IR-588 (S4). `quoin matrix --json` (quoin 0.28.3) ran past 500 s on
this repository and was stopped, so this is a manual AC-to-test check over
the criteria the PR claims. Plan completion: not assessed.

Examined:

- **FR-403-AC-2, `mod` half.** `modulo_through_the_evaluator_matches_the_quire_eval_operation`
  (trace FR-403-AC-2, TC-1121) runs `-7 mod 2` through the reference
  evaluator. It asserts the value and that the admitted charge points end with
  those of `quire_eval::modulo` called directly on a fresh meter, and that the
  direct list is non-empty. It passes. The `compile_fail` half covers
  `value::supplied::SuppliedRun`, `model::refusal::render_identity` and
  `value::modulo` (SR-16276 FND-004 is about their missing error code). The
  `divide` half is blocked on FR-400 / X-13, as recorded: `AdmittedIntegerDivision`
  is still a qsl-semantics type, and `quire-checked` is an empty skeleton.
- **FR-403-AC-8.** `the_subsetting_refusal_names_the_lowest_violating_member_in_any_listing_order`
  can fail: listing 2 puts `a1` before `a0`, so an admission that walked
  document order would name `a1`. Admission walks `admitted` in key order
  (population.rs:1504) and reads the `HashMap` by key only. The test passes
  under `--features test-support`, the gate its module is behind. FND-001 is
  about how far it matches the AC's wording.
- **FR-403-AC-9 (inspection).** qsl-eval re-exports no quire-eval item, and
  qsl-eval's manifest has no `std` feature.
- **FR-403-AC-11.** `FamilyKind::StateModel.catalog_code_prefix()` returns SV's
  constant (family/mod.rs:127), and qsl-eval defines no `StateModelUndefined`,
  `PreconditionFailure`, `ReferenceUnresolved` or `ProtocolReferenceUnsupported`.
  `NavigationRefusal` stays, blocked on `DeclarationKey` (recorded). The
  "raised by each evaluator" half waits for the no_std evaluator (S6). The
  prefix is already asserted by the existing `state_model_family.rs` and
  `dispatch_calls.rs` tests, and the rendering by SV's own tagged tests.

## Verdict

**Coverage is adequate for what S4 can deliver.** The one gap is low: the
AC-8 test's fixture does not match the AC's member count.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-403-AC-8 says the document's "three members are listed in two different orders, with a subsetting violation on two of them". The test admits four members (`a0`..`a3`), with `a0` and `a1` violating and `a2`/`a3` as subset targets. It asserts the intended property, but a reader cannot match the fixture to the AC. Fix one of them: either the AC says "members" (or "four"), or the fixture uses three members. | qsl-semantics/tests/it/model_population.rs:2235-2284 |

## Dispositions

Round 1 was reviewed at b5e0c0d03, and the fix commit is 31f5d6c09. I mutation-checked it: I changed `admit_binding`'s subsetting walk to document order and ran the test. It failed on listing `["a3", "a2", "a1"]` (left `"a2"`, right `"a1"`), then passed after the change was reverted. So the test detects a document-order walk.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed 31f5d6c09 | The fixture now has three members. `a1` and `a2` each violate subsetting, `a3` is plain, and the two listings are `[a1, a2, a3]` and `[a3, a2, a1]`. The test expects `a1`, which matches FR-403-AC-8. |
