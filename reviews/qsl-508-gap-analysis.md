---
id: SR-12072
title: "Gap analysis of quire-spec-language PR #934: FR-326-AC-1..5 (TC-836) and FR-341 classifier arms (TC-861)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@e2c39842b97155a1703db5e0cf233d0d4c4bca93; PR #934: FR-326-AC-1 to FR-326-AC-5 (TC-836), FR-341-AC-1, AC-3, AC-5, AC-6 (TC-861), against qsl-semantics/src/check/temporal_clause/tests.rs, qsl-semantics/src/check/profile.rs, qsl-semantics/src/check/assemble/tests.rs, qsl-replay/src/spine/clause/tests/refinement.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #934

## Summary

Ticket: QSL-508. Plan completion: not assessed. Planless, manual
acceptance-criteria-to-tests check against the diff. Nothing was run.

Checked each FR-326 AC against the tests that trace it:

- FR-326-AC-1: `a_bare_operator_is_unbounded_under_infinite_trace_and_refused_under_a_bounded_profile` checks `None`, no horizon, (`temporal-satisfaction`, `Unbounded`) with exactly one InfiniteTrace domain at the clause node, and MissingInterval at `eventually`'s span. Correct.
- FR-326-AC-2: `intervals_are_admitted_by_profile_and_the_horizon_is_the_greatest_reach` checks horizon 5, `[3,*]` refused at its span under event-position and `Some{3, Open}` under infinite-trace, and `[5,3]` refused under both profiles. Correct.
- FR-326-AC-3: `a_horizon_past_u64_is_a_work_budget_limit_at_the_overflowing_operator` checks WorkBudget, actual `u64::MAX + 1`, located at the outer operator. Correct.
- FR-326-AC-4: `the_unit_selects_exactly_one_temporal_profile` checks both cases at the clause span, with the identities found. Correct.
- FR-326-AC-5: `a_bounded_clause_over_a_bounded_type_is_bounded_and_keys_carry_the_profile` checks `Bounded` and unequal event/fixed keys. Correct.
- FR-341-AC-5 and AC-6: `each_temporal_clause_refusal_has_its_class` checks NotEmitted as `unsupported` and Missing/Open as `prohibited`. Correct.

All oracles are hand-written spans (`text.find`), literal numbers and literal enum values. None is derived from the code under test.

Of the 11 trace lines in the diff, six bindings tag an AC that the test does not check. FR-326-AC-4 and FR-326-AC-1 are each also covered by a correct test, so the matrix still counts them covered. The wrong tags cover behaviour that has no AC: the `using` alias check, `NotEmitted`, `ParameterType`, catalog resolution and parameter domains.

## Verdict

FIX: three medium trace findings. The production code has no major defect (SR-12071).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Three tests are traced to FR-326-AC-4 but do not check it. AC-4 is the refusal of zero or two temporal profile selections. `the_temporal_profiles_resolve_in_the_temporal_catalog` checks catalog resolution by identity (FR-110). `the_assembler_keeps_temporal_clauses_and_refuses_an_undeclared_alias` checks that the assembler keeps clauses and refuses an `UndeclaredAlias`. `an_unresolved_over_type_refuses_at_the_type` checks a `ParameterType`/`missing_declaration` refusal. Retag each to the requirement it checks (FR-110, or FR-326 with no AC), or drop the AC id. | qsl-semantics/src/check/profile.rs:227-229; qsl-semantics/src/check/assemble/tests.rs:454-456; qsl-semantics/src/check/temporal_clause/tests.rs:355-357 |
| FND-002 | medium | Two tests are traced to FR-326-AC-1 but do not check it. AC-1 is the bare `eventually` under infinite-trace versus event-position, with exactly one infinite-trace domain. `the_over_parameters_unbounded_positions_join_the_record` checks that an unbounded `over` type adds an Integer domain, and the bounded-profile-over-unbounded-type record. That is the FR-326 "Requirement record" behaviour, which has no AC. `the_package_check_refuses_a_clause_no_emission_writes_yet` checks the `NotEmitted` refusal from `PackageDeclarations::check`. FR-326 has no AC for that interim behaviour. Retag both to FR-326 with no AC id. | qsl-semantics/src/check/temporal_clause/tests.rs:322-324; qsl-semantics/src/check/temporal_clause/tests.rs:373-375 |
| FND-003 | medium | `each_temporal_clause_refusal_has_its_class` claims FR-341-AC-1, which requires that "a ConfigVersion unit that compiles classifies `admitted`". The test classifies only refusals. The `refused` cases it checks (ProfileSelection, InvertedInterval, ParameterType) are FR-341 rule 3, which is AC-3's typed-refusal case. Replace FR-341-AC-1 with FR-341-AC-3. | qsl-replay/src/spine/clause/tests/refinement.rs:298-304 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | retagged: profile test to FR-110; assembler and over-type tests to FR-326 with no AC id (this commit) |
| FND-002 | fixed | retagged to FR-326 with no AC id (this commit) |
| FND-003 | fixed | FR-341-AC-1 replaced by FR-341-AC-3 (this commit) |
