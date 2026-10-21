---
id: SR-12274
title: "Gap analysis of QSL-878: TP-2 closure certificate, counted past windows, PB-3 zero-progress refutation (PR #1158)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ea592e8308a7b01beacd51952f7421e5f2e8a7c8; FR-338-AC-5, FR-338-AC-6, FR-338-AC-7, FR-213-AC-3, TC-525, TC-658; qsl-analyze/tests/it/{bounded_closure.rs,protocol_counted_steps.rs}, qsl-eval/tests/it/{automaton_key_bounded.rs,property_automata.rs,temporal_trace.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-213
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-658
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-525
    type: reviews
---
# SR-12274: Gap analysis of QSL-878 (PR #1158)

## Summary

Ticket: QSL-878. PR quire-spec-language#1158, head ea592e830. Plan completion: not assessed.

The bindings were found by searching the diff for `#[trace(...)]` and reading each test body against the AC text on the PR head. The focused test run is recorded in SR-12273 (16 passed).

## Verdict

**No major gap.** FR-213-AC-3 is now met in full: `eventually[0,2] holds(k.v >= 2)` over the repaired `Fill` settles `proved` through the core checker, and the refutation half still replays. FR-338-AC-6's bounded-monitor vectors are asserted on exact certificates, written from the Counter arithmetic rather than from the engine. The non-progress vector is asserted for starvation under `scheduling adversarial`, for its exclusion under the default scheduler, and for the checker's `WitnessFails`. AC-6's paired "counted progress" and "discharged debt" vectors are missing (FND-001).

The PR's remaining-work list matches QSL-878's three items and is honest. Still open: past-over-future operators (item 1); keys for general tableau, liveness and Büchi states (the rest of item 2); and AC-6's `on each`, lower-bound and auxiliary-exhaustion clauses and AC-7's general key facts (item 3). QSL-878 must stay open.

Bindings checked:

- `bounded_closure::a_bounded_eventually_that_is_met_carries_a_certificate_the_core_accepts` → FR-338-AC-6: correct (false then true advances to accept, with exact certificate, `Accepted`, `Proved`).
- `bounded_closure::a_bounded_eventually_that_is_missed_is_refuted` → FR-338-AC-6: correct (false then false rejects).
- `bounded_closure::the_terminal_false_extension_closes_the_last_region` → FR-338-AC-6: correct.
- `bounded_closure::a_bounded_certificate_missing_a_reached_state_is_rejected_at_it`, `the_positions_of_the_horizon_are_read_after_the_monitor_accepts`, `a_stopped_bounded_check_names_what_stopped_it` → FR-338-AC-6: correct as supporting rejection and stop vectors.
- `protocol_counted_steps::the_model_checker_counts_steps_over_the_repaired_fill` → FR-213-AC-3: correct, now full.
- `protocol_counted_steps::an_uncounted_cycle_that_keeps_an_interval_open_refutes_it_under_adversarial_scheduling`, `an_uncounted_cycle_that_a_thread_can_leave_is_not_a_fair_behaviour`, `a_closure_with_a_fair_uncounted_cycle_is_rejected_as_a_proof` → FR-338-AC-6: correct for the starvation half (FND-001 for the other half).
- `temporal_trace::a_lasso_with_an_uncounted_step_is_read_in_counted_steps`, `a_loop_without_a_counted_step_meets_no_interval_that_needs_one` → FR-338-AC-6: correct (zero-distance steps read letters without reducing debts).
- `automaton_key_bounded::eventually_one_on_origin_walks_to_accept_and_to_reject`, `a_step_that_does_not_count_reads_its_letter_without_reducing_the_debt`, `always_carries_its_extent_and_discharges_at_the_closed_boundary`, `the_terminal_false_extension_closes_the_open_region` → FR-338-AC-6: correct. These are FR-338's own vector table, with literal octets.
- `automaton_key_bounded::a_u64_max_horizon_stays_exact` → FR-338-AC-7: correct ("u64::MAX bounds ... remain exact").
- `automaton_key_bounded::a_bounded_key_is_admitted_against_its_own_context_only` → FR-338-AC-5: correct.
- `automaton_key_bounded::another_shape_has_no_bounded_translation` → FR-338-AC-7: wrong (FND-002).
- `protocol_counted_steps::a_past_window_is_measured_in_counted_steps`, `property_automata::a_past_interval_over_counted_steps_answers_to_counted_distance`, `property_automata::a_counted_lasso_answers_to_counted_distance` → FR-338-AC-6: wrong. AC-6 says nothing about past windows or differentials (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-338-AC-6 requires "the non-progress lasso vectors distinguish starvation from actual counted progress without invented fairness". FR-338 gives the paired oracles that "prevent rejecting every pending clock indiscriminately": a cycle that includes a counted edge passes no non-progress check, and a zero-counted loop whose `eventually` has already discharged "does not produce a false non-progress rejection". Only the starvation side is tested. The default-scheduler test separates by fairness, not by progress. So a checker that recorded zero edges between any product states, `Accept` included, or that rejected every reachable uncounted cycle under `scheduling adversarial`, would pass every test. Fix: under `scheduling adversarial`, add (a) a protocol whose only cycle takes a counted step (for example a `send`/`receive` loop) and (b) a zero-counted cycle reached after the debt is met (for example the dup/lose channel with the attempt before the `send`). Assert that EN-1 settles `proved` and that `check_protocol_closure` returns `Accepted` for each. | qsl-analyze/tests/it/protocol_counted_steps.rs:593-704, qsl-replay/src/certificate/closure.rs:725-745 |
| FND-002 | low | Some trace tags claim criteria their tests do not assert. `another_shape_has_no_bounded_translation` is tagged FR-338-AC-7, but AC-7 is about keys retaining facts and exact bounds, and this test asserts a translation refusal. The counted past-window tests and the counted-lasso differential are tagged FR-338-AC-6, whose text names no past window or automaton-evaluator differential. The PR body also maps FR-338-AC-7 to "past-window and counted-lasso agreement", which AC-7 does not say. Fix: tag those four tests `TC-658` (or `TC-525`) only, and correct the PR body's AC list. | qsl-eval/tests/it/automaton_key_bounded.rs:306-330, qsl-analyze/tests/it/protocol_counted_steps.rs:556-557, qsl-eval/tests/it/property_automata.rs:450-451, qsl-eval/tests/it/property_automata.rs:479-480 |
