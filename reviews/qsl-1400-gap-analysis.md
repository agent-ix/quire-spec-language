---
id: SR-2026111015000000000010
title: "gap-analysis of PR 2676 source object types against FR-569 and TC-1408"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR 2676 tests: qsl-forms/tests/it/object_forms.rs, qsl-semantics/tests/it/source_objects.rs, tests/it/source_object_emission.rs, qsl-semantics/tests/it/operation_bodies.rs"
review_set: subset
---

## Summary

Ticket: QSL-1400. `quoin matrix --repo .` (quoin 0.30.0) reports 4559 of 4676 criteria repo-wide as untagged, so it does not read this repo's `#[trace]` tags. A manual AC-to-test check replaced it. AC-6 to AC-8 wait on QC-30, and AC-9 and AC-10 are deferred by the lead; neither absence is a finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Trace tags overclaim. FR-390-AC-73 needs E-S slots evaluated and fixture E views unaffected, which no test does. FR-390-AC-70 and FR-569-AC-11 need fixture S, S-R, S-P, S-M and E-S, and the test uses a 2-object mini unit. FR-383-AC-67 is tagged on a protocol test with no E-S and no S4. The AC-11 test never checks that the refusal span is at `object Order`, so moving the span to another object passes. | tests/it/source_object_emission.rs:74 |
| FND-002 | medium | AC-4's span is unchecked. `a_declaration_check_refuses_with_its_code_at_the_offending_name` binds `_at` and never uses it, so 6 of 8 cases check only the code. The `extends M::Order` and `scaled(m: Text..)` `variance-parameter` cases have no test, although the probe shows the code refuses both correctly. | qsl-semantics/tests/it/source_objects.rs:557 |
| FND-003 | medium | The S-M protocol test cannot tell the source `Order` from `M::Order`. It asserts only `Seller.context.is_some()` and `model_selection == None`, and it never compares the `over` context for `Order` against `M::Order`. A wrong resolution to the model type passes, so FR-218-AC-39's "neither resolves to the other" is untested. | qsl-semantics/tests/it/source_objects.rs:732 |
| FND-004 | medium | AC-5 is partly asserted. `table_shapes` compares candidate indices only. The order "Qs2 or Qs, own first" is unchecked, since counts are [1, 2]. Not checked: `r.size()` over `Reference<Sub>` versus `Reference<Order>`, the `deref(s).total` member node, the `allInstances` type, and the bound type. The S-R' test redefines `bump` on a new `Wide`, not on `Sub`, and never runs D-R'. AC-2's model-side case (`on Order` and `on M::Order` against different fields) has no test. | qsl-semantics/tests/it/source_objects.rs:637 |
| FND-005 | low | Test hygiene. `let _ = AssemblyCause::EmptyUnitList;` is dead code. Fixture R is written `>=`, a weaker form of the spec's `= pre+1` (see spec problem 8). `the_model_twin_checks_the_clauses` has no trace and no assertion beyond the check succeeding. | qsl-semantics/tests/it/source_objects.rs:175 |

## Scope and evidence

- FR-569-AC-1 (object_forms, 4 tests): examined, adequate.
- FR-569-AC-2 and AC-3: partial, see FND-003 and FND-004 here and FND-001 in SR-2026111015000000000009.
- FR-569-AC-4: see FND-002.
- FR-569-AC-5: see FND-004.
- FR-569-AC-11, FR-218-AC-39, FR-383-AC-67, FR-390-AC-70 and FR-390-AC-73: see FND-001 and FND-003.
- Spec problems the lead raised: the verdicts are in the reviewer report and the Linear comment.

## Verdict

Several AC tests would pass against a wrong implementation, mostly through unchecked spans and source-versus-model identity. Trace tags claim criteria that the tests do not exercise.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | AC-11 names fixtures S, S-R, S-P, S-M and TC-1407 E-S, but the S4 tests run only S, S-R and S-P. The model-plus-object-plus-protocol shape is covered by `a_source_object_refusal_wins_over_a_refused_protocol`, but literal S-M and E-S never reach S4. | qsl-package/src/emit/tests/source_objects.rs:37 |
| FND-007 | low | TC-1408's S-M fixture writes `over (view: M::Order)`, while FR-218-AC-39, which says it runs "in TC-1408 variant S-M", checks `over (view: Order)`. The test covers both spellings, but TC-1408 states only one. Fix TC-1408's S-M text to name both `over` spellings. | spec/test-cases/TC-1408-declare-object-types-populations-and-operations-in-source.md:58 |
| FND-008 | low | The new unit-scoping rule covers "a clause, role or `over`", but the test covers only a state clause. Nothing checks that a secondary unit's own clause resolves the type that unit declares; that unit only appears as the one that fails to resolve. | qsl-semantics/tests/it/source_objects.rs:1033 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-390-AC-70 and FR-390-AC-73 were dropped from the emission test, and FR-383-AC-67 from the protocol test. AC-11's span is now asserted exactly for S, S-R and S-P (qsl-package source_objects.rs). S-M and E-S remain (FND-006). |
| FND-002 | fixed | All 8 cases assert exactly one refusal at the exact offset. `extending_a_model_type_is_refused_at_the_entry` and `a_redefinition_with_a_narrower_parameter_refuses_variance_parameter` were added. The spec now says "on the member `Sub.scaled`". |
| FND-003 | fixed | `over (view: Order)` reads `view.extra`, a field only the source type has, while `over (view: M::Order)` refuses on it and checks without it. The source and model contexts are now distinguished. |
| FND-004 | fixed | The test compares receiver names, the own-first `Qs2 or Qs` shape, and the checked types of Cap, Every, SizedSub and totalOf on both routes. D-R' runs. AC-2 has its field-set test. `self.size()` replaces `r.size()`, matching the spec edit. |
| FND-005 | fixed | The dead line and the model-twin smoke test are gone. R reads `self.total > pre(self.total)`, matching the edited TC-1408. |
| FND-006 | still-open | S-M now runs through S4 (`fixture_s_m_refuses_at_the_first_object`). AC-11 also names TC-1407 fixture E-S (`object Node`), and no test in the repo builds E-S or runs it through S4. Add an E-S S4 refusal test, or have the lead record a deferral with a reason. |
| FND-007 | fixed | TC-1408's S-M text now adds "the same protocol with `over (view: Order)` (FR-218-AC-39), whose model context is the source `Order`", consistent with FR-218-AC-39 and the test. |
| FND-008 | fixed | `a_role_and_an_over_see_only_their_own_units_source_types` refuses another unit's role and `over` on `Order` with `missing-name`. In the same test, secondary unit b's own clause `Y` and protocol on `Order` check. `a_source_population_is_visible_only_in_the_unit_that_declares_it` covers state clauses; the temporal gap is code-review FND-009. |
| FND-006 | deferred | Lead deferral: TC-1407 fixture E-S declares `total: M::Total`, a model-typed field that slice 2 of FR-569 adds, so E-S cannot be built in this slice. Its S4 refusal test lands with slice 2. S, S-R, S-P and S-M are run through S4 here. |
