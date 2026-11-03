---
id: SR-2026110216388
title: 'PR #2155 code-review: AC sweep FR-144 collection algebra, 8 ACs tagged (QSL-1487)'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@peter/qsl-1487-ac-sweep; git diff origin/main...pr2155 (PR #2155): tests and trace tags only; tickets QSL-1487'
review_set: subset
date: 2026-11-03
---

## Summary

Tickets: QSL-1487. PR: quire-spec-language#2155 (head peter/qsl-1487-ac-sweep). Method: code-review over the trial merge of the PR onto origin/main (fd5bde253). Mergeability: clean (git merge-tree rc=0, trial merge onto fd5bde253 clean). Plan completion: not assessed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| QSpec-FR-144-AC-12 (QSL-1487) | spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md; tests: qsl-eval/tests/it/unbounded_set_call.rs an_unbounded_set_of_any_size_evaluates_and_stops_only_on_the_meter | examined |
| QSpec-FR-144-AC-15 (QSL-1487) | spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md; tests: qsl-eval/tests/it/child_composite.rs collections_of_the_parent_hold_children_and_order_by_declaration_key | examined |
| QSpec-FR-144-AC-16 (QSL-1487) | spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md; tests: qsl-semantics/tests/it/native_snapshot_values.rs defective_collections_refuse_in_a_field_and_in_a_parameter | examined |
| QSpec-FR-144-AC-17 (QSL-1487) | spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md; tests: qsl-semantics/tests/it/native_snapshot_values.rs defective_collections_refuse_in_a_field_and_in_a_parameter; set_bag_and_ordered_set_fields_admit_with_their_element_order | examined |
| QSpec-FR-144-AC-18 (QSL-1487) | spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md; tests: qsl-semantics/tests/it/native_snapshot_values.rs defective_collections_refuse_in_a_field_and_in_a_parameter | examined |
| QSpec-FR-144-AC-19 (QSL-1487) | spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md; tests: qsl-semantics/tests/it/native_snapshot_values.rs set_bag_and_ordered_set_fields_admit_with_their_element_order | examined |
| QSpec-FR-144-AC-20 (QSL-1487) | spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md; tests: qsl-semantics/tests/it/native_snapshot_values.rs defective_collections_refuse_in_a_field_and_in_a_parameter (new OrderedSet [1,1,"x"] vector) | examined |
| QSpec-FR-144-AC-22 (QSL-1487) | spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md; tests: qsl-semantics/tests/it/native_snapshot_values.rs edge_uuids_and_timestamps_admit_unchanged; a_noncanonical_or_out_of_domain_timestamp_refuses_invalid_value; defective_collections_refuse_in_a_field_and_in_a_parameter | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The AC-16 tag sits on defective_collections_refuse_in_a_field_and_in_a_parameter, which asserts wrong-value-kind and invalid-value only. The `missing-member` and `unknown-member` clauses are not asserted (the ticket says QSL's native spelling, ADR-034 CS-1, has no `type`/`elements` keys). The tag then records AC-16 as covered with two of its four causes untestable under the native spelling; that mapping question belongs to a ruling, not to a tag. | qsl-semantics/tests/it/native_snapshot_values.rs:2423-2440 |
| FND-002 | low | The AC is stated over supplied sets in FR-181 typed spelling; the tagged tests assert the equivalent native-spelling boundaries (uppercase UUID, "01", ±2^127) on scalar fields and elements. Defensible under ADR-034 CS-1..3; flagged so the mapping is explicit. Judgement call. | qsl-semantics/tests/it/native_snapshot_values.rs:781-800, 934-960 |

## Verdict

2 finding(s). Every added or changed test was read against the QSpec acceptance-criterion text. Gates on the trial merge: cargo test --all-features -p qsl-eval -p qsl-semantics all pass; gates rc=0. Ticket closability per ticket:

- QSL-1487: NO: AC-11, 14, 21, 23, 24, 25, 26, 27, 28 remain untagged; AC-16 tagged on partial coverage.
