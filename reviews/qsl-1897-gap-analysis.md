---
id: SR-2026111405325202
title: "gap analysis of PR2702 serving local-projection and realizability"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-573, FR-574, FR-057-AC-15, TC-155, TC-1413 against qsl-analyze/src/protocol_analysis/{item,manifest,realize}.rs, qsl-replay/src/protocol_analysis/{realizability_replay,settle}.rs, qsl-analyze/tests/it/protocol_realizability.rs; PR2702 5d32b7e0f4cc3d33ce7d17af10881b37d8a62c14"
review_set: subset
---

## Summary

Ticket: QSL-1897. This is a planless, diff-scoped gap analysis. Plan completion: not assessed. The reviewer read the tests' `#[trace]` tags against the AC text rather than running `quire matrix`. The brief allows no builds, and the tags are directly readable. Every new test uses `ix_trace_rs::trace`, and none has a `/// Trace:` line. Each test asserts typed values: labels, axes, `TerminalValue`, `NegotiationSettlement`, `RealizabilityDisagreement`.

Bindings checked: `a_driver_run_settles_each_kind_under_each_label` covers FR-573-AC-1, AC-2, AC-6 and AC-7, with 11 items in one `analyze` run, each asserted on all four axes and its value. `the_protocol_kinds_are_served_by_the_linked_manifest` covers the `Single` and `UnsupportedRequestedCapability` clauses of FR-057-AC-15 for both kinds. `the_realizability_replay_arm_reproduces_both_forms_and_refuses_edits` covers AC-2's replay half for both realizability forms. Its AC-3 binding is partial; see code-review FND-003 (SR-2026111405325201), which is not repeated here. Code with no owning requirement: none. The manifest, the item adapters, the settlement map, the replay arm and `Execution::Unsupported` each trace to FR-573 or ADR-036 PA-4/PA-20.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The driver test is tagged FR-573-AC-4 and FR-573-AC-5 but does not run the cut and meter cases those ACs name. AC-4 covers "a result that depended on a cut", and no cut-count item is settled. AC-5 covers `max_transitions` and the evaluation meter, and only `max_states` is settled. The new meter arm in `realizability_item` (`ExpansionLimit::EvaluationMeter` to `ModelCheckLimit::EvaluationMeter`, with its value from `meter_bound`) has no test. TC-1413 still lists the cut-count run (step 4) and the `max_transitions` and meter stops (step 5). One meter-stopped realizability item in the driver run would cover the new code. | qsl-analyze/src/protocol_analysis/item.rs:129-137; qsl-analyze/tests/it/protocol_realizability.rs:732-742 |

## Scope and evidence

- FR-573-AC-1 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): sound projection and realizable protocol settle proved, closed-scope, completed, satisfied, Uncertified. (examined)
- FR-573-AC-2 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): counterexamples settle refuted only after replay. (examined)
- FR-573-AC-3 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): a disagreeing replay settles inconclusive. (examined)
- FR-573-AC-4 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): horizon, absent premise, cut settle inconclusive. (examined)
- FR-573-AC-5 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): max_states, max_transitions, meter settle failed resource-incomplete. (examined)
- FR-573-AC-6 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): FR-575 construct settles unsupported. (examined)
- FR-573-AC-7 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): no proved is Certified or Trusted. (examined)
- FR-057-AC-15 (spec/functional/FR-057): negotiation cause codes and settle_candidates settlements. (examined)

## Verdict

Coverage is adequate for the brief, which does not ask for exhaustive tests. FND-001 is low, and adding a meter-stopped item would close it. The AC-3 gap is carried by code-review FND-003.


## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | Nothing tests the arm's own `NotSoleMover` check. Removing it leaves every test green, because the engine's `forced_pair` already filters non-sole pairs and only forged input reaches the arm's check. TC-1413 step 3 now lists "a pair at an end where another role can still move" as a replay edit. Add one replay of the waiting fixture's pair, expecting `NotSoleMover`. | qsl-replay/src/protocol_analysis/realizability_replay.rs:178-180; spec/test-cases/TC-1413-protocol-analysis-results-settle-with-fr-360-labels-and-uncertified.md:30-34 |
| FND-003 | low | `an_action_only_not_yet_enabled_at_the_other_end_does_not_refute` is tagged FR-571-AC-2, which requires the hidden-choice protocol to refute with `IndistinguishableHistories`. The test shows a different protocol does not refute under FR-571 Behavior's new waiting rule, a rule with no acceptance criterion (spec-review FND-005). This is the wrong binding: tag it to the new AC once it exists. | qsl-analyze/tests/it/protocol_realizability.rs:699-740 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 16c26295e: the driver run adds a projection stopped by `max_transitions` (item 11) and a realizability result decided on `cut-count` (item 12), each asserted on its value and axes. The meter mapping is covered by the unit test `an_evaluation_meter_stop_fails_the_run_naming_the_counter` (tagged TC-1413, FR-573-AC-5), which calls `metered` directly. That test fails without the mapping. TC-1413 now says the meter stop is settled on the engine's own stop mapping. That is acceptable, because a `bounded-static` arena cannot reach the meter. |
| FND-002 | fixed | e6bdb2b33: `NotSoleMover` is gone. Its replacements are tested at the arm: `indistinguishable_histories_without_disjoint_not_lost_sets_are_refused` adds a history that takes one of the Buyer's two enabled sends and expects `Avoidable`, and `a_history_ending_at_a_finding_is_refused` expects `FindingOnPlay`. |
| FND-003 | fixed | The waiting test is renamed `a_role_cannot_wait_for_an_action_not_yet_enabled` and tagged `TC-1411`, `FR-571-AC-7`, the new AC it exercises. The binding is correct. What the AC requires is disputed in code-review FND-012 and spec-review FND-006. |
