---
id: SR-2026110216380
title: 'PR #2149 code-review: AC sweep FR-142 quantities and units (QSL-1478)'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@87df8e903; git diff origin/main...pr2149 (PR #2149): tests and trace tags only; tickets QSL-1478'
review_set: subset
date: 2026-11-03
---

## Summary

Tickets: QSL-1478. PR: quire-spec-language#2149 (head 87df8e903). Method: code-review over the trial merge of the PR onto origin/main (fd5bde253). Mergeability: clean (git merge-tree rc=0, trial merge onto fd5bde253 clean). Plan completion: not assessed.

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

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The tag is applied to quantity_literals_lower_to_shared_quantity_literal_nodes_the_strict_reader_admits, which asserts the literal-node half. The ticket records that the second half (operation parameter whose typeRef names a unit) is asserted by no test, so the tag reports AC-14 covered while half of it is not. | qsl-package/src/emit/tests/admission_corpus.rs:2560-2580 |
| FND-002 | medium | the_base_dimension_rule_refuses_an_unanchored_base_dimension asserts U11g1 and U11g3 only (the ticket lists U11g2, U11g4, U11m, U11y, U11z1 as untested; blocked on the turn list, QSL-936). The AC is tagged, so a matrix reports it covered with 2 of 7 named rows verified. | qsl-semantics/src/check/assemble/tests.rs:2705-2745 |
| FND-003 | low | published_compound_unit_vectors_admit_and_each_mutation_is_refused_by_its_named_check prints "skipped: QSPEC_DIR not set" and returns (a pass) when QSPEC_DIR is unset, so a default `cargo test` reports the AC verified with no assertion run. Seven other tests on main use the same skip-pass idiom and it ran and passed here with QSPEC_DIR set, so this is low; a hard failure in CI contexts (or `make conformance` only) would be stronger. | qsl-semantics/tests/it/quantities.rs:693-700 |
| FND-004 | low | a_quantity_result_checks_and_a_converted_result_of_another_unit_is_a_mismatch calls assemble(&source) twice (`package` and a second `regions`), and asserts via `.any()` over all refusals, so an extra unrelated refusal would not be noticed. Use the one assembled value for both and assert the refusal list. | qsl-semantics/src/check/assemble/tests.rs:2837-2870 |

## Verdict

4 finding(s). Every added or changed test was read against the QSpec acceptance-criterion text. Gates on the trial merge: cargo test --all-features -p qsl-eval -p qsl-semantics -p qsl-package: only the 27 baseline qsl-package checked_v2/timed_order failures (QSL-1562), none in touched files; workspace check, clippy x2, arch-lint rc=0. Ticket closability per ticket:

- QSL-1478: NO: AC-15, 17, 18, 19, 20, 21, 23, 25, 26 remain untagged and AC-14 and AC-16 are tagged on partial coverage; QSL-936 (blocked-by) and IR work remain.
