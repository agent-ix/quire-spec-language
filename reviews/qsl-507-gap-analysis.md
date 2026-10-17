---
id: SR-12056
title: "Gap analysis of quire-spec-language PR #926: FR-325 TemporalTrace forms"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3d82d2a55d69cf2a69fb37e61e07a809fbea3597; PR #926: FR-325 (statement, Outputs, Behavior, AC-1 to AC-6), TC-835, FR-123 Inputs edit, against qsl-forms/src/temporal_clause.rs, qsl-forms/src/syntax.rs, qsl-cst/src/grammar.rs and qsl-forms/tests/it/temporal_clause_forms.rs; context FR-326, FR-233, FR-337"
review_set: subset
---
# Gap analysis of quire-spec-language PR #926

## Summary

Ticket: QSL-507. Plan completion: not assessed. No builds were run (read-only review); the author's `make ci` result is taken as the gate evidence.

Each FR-325 criterion has a value-asserting TC-835 test tagged `#[trace("TC-835", "FR-325-AC-n")]`:

- FR-325-AC-1: `an_unbounded_operator_over_a_bounded_one_keeps_each_operators_interval`. Covered.
- FR-325-AC-2: `binary_and_unary_operators_build_none_closed_open_and_inverted_intervals` and `every_temporal_operator_takes_each_interval_shape_and_nests_freely`. Covered, including free nesting.
- FR-325-AC-3: `a_bound_that_is_not_a_u64_fails_at_s1_at_the_bound`. Covered, including the `u64` overflow boundary.
- FR-325-AC-4: `the_clause_forms_are_the_same_under_every_profile_selection`. Covered as the amended AC states.
- FR-325-AC-5: `fairness_constraints_build_their_kind_granularity_operation_and_span`. Covered.
- FR-325-AC-6: `captures_build_their_type_value_and_span_beside_an_unchanged_formula`. Covered.

"The TemporalTrace forms SHALL be the only S2 producer for temporal clauses" holds: `temporal` dispatches to `temporal_clause::temporal_clause` and no other path builds one.

## Verdict

Changes requested: one high finding. The S2 clause form drops the clause's `clock` string, which FR-325's Outputs also omit, while the S3 and emission requirements that read only this form need it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `TemporalClauseForm` has no clock. The grammar requires `clock <string>` on every clause, but `temporal_clause` never reads the `Text` token, so the clock string and its span are discarded at S2. FR-325's Outputs list leaves it out too. The S2 form is the only S2 producer and FR-326's only clause input. FR-233 needs the clock to bind `model-steps`/`model-time` and to refuse `model-time-on-untimed-model` at the clock clause's span. FR-326 needs it for the `IntervalKey` `clock_binding`. FR-337-AC-1 emits clock `"model-steps"` as an argument. None of these can be met from this form. Add `clock: {text, span}` to FR-325 Outputs and `TemporalClauseForm`, build it in `temporal_clause`, and assert it in TC-835. | spec/functional/FR-325-parse-temporal-operators-with-an-optional-interval.md:49-60; qsl-forms/src/syntax.rs (TemporalClauseForm); qsl-forms/src/temporal_clause.rs:29-67; spec/functional/FR-233-bind-a-model-claim-to-model-time-or-model-steps.md:65-72; spec/functional/FR-326-admit-temporal-operators-by-the-unit-s-temporal-profile.md:56,66 |
