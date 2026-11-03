---
id: SR-2026110216379
title: 'PR #2148 gap-analysis: AC sweep FR-143/FR-149/FR-039 (QSL-1479/1480/1477)'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@f8a51df0d; git diff origin/main...pr2148 (PR #2148): tests and trace tags only; tickets QSL-1479, QSL-1480, QSL-1477'
review_set: subset
date: 2026-11-03
---

## Summary

Tickets: QSL-1479, QSL-1480, QSL-1477. PR: quire-spec-language#2148 (head f8a51df0d). Method: gap-analysis over the trial merge of the PR onto origin/main (fd5bde253). Mergeability: clean (git merge-tree rc=0, trial merge onto fd5bde253 clean). Plan completion: not assessed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| QSpec-FR-143-AC-11 (QSL-1479) | spec/functional/type-model/FR-143-evaluate-record-tuple-recursive-values.md; tests: qsl-eval/tests/it/model_reference_queries.rs; relationship_navigation.rs l08_upcast_lookup_result_equals_the_subtype_reference; l11_deref_...; navigation_follows_the_whole_reference_key_never_the_object_identity_alone | examined |
| QSpec-FR-143-AC-12 (QSL-1479) | spec/functional/type-model/FR-143-evaluate-record-tuple-recursive-values.md; tests: qsl-semantics/src/check/sum_case/tests.rs; qsl-eval union_case.rs a_union_construction_checks_as_the_member_over_its_union; a_construction_of_the_wrong_shape_refuses_at_the_construction; collections_of_unions_order_dedupe_and_count_by_the_union_key | examined |
| QSpec-FR-143-AC-13 (QSL-1479) | spec/functional/type-model/FR-143-evaluate-record-tuple-recursive-values.md; tests: qsl-semantics/src/check/assemble/tests.rs a_union_recurses_only_through_an_escaping_edge; a_cycle_through_a_record_and_a_union_is_refused_at_both_declarations | examined |
| QSpec-FR-143-AC-14 (QSL-1479) | spec/functional/type-model/FR-143-evaluate-record-tuple-recursive-values.md; tests: qsl-eval/src/value/expression/evaluate/union_case.rs construction_and_case_charge_exactly_their_accounting_points | examined |
| QSpec-FR-143-AC-15 (QSL-1479) | spec/functional/type-model/FR-143-evaluate-record-tuple-recursive-values.md; tests: qsl-replay/src/execute/tests/union.rs; qsl-eval union_admission.rs a_witness_union_that_is_not_a_member_of_its_slots_union_is_refused; a_supplied_union_value_is_admitted_only_as_a_member_of_its_union | examined |
| QSpec-FR-143-AC-16 (QSL-1479) | spec/functional/type-model/FR-143-evaluate-record-tuple-recursive-values.md; tests: qsl-eval/tests/it/union_value_profile.rs a_union_resolves_the_selection_a_record_resolves | examined |
| QSpec-FR-143-AC-17 (QSL-1479) | spec/functional/type-model/FR-143-evaluate-record-tuple-recursive-values.md; tests: qsl-eval/tests/it/child_composite.rs; qsl-semantics/tests/it/child_composite.rs two_aliases_of_one_model_agree_on_keys_order_and_equality; a_model_record_key_is_its_model_node_key_whatever_the_alias | examined |
| QSpec-FR-143-AC-18 (QSL-1479) | spec/functional/type-model/FR-143-evaluate-record-tuple-recursive-values.md; tests: qsl-eval/tests/it/absent_literal.rs absent_denotes_the_none_of_the_option_its_position_expects | examined |
| QSpec-FR-143-AC-19 (QSL-1479) | spec/functional/type-model/FR-143-evaluate-record-tuple-recursive-values.md; tests: qsl-eval/tests/it/absent_literal.rs absent_with_no_unique_option_type_refuses | examined |
| QSpec-FR-143-AC-20 (QSL-1479) | spec/functional/type-model/FR-143-evaluate-record-tuple-recursive-values.md; tests: qsl-eval/tests/it/absent_literal.rs an_optional_field_takes_no_absent_literal | examined |
| QSpec-FR-149-AC-12 (QSL-1480) | spec/functional/type-model/FR-149-apply-complete-equality-matrix.md; tests: qsl-eval/tests/it/equality_matrix.rs e16a_references_admit_a_conforming_upcast_in_either_order; e16b_... | examined |
| QSpec-FR-149-AC-14 (QSL-1480) | spec/functional/type-model/FR-149-apply-complete-equality-matrix.md; tests: qsl-eval/tests/it/equality_matrix.rs e30_uuid_and_timestamp_compare_by_payload_as_one_terminal_pair | examined |
| QSpec-FR-149-AC-15 (QSL-1480) | spec/functional/type-model/FR-149-apply-complete-equality-matrix.md; tests: qsl-eval/tests/it/equality_matrix.rs e31_uuid_and_timestamp_never_compare_with_another_kind | examined |
| QSpec-FR-149-AC-16 (QSL-1480) | spec/functional/type-model/FR-149-apply-complete-equality-matrix.md; tests: qsl-eval/tests/it/equality_matrix.rs e32_native_keys_order_by_canonical_payload_bytes | examined |
| QSpec-FR-149-AC-17 (QSL-1480) | spec/functional/type-model/FR-149-apply-complete-equality-matrix.md; tests: qsl-eval/tests/it/child_composite.rs; qsl-semantics/tests/it/child_composite.rs equality_over_a_child_* tests (4) | examined |
| QSpec-FR-149-AC-18 (QSL-1480) | spec/functional/type-model/FR-149-apply-complete-equality-matrix.md; tests: qsl-semantics/tests/it/child_composite.rs equality_over_event_operands_stays_refused; a_set_bag_or_ordered_set_of_events_is_operator_ineligible | examined |
| QSpec-FR-149-AC-19 (QSL-1480) | spec/functional/type-model/FR-149-apply-complete-equality-matrix.md; tests: qsl-eval/tests/it/int_arithmetic_expected_type.rs a_guarded_sum_is_typed_by_the_int_it_is_compared_with | examined |
| QSpec-FR-149-AC-20 (QSL-1480) | spec/functional/type-model/FR-149-apply-complete-equality-matrix.md; tests: qsl-eval/tests/it/int_arithmetic_expected_type.rs an_unguarded_sum_refuses_unproved_range | examined |
| QSpec-FR-149-AC-21 (QSL-1480) | spec/functional/type-model/FR-149-apply-complete-equality-matrix.md; tests: qsl-eval/tests/it/int_arithmetic_expected_type.rs only_a_binary_arithmetic_operand_of_an_equality_takes_the_expected_type | examined |
| QSpec-FR-039-AC-1 (QSL-1477) | spec/functional/foundation/FR-039-normalize-exact-rational-literals.md; tests: tests/it/composed_types.rs rational_literals_normalize_before_the_bounds_and_the_boundaries_admit | examined |
| QSpec-FR-039-AC-2 (QSL-1477) | spec/functional/foundation/FR-039-normalize-exact-rational-literals.md; tests: tests/it/composed_types.rs zero_denominators_and_out_of_range_literals_refuse_with_distinct_causes | examined |
| QSpec-FR-039-AC-3 (QSL-1477) | spec/functional/foundation/FR-039-normalize-exact-rational-literals.md; tests: tests/it/composed_types.rs both new tests | examined |
| QSpec-FR-039-AC-4 (QSL-1477) | spec/functional/foundation/FR-039-normalize-exact-rational-literals.md; tests: tests/it/composed_types.rs a_rational_literal_never_picks_its_own_domain | examined |
| QSpec-FR-039-AC-5 (QSL-1477) | spec/functional/foundation/FR-039-normalize-exact-rational-literals.md; tests: tests/it/composed_types.rs exhaustion_preserves_earlier_types_but_never_admits_an_unfinished_forward_dependency | examined |
| QSpec-FR-149-AC-13 (QSL-1480) | spec/functional/type-model/FR-149-apply-complete-equality-matrix.md; tests: (no tagged test)  | examined |
| QSpec-FR-149-AC-22 (QSL-1480) | spec/functional/type-model/FR-149-apply-complete-equality-matrix.md; tests: (no tagged test)  | examined |
| QSpec-FR-149-AC-23 (QSL-1480) | spec/functional/type-model/FR-149-apply-complete-equality-matrix.md; tests: (no tagged test)  | examined |
| QSpec-FR-149-AC-24 (QSL-1480) | spec/functional/type-model/FR-149-apply-complete-equality-matrix.md; tests: (no tagged test)  | examined |
| QSpec-FR-149-AC-25 (QSL-1480) | spec/functional/type-model/FR-149-apply-complete-equality-matrix.md; tests: (no tagged test)  | examined |
| QSpec-FR-149-AC-26 (QSL-1480) | spec/functional/type-model/FR-149-apply-complete-equality-matrix.md; tests: (no tagged test)  | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Untagged on the PR tree; blocked on an IR kernel op (quire-exact has no quiet ordering op); QSL-1388. QSL-1480 stays open. Note qsl-eval equality_matrix.rs x05 still asserts float ordering is operator-ineligible, stale against this AC. | spec (QSpec FR-149):- |
| FND-002 | medium | Untagged; float half contradicted by quire-exact equality.rs (refuses Value::Float leaves) and e28; blocked on IR / QSL-1388. | spec (QSpec FR-149):- |
| FND-003 | medium | Untagged; no kernel value kind in quire-exact @87e4023; QSL-1068 related. | spec (QSpec FR-149):- |
| FND-004 | medium | Untagged; same IR dependency as AC-23. | spec (QSpec FR-149):- |
| FND-005 | medium | Untagged; blocked on IR (numericEqual leaf). | spec (QSpec FR-149):- |
| FND-006 | medium | Untagged; blocked on IR. | spec (QSpec FR-149):- |

## Verdict

Coverage of the QSpec criteria of the PR's tickets, computed by tag extraction over the trial tree (`QSpec-FR-n-AC-k` against the live ACs of the QSpec spec at /Users/peter/dev/quire-specification). 6 criterion(s) remain untagged. Ticket closability per ticket:

- QSL-1479: NO: AC-11..20 are all tagged and pass, but AC-18 omits two named sub-cases (self.v = absent write; lookup ... absent empty) and AC-11 is thin; finish those (needs a model fixture) before closing.
- QSL-1480: NO: AC-13, 22, 23, 24, 25, 26 remain untagged (IR-blocked, QSL-1388/QSL-1068); AC-15/16/21 are tagged on partial coverage.
- QSL-1477: NO against full AC coverage: all five ACs are tagged and pass, but AC-4's runtime-expression-argument clause is untested; add that test, then close.
