---
id: SR-12305
title: 'Gap analysis of PR #1131: HP-5 through the possible family (FR-181)'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@62b9d71fd; git diff origin/main...62b9d71fd (PR #1131, branch qsl-556-hp5): spec/functional/FR-181-check-a-single-existential-claim-through-the-possible-family.md, qsl-analyze/src/model_check/hyper/single.rs, qsl-analyze/tests/it/hyper_single.rs, qsl-replay/src/model_check/existential_outcome.rs; ticket QSL-556'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-181
  type: reviews
---

## Summary

Ticket: QSL-556. PR: quire-spec-language#1131, head 62b9d71fd. Planless gap analysis of FR-181-AC-1 to AC-7 against the tagged tests in `qsl-analyze/tests/it/hyper_single.rs` and the engine in `hyper/single.rs`. Plan completion: not assessed. The full `quoin matrix --repo .` run did not finish within 600 s, so the trace tags were read from the test source.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-181-AC-1 | spec/functional/FR-181-...md | examined |
| FR-181-AC-2 | spec/functional/FR-181-...md | examined |
| FR-181-AC-3 | spec/functional/FR-181-...md | examined |
| FR-181-AC-4 | spec/functional/FR-181-...md | examined |
| FR-181-AC-5 | spec/functional/FR-181-...md | examined |
| FR-181-AC-6 | spec/functional/FR-181-...md | examined |
| FR-181-AC-7 | spec/functional/FR-181-...md | examined |
| FR-181 Outcomes, Check, Phase 0, Replay | spec/functional/FR-181-...md | examined |
| 26 tests in hyper_single | qsl-analyze/tests/it/hyper_single.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-181's replay, certificate and settlement halves have no owning ticket. That covers the `proved`, `refuted`, `inconclusive` and `incomplete` records of AC-1 to AC-6, AC-4's second sentence (FR-128 refusal of an unclosed loop), the FR-163 certificate a `Witnessed` must carry, and FR-181 Replay. The PR body says `Remaining work: the replay, certificate and settlement slices above`, but no Linear ticket names them. QSL-892 covers FR-170 state-graph replay, not the HP-5 lasso and trap replay. QSL-929 covers FR-182/183 spec gaps. When QSL-556 closes on merge, FR-181 is left half built and nothing tracks the rest. File a ticket for each slice, or one slice ticket, linked from QSL-556. | QSL-556; qsl-analyze/tests/it/hyper_single.rs:15-18 |
| FND-002 | low | Three tests are tagged to an AC whose claim they do not assert. The new `a_loop_through_an_undecided_state_is_not_a_witness` (FR-181-AC-3) checks the undecided-state rule (FR-181 Check/open), not AC-3's `fair weak V::Vault::reset;` claim over the reset vault. The same holds for `a_cycle_through_a_partly_expanded_state_is_not_a_witness` (AC-3, fairness on `step` with `max_transitions` 1) and `an_unfinished_part_is_not_a_trap` (AC-2, `max_transitions` 1). This is the pattern SR-13628 FND-002 fixed for six other tests. Tag them with the TC id alone. | qsl-analyze/tests/it/hyper_single.rs:796, 833, 1221 |

## Verdict

No major finding. One medium is a tracking gap: the engine half is complete, but nothing tracks the rest. One low is a trace-tag finding.

Each AC has a tagged test that can fail. AC-1: `the_secure_vault_is_witnessed_by_a_step_1_loop_per_initial_state`, `sampled_witnesses_name_their_walk_by_seed_and_trace_index`. AC-2: `an_unreachable_output_traps_the_first_initial_state`. AC-3: `a_fairness_set_on_the_existential_variable_changes_the_verdict`. AC-4: `a_horizon_of_one_leaves_the_states_at_depth_one_open`. AC-5: `an_undefined_letter_refutes_although_every_initial_state_has_a_witness` and the tie tests. AC-6: `a_stopped_exploration_with_sampled_witnesses_is_witnessed_unchecked`. AC-7: `instances_share_the_runs_limits`; the shared-budget mutant fails it. The engine halves match the ACs. Every AC-tagged test now says in its doc that it covers the engine half. No stub, `todo!` or tautological assertion was found. The ten mutants in SR-12304 were all killed.

## Dispositions of SR-13628 (gap-analysis at 4943555dd)

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 2a81de08c: each AC-tagged test's doc reads "Engine half of the criterion: the outcome shape. Replay, the certificate and settlement are not built." The tracking half of the fix is still missing; it is raised again as FND-001 above. |
| FND-002 | fixed | 2a81de08c: `a_clause_that_is_not_single_existential_is_refused` is TC-606 only; `a_contract_with_no_value_leaves_its_state_open`, `the_limits_and_cancellation_stop_the_run` and `walks_the_step_ceiling_stopped_are_counted` are TC-616 only; `instances_share_the_runs_limits` and `no_instance_and_no_initial_state_decide_nothing` are now FR-181-AC-7, which the new AC states. |
| FND-003 | fixed | 2a81de08c: FR-181-AC-5 now says the outcome is `Undefined` in both runs while phase 0 finds the sampled lasso. TC-615 step 1 was not updated (SR-12306 FND-003). |
| FND-004 | fixed | 2a81de08c: FR-181 Outcomes ranks a trap at the first product state of its part and gives a tie to `Undefined`, which is what the engine does. |
| FND-005 | fixed | 2a81de08c: FR-181 Outcomes states the per-instance rule, and FR-181-AC-7 states it as a criterion. TC-606 does not cover AC-7 (SR-12306 FND-001). |
| FND-006 | fixed | 2a81de08c: FR-181-AC-3 reads `fair weak V::Vault::reset;`. TC-606 step 3 still has the braced form (SR-12306 FND-002); FR-171, FR-173, FR-174 and FR-177 are left to their owners. |
