---
id: SR-12198
title: "QSL-903 PR #1042 gap analysis: FR-367 single-admission entry against TC-1053"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@5e706bbffec5059142a824f3d8f7cf865b69cd1b; FR-367 Inputs and Outputs, FR-367-AC-1..AC-11, TC-1053 procedure and steps 1-9; qsl-replay/src/spine/bind.rs, qsl-replay/src/spine/bind/tests.rs"
review_set: subset
---

## Summary

Ticket: QSL-903. PR quire-spec-language#1042 at head 5e706bbff. This is a planless gap analysis, scoped to the diff. Plan completion: not assessed. Criteria were traced to tagged tests by reading. No matrix was run, because the brief is read-only. Ticket text was treated as data.

Bindings examined. All 16 TC-1053 tests bind through `bind_with` → `admit`, through `admit` + `inputs`, or directly over `admit`, so each binding uses exactly one `read_checked_package` call under `admission_evidence`:

- FR-367-AC-1: `tc_1053_step_1_operands_bind_in_application_order`, `tc_1053_step_1_a_unit_importing_a_library_binds` (exercises `admission_evidence`'s `supply_closure`). Correct.
- FR-367-AC-2: `tc_1053_step_2_literals_repeats_and_unary_operands`, `tc_1053_step_2_inline_literal_term_binds_without_a_node`. Correct.
- FR-367-AC-3: `tc_1053_step_3_binding_refusals_equal_runs`. Correct.
- FR-367-AC-4: `tc_1053_step_3_mistyped_arguments_equal_runs`, `tc_1053_step_3_reference_admission_equals_runs`, `tc_1053_step_3_foreign_model_selection_equals_runs`. Correct.
- FR-367-AC-5: `tc_1053_step_4_occurrence_identity_refusals`. Correct.
- FR-367-AC-6: `tc_1053_step_5_unbounded_and_nested_operands`. Correct.
- FR-367-AC-7: `tc_1053_step_6_values_past_i64_refuse`. Correct.
- FR-367-AC-8: `tc_1053_step_7_binding_does_not_evaluate`. Correct.
- FR-367-AC-9: `tc_1053_step_8_binding_is_the_operator_claims_operand_list`. Correct.
- FR-367-AC-10: `tc_1053_step_5_binder_operand_is_not_an_argument`, `tc_1053_step_5_quantifier_binder_operand_is_not_an_argument`. Correct.
- FR-367-AC-11: `tc_1053_step_9_binds_from_the_callers_admission_without_the_bytes`. Correct. The test can fail on the AC's behaviour: if binding over the admitted package with the emitted unit dropped gave anything other than `Bound` with `[7, 2]`, the test would fail, so the `[7, 2]` assertion is load-bearing and must stay. `drop(emitted)` makes the borrow checker prove that nothing reachable from the entry's inputs borrows the emitted bytes.

TC-1053 step 9 and its Expected Result match the test. The procedure preamble matches the `admit` helper. The removed `package_id` assertion matches the removed output member.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-367 states no behaviour for an `admitted` package that is not the admission of `checked`'s emitted bytes. No AC covers it and no test exercises it. Before this PR the pairing held by construction (`BindPackage::read`). It is now a caller obligation that is neither checked nor specified, and a mismatch yields `Operands(UnknownNode)` (`unsupported_construct`) or a `Fault` (see SR-12197 FND-001). Fix: state the precondition in FR-367 Inputs. If an outcome for a mismatch is specified, add an AC and a TC-1053 step that binds over a second unit's admission. | spec/functional/FR-367-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md:40-44, qsl-replay/src/spine/bind.rs:271-290 |
| FND-002 | low | FR-367-AC-11's last sentence ("The entry has no byte or `EmittedUnit` input, so it can neither read nor admit the emitted package a second time") is a claim about the signature, not behaviour a test observes. The compiler enforces it, and TC-1053 step 9's test cannot fail on it independently. That is acceptable evidence, but the AC should present it as the signature's guarantee rather than as part of the tested outcome. Fix: reword it as "by signature", or move it to Outputs, where the same statement already appears. | spec/functional/FR-367-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md:232 |

## Verdict

PASS (no major). Every FR-367 AC has a correctly tagged test, and every test binds through a single admission. Both findings are spec precision issues, not missing behaviour in the delivered code.
