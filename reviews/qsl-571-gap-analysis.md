---
id: SR-12236
title: "Gap analysis of QSL-571 E17d against FR-192-AC-1..5, FR-193-AC-1..4 and TC-627, TC-628, TC-640 (PR #1118)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@c04bde8337d3005ee606c6900b0e427940d7b824; spec/functional/FR-192-*.md (Behavior incl. #1098 Expansion rows, AC-1..AC-5), spec/functional/FR-193-*.md (Behavior, AC-1..AC-4), spec/functional/FR-189-*.md and FR-191-*.md (context); QSpec FR-408, FR-409, FR-410 (read by id); qsl-replay/src/statistical/settle.rs (tests), qsl-analyze/src/statistical/tests.rs, qsl-eval/src/simulation/sampled/replay.rs (tests)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-192
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-193
    type: reviews
---
# SR-12236: Gap analysis of QSL-571 E17d against FR-192-AC-1..5, FR-193-AC-1..4 and TC-627, TC-628, TC-640 (PR #1118)

## Summary

Ticket: QSL-571. Planless. Plan completion: not assessed. The traceability below comes from the trace tags in the diff. No `quoin matrix` run was made, because the brief says no gates.

How the tests bind to the criteria:

- **FR-192-AC-1: untagged, correctly.** The tests are `each_outcome_row_settles_as_the_map_states` and `each_refusal_row_settles_as_the_map_states` (`TC-627`). They assert every row the code has: three Completed decisions with their categories, `max_samples` with setting `statistical.max_samples`, both expansion limits with their configured bounds (4,096 and 512), internal and non-internal unnamed expansion stops with their diagnostics, cancellation, `NotMarkov`, `TotalAboveDrawRange`, `NoTest`, `ConfidenceBeyondPrecision`, the four request refusals, `Internal` and `NotStatistical`. The AC's `setting model_check.meter` clause is not met (SR-12235 FND-004), so the AC tag is withheld. That is honest.
- **FR-192-AC-2: `okamoto_sprt_and_regenerative_results_settle_measured`.** The binding is correct. It checks Okamoto at 5 ms (accepted, interval, FiniteSample, 23,026, one test, α' = 1/100), SPRT at 2 ms (rejected, no interval) and the regenerative `LongRun` (accepted, Asymptotic). The inputs are the FR-189 tests' ADR-024 §7 samplers, which is the same basis as the merged FR-189-AC-1 tags.
- **FR-192-AC-3: `a_rejected_measurement_fails_the_gate_as_a_violation`.** The binding is correct. Both-items and one-violation documents get equal categories and exit codes, the accepted-only document is Success, and zero items have `result: proved`.
- **FR-192-AC-4: not implemented, and not tagged.** There is no EN-4 manifest capability (`probabilistic-satisfaction`, `statistical`), and no `EveryScheduler`/`MissingConfidence`/`ExactOnlyForm` refusal exists, so the map has no such rows (FND-001).
- **FR-192-AC-5: `an_undefined_sample_settles_measured_rejected`.** The binding is correct for the record: rejected, violation, the test's `UndefinedEvaluation` with `DivisionByZero` in the typed verdict, and a failed gate. The wire check is weak (FND-002).
- **FR-193-AC-1 to AC-4: untagged, correctly.** The witness-keeping tests are `the_first_violating_samples_are_kept_in_trace_order`, `witnesses_interleave_across_tests_by_trace_index`, `a_long_run_fraction_keeps_no_witness`, `the_violating_side_follows_the_bound_direction` (`TC-628`) and `an_undefined_sample_is_kept_whatever_max_witnesses_is` (`TC-640`). The replay-core tests are `a_recorded_service_behaviour_replays_and_each_alteration_is_caught` and `a_changed_post_state_disagrees_at_its_step` (`TC-628`). Together they cover the keeping rules and the draw, support and path-probability checks over the real `Service` model. Each AC also needs a model-backed witness, the claim value on replay (UE-5), the `replay_sampled_witness` facade or the `TemporalCounterexample` conversion, and none of those exist yet (FND-001).

## Verdict

**Coverage is adequate for every criterion tagged (FR-192-AC-2, AC-3, AC-5).** No tag claims more than its test asserts. Nothing in this file blocks the merge. FND-001 is a tracking condition: six of the nine criteria are fully or partly open after this PR, so QSL-571 should not close when the PR merges.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Six criteria stay open after this PR: FR-192-AC-1 (the `model_check.*` setting clause, pending the planner's FR-255 decision), FR-192-AC-4 (the EN-4 manifest and the three refusals, which need FR-186's checked claim from QSL-570 PR 2), and FR-193-AC-1 to AC-4 (the model-backed sampler, the replay-side claim evaluation, `replay_sampled_witness` and the `TemporalCounterexample` conversion). The `NotMarkov{…}` payload and the quantile witness's measure value are also open. The PR body lists all of this. Keep QSL-571 open, or move the remainder to a ticket blocked by QSL-570, so the criteria are not counted done. | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md (AC-1, AC-4); spec/functional/FR-193-keep-and-replay-sampled-witnesses.md (AC-1..AC-4) |
| FND-002 | low | FR-192-AC-5's wire assertion checks only `json["cause"]["kind"] == "undefined-evaluation"`. It cannot catch the missing `at`/`cause` (SR-12235 FND-001). When that is fixed, assert the cause's `at` (test, trace, position) and `cause`. | qsl-analyze/src/statistical/tests.rs:1816-1855 |
| FND-003 | low | `witnesses_interleave_across_tests_by_trace_index` never asserts that both tests contribute a witness. If test 1 had no low-trace violating sample, the test would pass for an implementation that keeps witnesses in run order. Assert that `kept` holds at least one witness of each test. | qsl-analyze/src/statistical/tests.rs:1904-1945 |
