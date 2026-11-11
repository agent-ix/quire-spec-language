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
