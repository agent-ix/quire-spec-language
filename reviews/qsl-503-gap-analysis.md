---
id: SR-12062
title: "QSL-503 PR #929 gap analysis: FR-321, FR-322, FR-323 and FR-046-AC-9 union runtime"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@c2fb6d92d5eb6d5c5ea3f7852163f53dbba96db6; FR-321-AC-1..4, FR-322-AC-1..4, FR-323-AC-1, FR-323-AC-2, FR-046-AC-9; qsl-eval/src/value/expression/evaluate.rs, qsl-eval/src/value/expression/evaluate/union_case.rs, qsl-eval/src/value/expression/union_admission.rs, qsl-replay/src/execute/argument.rs, qsl-replay/src/execute/tests/union.rs, qsl-semantics/src/check/union_fixture.rs"
review_set: subset
---

## Summary

Ticket: QSL-503. PR quire-spec-language#929. Planless, diff-scoped gap
analysis. Plan completion: not assessed. Criteria were traced to tagged tests
by reading; no matrix was run (read-only brief). QSpec is private and is cited
by id only.

Bindings examined:

- FR-321-AC-1: `a_supplied_union_value_is_admitted_only_as_a_member_of_its_union`
  (direct admission) and
  `a_witness_union_that_is_not_a_member_of_its_slots_union_is_refused`
  (replay). Both check foreign key, another union's member, undeclared
  member, short payload and wrong-kind payload, all refused as
  `wrong-value-kind` at parameter 0, and both admit `Rect(2, 3)`.
- FR-321-AC-2: `a_dangling_reference_in_a_union_payload_is_refused_by_the_reference_walk`.
- FR-321-AC-3: `union_values_round_trip_through_their_witness_spelling`
  (witness to kernel, and witness to text to witness to kernel).
- FR-321-AC-4: `a_ten_thousand_level_supplied_tree_is_admitted` and
  `a_ten_thousand_level_witness_tree_converts_and_is_bounded_by_occurrences`
  (29,998 stops `ValueOccurrences` in the conversion phase, 29,999 admits,
  on a 512 KiB stack).
- FR-322-AC-1: `case_selects_the_active_members_arm_and_binds_its_payload`,
  `a_case_in_an_arm_body_evaluates_its_inner_arm`.
- FR-322-AC-2: `a_stopped_scrutinee_or_argument_is_the_outcome_with_nothing_after_it`,
  `a_scrutinee_matching_no_arm_is_an_s6a_internal_fault`.
- FR-322-AC-3: `construction_and_case_charge_exactly_their_accounting_points`.
- FR-322-AC-4: `case_nesting_ten_thousand_deep_completes_and_stops_at_its_exact_work`.
- FR-323-AC-1/AC-2: `collections_of_unions_order_dedupe_and_count_by_the_union_key`.
- FR-046-AC-9: `predicates_and_queries_over_a_union_sequence_follow_occurrence_order`.

None of the added tests is a stub or a tautology. Each asserts a typed
outcome, refusal, charge sequence or limit stop, never message text.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS. No major gap beyond those the author already recorded. The author's
recorded gaps are planner questions, not defects, and are not re-raised here:

- Hand-built bodies over sealed fixture unions instead of source programs,
  because lowering still refuses unions (QSL-504).
- No kernel-to-witness producer for FR-321-AC-3's "back" direction.
- The FR-321-AC-4 occurrence stop exists only on the replay conversion path.
  FR-321-AC-5 belongs to QSL-778.
- FR-322-AC-4 and FR-046-AC-9 run as an unrolled or inline `case`, not
  through function recursion or calls. FR-322-AC-4's exact 39,996/39,997
  numbers are therefore not reached.
- The QSpec TC-265 cases are not covered.
- FR-323-AC-3 and S3 admission of collections of unions belong to
  QSL-502/QSL-504.

Not raised (below the major-only bar): FR-321-AC-1 also lists "an enum's"
`VariantId` among the refused cases. Neither admission test supplies an enum
value at a `Shape` slot. The refusal follows trivially from the type-pair
mismatch.
