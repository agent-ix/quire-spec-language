---
id: SR-2026110216389
title: 'PR #2155 gap-analysis: AC sweep FR-144 collection algebra, 8 ACs tagged (QSL-1487)'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@peter/qsl-1487-ac-sweep; git diff origin/main...pr2155 (PR #2155): tests and trace tags only; tickets QSL-1487'
review_set: subset
date: 2026-11-03
---

## Summary

Tickets: QSL-1487. PR: quire-spec-language#2155 (head peter/qsl-1487-ac-sweep). Method: gap-analysis over the trial merge of the PR onto origin/main (fd5bde253). Mergeability: clean (git merge-tree rc=0, trial merge onto fd5bde253 clean). Plan completion: not assessed.

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
| QSpec-FR-144-AC-11 (QSL-1487) | spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md; tests: (no tagged test)  | examined |
| QSpec-FR-144-AC-14 (QSL-1487) | spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md; tests: (no tagged test)  | examined |
| QSpec-FR-144-AC-21 (QSL-1487) | spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md; tests: (no tagged test)  | examined |
| QSpec-FR-144-AC-23 (QSL-1487) | spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md; tests: (no tagged test)  | examined |
| QSpec-FR-144-AC-24 (QSL-1487) | spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md; tests: (no tagged test)  | examined |
| QSpec-FR-144-AC-25 (QSL-1487) | spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md; tests: (no tagged test)  | examined |
| QSpec-FR-144-AC-26 (QSL-1487) | spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md; tests: (no tagged test)  | examined |
| QSpec-FR-144-AC-27 (QSL-1487) | spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md; tests: (no tagged test)  | examined |
| QSpec-FR-144-AC-28 (QSL-1487) | spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md; tests: (no tagged test)  | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Untagged: no test reads the published model reference-key vectors (partial via derived Ord). | spec (QSpec FR-144):- |
| FND-002 | medium | Untagged: spec conflict, QSpec FR-144-AC-14 admits Set<U> with a Float64 payload and u1=u2 ill_typed, QSL FR-323-AC-3 says numericEqual, code refuses Set<F>; needs titan:QSL:plan ruling. | spec (QSpec FR-144):- |
| FND-003 | medium | Untagged: no collection-level Decimal snapshot tests (TC-1171 steps 4-9). | spec (QSpec FR-144):- |
| FND-004 | medium | Untagged: no collection-level String tests (FR-106-AC-60). | spec (QSpec FR-144):- |
| FND-005 | medium | Untagged: no collection-level Rational/Quantity tests (FR-106-AC-55/56/59). | spec (QSpec FR-144):- |
| FND-006 | medium | Untagged: no collection-level Float tests (FR-106-AC-57/58; no 7ff8000000000001 vector). | spec (QSpec FR-144):- |
| FND-007 | medium | Untagged: Timer/Clock set tests exist under other ids (clock_places.rs, timed_subject.rs) and need reading and tagging. | spec (QSpec FR-144):- |
| FND-008 | medium | Untagged: Bytes collection keys not implemented (NativeValueType refuses). | spec (QSpec FR-144):- |
| FND-009 | medium | Untagged: JsonObject collection keys not implemented. | spec (QSpec FR-144):- |

## Verdict

Coverage of the QSpec criteria of the PR's tickets, computed by tag extraction over the trial tree (`QSpec-FR-n-AC-k` against the live ACs of the QSpec spec at /Users/peter/dev/quire-specification). 9 criterion(s) remain untagged. Ticket closability per ticket:

- QSL-1487: NO: AC-11, 14, 21, 23, 24, 25, 26, 27, 28 remain untagged; AC-16 tagged on partial coverage.
