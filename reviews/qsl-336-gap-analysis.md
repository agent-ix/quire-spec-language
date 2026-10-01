---
id: SR-907
title: "QSL-336 gap analysis of PR 541 (FR-122 AC-1 to AC-6, TC-517, FR-106-AC-8, TC-464 step 5)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ec9476fb2a8970754aaf6974647841a995991f32; spec/functional/FR-122-replay-a-state-clause-counterexample.md; spec/test-cases/TC-517-replay-a-state-clause-counterexample.md; spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/test-cases/TC-464-snapshots-and-invocations-admit.md; qsl-replay/src/execute/state_clause.rs; qsl-replay/src/spine/clause/tests/state_clause_replay.rs; qsl-semantics/src/model/observation.rs; qsl-semantics/tests/it/state_clauses.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-517
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-464
    type: reviews
---
## Summary

Ticket: QSL-336 (code half). PR: quire-spec-language#541 at ec9476fb. There
is no plan bundle. The acceptance text is FR-122 AC-1 to AC-6 (as this PR
amends AC-2 under the leader's FR-072 ruling), TC-517 steps 1 to 6,
FR-106-AC-8 and TC-464 step 5 on main.

| AC / step | Test(s) | Verdict |
| --- | --- | --- |
| FR-122-AC-1 / step 1 | `a_violated_postcondition_reproduces_keeping_its_identities`, `an_input_arm_envelope_reproduces_without_a_witness`, `a_violated_invariant_reproduces_keeping_its_snapshot` | covered (see FND-001) |
| FR-122-AC-2 / step 2 | `a_holding_postcondition_is_inconclusive_by_verdicts`, `a_holding_invariant_is_inconclusive_by_verdicts`, `an_exhausted_evaluation_budget_is_inconclusive_with_no_value` (all assert `record().is_none()` per the ruling) | covered |
| FR-122-AC-3 / step 3 | `a_stale_clause_node_refuses_naming_both_nodes_before_admission` (node, node+occurrence, each with and without the invocation), `a_stale_occurrence_refuses_naming_both_occurrences_before_admission`, `a_source_edit_refuses_by_the_stale_package_rule`, `a_clause_naming_no_state_clause_refuses_missing_name` | covered; node/occurrence mutants killed |
| FR-122-AC-4 / step 4 | `a_violated_precondition_reproduces_over_its_pre_call_state`, `a_holding_precondition_is_inconclusive_by_verdicts`, `an_observation_form_other_than_the_clause_kinds_refuses_before_admission` (three mismatches, with and without documents) | covered; form-check mutant killed |
| FR-122-AC-5 / step 5 | `a_frame_violating_invocation_refuses_with_the_admission_record`, `an_absent_pre_snapshot_refuses_with_the_admission_record`, `edited_invocation_bytes_refuse_byte_digest_mismatch`, `an_incomplete_population_refuses_with_the_incomplete_record` | covered |
| FR-122-AC-6 / step 6 | `replaying_one_envelope_twice_gives_equal_results` (generic `FamilyPayload` bound, three forms) | covered |
| FR-106-AC-8 / TC-464 step 5 | seven `tc464_step5_*` tests: admits, postcondition, `post` snapshot, no `target`, `ghost`, archive incomplete, snapshot incomplete | covered; "runs no frame check" is structural (no post snapshot exists) |

Every test carries `#[trace("TC-517", "FR-122-AC-n")]` or
`#[trace("TC-464", "FR-106-AC-8")]` against the AC it checks. All bindings
are correct. The reviewer ran the focused suites (`cargo test -p
qsl-replay`, 226 passed; `-p qsl-semantics --test it tc464`, 12 passed;
`state_clauses`, 97 passed; `-p qsl-eval`, all pass), each with exit 0.

The coverage gap on the invocation-side check-7 seeding is code-review
SR-906 FND-003. The leader's form ruling is FR-122's own Behavior text and
is tested as such.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-122-AC-1 says "Each result holds the source digest, the `package_id`, the payload's `clause`, the envelope's `clause_node` and `occurrence_key`". The `Input`-arm test asserts only the settlement, the value and `documents()`, so a regression that drops the identities from the `Input`-arm result would pass. The two `Witness`-arm tests do assert them through `assert_reproduces_keeping_identities`. | qsl-replay/src/spine/clause/tests/state_clause_replay.rs:336-355 |

## Verdict

Every AC and TC step in scope is delivered and tested. Each test is traced
to the right AC and its oracle is independent: identities come from the
compiled package and expected values are constants. One low gap.

## Dispositions

Round 1, reviewed at 4aa30e7bf719768fffc2c1ad3eeb1bd26ef83b93.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 4aa30e7b: `an_input_arm_envelope_reproduces_without_a_witness` now asserts the source digest, `package_id`, clause, `clause_node` and `occurrence_key` alongside `documents()`. |
