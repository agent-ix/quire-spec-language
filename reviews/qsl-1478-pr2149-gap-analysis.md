---
id: SR-2026110216381
title: 'PR #2149 gap-analysis: AC sweep FR-142 quantities and units (QSL-1478)'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@87df8e903; git diff origin/main...pr2149 (PR #2149): tests and trace tags only; tickets QSL-1478'
review_set: subset
date: 2026-11-03
---

## Summary

Tickets: QSL-1478. PR: quire-spec-language#2149 (head 87df8e903). Method: gap-analysis over the trial merge of the PR onto origin/main (fd5bde253). Mergeability: clean (git merge-tree rc=0, trial merge onto fd5bde253 clean). Plan completion: not assessed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| QSpec-FR-142-AC-8 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: qsl-semantics/tests/it/quantities.rs published_compound_unit_vectors_admit_and_each_mutation_is_refused_by_its_named_check | examined |
| QSpec-FR-142-AC-11 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: qsl-semantics/src/check/assemble/tests.rs a_quantity_type_form_resolves_to_its_unit_and_refuses_a_non_unit_name; a_quantity_argument_of_another_unit_is_a_type_mismatch | examined |
| QSpec-FR-142-AC-12 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: qsl-semantics/src/check/assemble/tests.rs a_quantity_result_checks_and_a_converted_result_of_another_unit_is_a_mismatch | examined |
| QSpec-FR-142-AC-13 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: qsl-semantics/src/check/assemble/tests.rs a_unit_and_a_record_share_a_name_and_two_units_do_not; a_declaration_named_quantity_is_invalid_syntax | examined |
| QSpec-FR-142-AC-14 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: qsl-package/src/emit/tests/admission_corpus.rs quantity_literals_lower_to_shared_quantity_literal_nodes_the_strict_reader_admits | examined |
| QSpec-FR-142-AC-16 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: qsl-semantics/src/check/assemble/tests.rs the_base_dimension_rule_refuses_an_unanchored_base_dimension | examined |
| QSpec-FR-142-AC-22 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: qsl-semantics/tests/it/dimension_equivalence.rs dimension_equivalences_are_decided_by_fr_142_steps | examined |
| QSpec-FR-142-AC-24 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: qsl-semantics/tests/it/dimension_equivalence.rs an_equivalence_derives_one_class_with_the_least_key_as_root | examined |
| QSpec-FR-142-AC-27 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: qsl-semantics/src/check/type_form.rs; qsl-eval/tests/it/quantity_literals.rs carried_quantity_rejects_unbounded_or_invalid_carriers; a_carried_quantity_is_not_an_arithmetic_operand; a_carried_quantity_exponent_expression_is_not_admitted | examined |
| QSpec-FR-142-AC-28 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: qsl-eval/tests/it/quantity_literals.rs a_carried_quantity_conversion_returns_a_carried_value; carried_quantity_rounding_refusal_and_back_conversion | examined |
| QSpec-FR-142-AC-29 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: qsl-eval/tests/it/quantity_literals.rs carried_quantity_rounding_refusal_and_back_conversion | examined |
| QSpec-FR-142-AC-30 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: qsl-eval/tests/it/quantity_literals.rs records_and_collections_compare_same_typed_carried_quantities | examined |
| QSpec-FR-142-AC-20 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: qsl-semantics/tests/it/dimension_equivalence.rs unknown_package_dimensions_and_repeated_source_dimensions_refuse (deliberately tagged TC only, no AC-20 tag) | examined |
| QSpec-FR-142-AC-15 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: (no tagged test)  | examined |
| QSpec-FR-142-AC-17 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: (no tagged test)  | examined |
| QSpec-FR-142-AC-18 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: (no tagged test)  | examined |
| QSpec-FR-142-AC-19 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: (no tagged test)  | examined |
| QSpec-FR-142-AC-20 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: (no tagged test)  | examined |
| QSpec-FR-142-AC-21 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: (no tagged test)  | examined |
| QSpec-FR-142-AC-23 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: (no tagged test)  | examined |
| QSpec-FR-142-AC-25 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: (no tagged test)  | examined |
| QSpec-FR-142-AC-26 (QSL-1478) | spec/functional/type-model/FR-142-evaluate-quantities-and-units.md; tests: (no tagged test)  | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Untagged: assembler collects every error with no turn list/first-defect rule (QSL-936, Backlog). | spec (QSpec FR-142):- |
| FND-002 | medium | Untagged: same unit-graph topology work as AC-15 (QSL-936). | spec (QSpec FR-142):- |
| FND-003 | medium | Untagged: same unit-graph topology work as AC-15 (QSL-936). | spec (QSpec FR-142):- |
| FND-004 | medium | Untagged: package unit/dimension imports not implemented (`Quantity<R::s>` -> UnresolvedTypeName); no ticket found beyond QSL-936 (topology half). | spec (QSpec FR-142):- |
| FND-005 | medium | Untagged by design: refusal half asserted (unknown_package_dimensions_...), admission half not implemented. | spec (QSpec FR-142):- |
| FND-006 | medium | Untagged: blocked on package-unit work and IR (QSL-1332, IR-1032/IR-1153/STD-630). | spec (QSpec FR-142):- |
| FND-007 | medium | Untagged: needs IR identification edge (IR-1032/IR-1153/STD-630) and QSL-1332. | spec (QSpec FR-142):- |
| FND-008 | medium | Untagged: no quantity negation op (QSL-1389, FR-093-AC-49). | spec (QSpec FR-142):- |
| FND-009 | medium | Untagged: root-ratio conversions across owners; IR dependency (QSL-1332). | spec (QSpec FR-142):- |

## Verdict

Coverage of the QSpec criteria of the PR's tickets, computed by tag extraction over the trial tree (`QSpec-FR-n-AC-k` against the live ACs of the QSpec spec at /Users/peter/dev/quire-specification). 9 criterion(s) remain untagged. Ticket closability per ticket:

- QSL-1478: NO: AC-15, 17, 18, 19, 20, 21, 23, 25, 26 remain untagged and AC-14 and AC-16 are tagged on partial coverage; QSL-936 (blocked-by) and IR work remain.
