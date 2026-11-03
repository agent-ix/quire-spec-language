---
id: SR-2026110216377
title: 'PR #2147 gap-analysis: AC sweep FR-151/FR-150/FR-153/FR-152/FR-147 (QSL-1483/1482/1481/1484/1485)'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@4414aca42; git diff origin/main...pr2147 (PR #2147): tests and trace tags only; tickets QSL-1483, QSL-1482, QSL-1481, QSL-1484, QSL-1485'
review_set: subset
date: 2026-11-03
---

## Summary

Tickets: QSL-1483, QSL-1482, QSL-1481, QSL-1484, QSL-1485. PR: quire-spec-language#2147 (head 4414aca42). Method: gap-analysis over the trial merge of the PR onto origin/main (fd5bde253). Mergeability: clean (git merge-tree rc=0, trial merge onto fd5bde253 clean). Plan completion: not assessed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| QSpec-FR-151-AC-17 (QSL-1483) | spec/functional/type-model/FR-151-resolve-conformance-redefinition-dispatch.md; tests: qsl-replay/tests/dispatch_link_compile.rs effective_precondition_is_own_conjunction_or_the_redefined_precondition | examined |
| QSpec-FR-151-AC-18 (QSL-1483) | spec/functional/type-model/FR-151-resolve-conformance-redefinition-dispatch.md; tests: qsl-replay/tests/dispatch_link_compile.rs a_member_without_own_precondition_clauses_is_unconditionally_admitted | examined |
| QSpec-FR-150-AC-7 (QSL-1482) | spec/functional/type-model/FR-150-preserve-model-normalization-provenance.md; tests: qsl-semantics/tests/it/model_normalization.rs n07_record_order_does_not_affect_identity_or_view | examined |
| QSpec-FR-150-AC-12 (QSL-1482) | spec/functional/type-model/FR-150-preserve-model-normalization-provenance.md; tests: qsl-semantics/tests/it/model_identity_links.rs links_expand_to_independent_enumeration; the_strict_reader_refuses_every_mis_wired_link | examined |
| QSpec-FR-150-AC-13 (QSL-1482) | spec/functional/type-model/FR-150-preserve-model-normalization-provenance.md; tests: qsl-semantics/tests/it/model_identity_links.rs one_changed_phase_four_fact_changes_only_the_declaration_that_holds_it; identity_uses_owner_and_original_while_facts_retain_ancestry | examined |
| QSpec-FR-150-AC-14 (QSL-1482) | spec/functional/type-model/FR-150-preserve-model-normalization-provenance.md; tests: qsl-semantics/tests/it/model_identity_cost.rs cost_follows_closed_formulas | examined |
| QSpec-FR-153-AC-11 (QSL-1481) | spec/functional/type-model/FR-153-query-closed-model-environments.md; tests: qsl-replay/src/spine/clause/tests/model_gate.rs an_operation_body_needs_the_model_profile; a_dispatched_call_and_a_lookup_need_the_model_profile; population_types_all_instances_and_field_navigation_need_no_model_profile | examined |
| QSpec-FR-153-AC-12 (QSL-1481) | spec/functional/type-model/FR-153-query-closed-model-environments.md; tests: qsl-semantics/tests/it/named_population.rs; qsl-eval/tests/it/named_population.rs a_precondition_and_an_invariant_name_a_population_as_an_operand; a_bound_names_a_bounded_domain_and_a_named_population_is_a_domain | examined |
| QSpec-FR-153-AC-13 (QSL-1481) | spec/functional/type-model/FR-153-query-closed-model-environments.md; tests: qsl-semantics/tests/it/named_population.rs an_unknown_population_or_an_alias_that_is_also_an_enum_refuses; a_population_as_a_value_refuses_as_operator_ineligible; a_function_body_names_no_population_and_seq_stays_refused | examined |
| QSpec-FR-153-AC-14 (QSL-1481) | spec/functional/type-model/FR-153-query-closed-model-environments.md; tests: qsl-eval/tests/it/named_population.rs; qsl-semantics/tests/it/model_population.rs a_named_population_is_read_from_the_snapshot_of_the_clause_s_observation; l02_unknown_closure_is_incomplete_not_refused | examined |
| QSpec-FR-152-AC-7 (QSL-1484) | spec/functional/type-model/FR-152-bind-systems-model-structures.md; tests: qsl-eval/tests/it/relationship_navigation.rs four navigation tests (TC-1034 rows 1,5,8,11) | examined |
| QSpec-FR-152-AC-9 (QSL-1484) | spec/functional/type-model/FR-152-bind-systems-model-structures.md; tests: qsl-semantics/tests/it/relationship_navigation.rs n04_...; a_far_end_role_finds_no_candidate_and_refuses_a_type_mismatch | examined |
| QSpec-FR-152-AC-10 (QSL-1484) | spec/functional/type-model/FR-152-bind-systems-model-structures.md; tests: qsl-eval/tests/it/relationship_navigation.rs a_count_below_the_lower_bound_refuses_below_minimum_at_the_same_point; an_incomplete_or_unlisted_extent_is_incomplete_scope_and_an_empty_one_is_empty | examined |
| QSpec-FR-152-AC-11 (QSL-1484) | spec/functional/type-model/FR-152-bind-systems-model-structures.md; tests: qsl-eval/tests/it/relationship_reachability.rs; qsl-semantics/tests/it/relationship_reachability.rs row_9, row_11, row_12, row_13, c17 | examined |
| QSpec-FR-152-AC-12 (QSL-1484) | spec/functional/type-model/FR-152-bind-systems-model-structures.md; tests: qsl-eval/tests/it/relationship_reachability.rs; qsl-semantics/tests/it/relationship_reachability.rs row_16, row_17, row_20, row_21, row_22, c08-c13, c16, c18 | examined |
| QSpec-FR-147-AC-11 (QSL-1485) | spec/functional/expressions/FR-147-evaluate-integer-division-domains.md; tests: qsl-forms/tests/it/value_forms.rs; qsl-semantics/tests/it/state_clauses.rs; qsl-eval/tests/it/source_call.rs integer_operator_forms_retain_operand_spans_in_dead_branches; a_complete_function_does_not_enable_integer_operators_in_a_state_only_clause; integer_division_operators_follow_each_selected_signed_law | examined |
| QSpec-FR-151-AC-16 (QSL-1483) | spec/functional/type-model/FR-151-resolve-conformance-redefinition-dispatch.md; tests: (no tagged test)  | examined |
| QSpec-FR-147-AC-3 (QSL-1485) | spec/functional/expressions/FR-147-evaluate-integer-division-domains.md; tests: (no tagged test)  | examined |
| QSpec-FR-147-AC-7 (QSL-1485) | spec/functional/expressions/FR-147-evaluate-integer-division-domains.md; tests: (no tagged test)  | examined |
| QSpec-FR-147-AC-8 (QSL-1485) | spec/functional/expressions/FR-147-evaluate-integer-division-domains.md; tests: (no tagged test)  | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No QSpec-FR-151-AC-16 tag exists on the PR tree (cov.py: untagged-pr [16]). Needs the several-unit package compile (FR-480; owners QSL-1519/QSL-1518 per the ticket). QSL-1483 cannot be closed against this PR. | spec (QSpec FR-151):- |
| FND-002 | medium | Untagged on the PR tree; classified (d) not a QSL unit (backend provider contract STD-562 / IR I13). QSL-1485 stays open. | spec (QSpec FR-147):- |
| FND-003 | medium | Untagged on the PR tree; qualification-corpus AC, no QSL integer-division SMT provider (QSL-1391 related). QSL-1485 stays open. | spec (QSpec FR-147):- |
| FND-004 | medium | Untagged on the PR tree; same dependency as AC-7. QSL-1485 stays open. | spec (QSpec FR-147):- |

## Verdict

Coverage of the QSpec criteria of the PR's tickets, computed by tag extraction over the trial tree (`QSpec-FR-n-AC-k` against the live ACs of the QSpec spec at /Users/peter/dev/quire-specification). 4 criterion(s) remain untagged. Ticket closability per ticket:

- QSL-1483: NO: QSpec FR-151-AC-16 untagged (needs FR-480 several-unit compile; QSL-1518/QSL-1519). AC-17 and AC-18 are tagged and pass; the AC-17 test is killed by an Or->And mutation of the effective-precondition combinator (checked_dispatch.rs:1058).
- QSL-1482: YES: AC-7, 12, 13, 14 are all tagged and pass on the trial merge.
- QSL-1481: NO: AC-11/12/13 are tagged and pass, but AC-14 is tagged only for 2 of its 3 causes and the ticket itself records an open QSpec/QSL conflict (missing-required-artifact vs incomplete-scope; unclosed-subtypes vs Undefined incomplete-closure). Needs the titan:QSL:plan ruling and tests, or a retraction of the AC-14 tag.
- QSL-1484: YES: AC-7, 9, 10, 11, 12 are all tagged and pass on the trial merge.
- QSL-1485: NO: AC-11 tagged and passing; AC-3, AC-7, AC-8 remain untagged by design (backend/qualification lane, not QSL).
