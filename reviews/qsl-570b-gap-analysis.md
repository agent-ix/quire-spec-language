---
id: SR-12246
title: "Gap analysis of QSL-570 part 2a against FR-189-AC-1..14, FR-190, FR-193-AC-1 and TC-624, TC-625, TC-628, TC-640 (PR #1132)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@44fb756e505881713c424173262a365a151934ff; spec/functional/FR-189-*.md (Behavior, AC-1..AC-14), FR-190, FR-191, FR-192, FR-193 (AC-1..AC-4), FR-126, FR-196 (context); ADR-018 SM-1/SM-4, ADR-024 PM/PF/ST/SV rows; qsl-analyze/tests/it/statistical_model.rs, qsl-analyze/src/statistical/tests.rs, qsl-replay/src/statistical/settle.rs (tests)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-193
    type: reviews
---
# SR-12246: Gap analysis of QSL-570 part 2a against FR-189-AC-1..14, FR-190, FR-193-AC-1 and TC-624, TC-625, TC-628, TC-640 (PR #1132)

## Summary

Ticket: QSL-570. Planless. Plan completion: not assessed. The traceability below comes from the trace tags in the diff. No `quoin matrix` run was made, because the brief says no gates.

How the tests bind to the criteria:

- **FR-189-AC-1, `p95_over_service_is_decided_from_the_models_own_behaviours`.** The binding is correct. The test checks Okamoto `N = 23,026`, one test, Accepted, half-width `1/100` (high − low = `2/100`), and lower end `>= 19/20`. At 2 ms it is Rejected with upper end `< 19/20`. SPRT decides both the same way with fewer samples and no interval. The verdicts are real under seed 7: the true p95 probability (0.969) is 0.019 above `θ`, nearly twice `ι`. "Each of at most 11 positions" is checked only on the kept witnesses (FND-002).
- **FR-189-AC-6, `a_dropped_coin_rejects_its_test_at_the_undefined_position`.** The binding is correct. The test checks Rejected, test 0, `trace = draws − 1` (no later draw), the witness's last step a `drop` with `b = 0` and every earlier step `b = 1`, position = step count, an `Atom` locus, `AbsentKey`, the undefined witness kept, and the claim Rejected.
- **FR-189-AC-7, `a_draw_above_the_range_is_unsupported`.** The binding is correct. The test checks `TotalAboveDrawRange` with the exact decimal `2^256 + 1`, one trace, and the settled cause `total-above-draw-range`.
- **FR-193-AC-1, `the_two_ms_claim_keeps_its_first_violating_behaviours`.** The binding is correct. The test checks three witnesses in increasing trace order, each with latency above 2 or censored, path probability = the product of its steps, `request` probability 1, and at least one first `attempt` with `d = 3`, `ok` at `343/5000`. "The first in trace-index order" is held by PR 1's seam test `the_first_violating_samples_are_kept_in_trace_order`.
- **Untagged, correctly (partial coverage):** FR-189-AC-4's activation clause (samples 23,026 and draws > samples), FR-189-AC-12's stop (an activation that never holds, `MaxDraws` 1,000, 0 samples), FR-189-AC-5's `NotMarkov` clause (post-states 2 and the scheduled identity), FR-195/FR-192 gate refusals through `check_statistical`, a mean of a fraction (`N = 116`), a regenerative long-run fraction (no witness), and the generator check. Each tag is TC-only because its AC has clauses this PR does not exercise.
- **Seam tests (PR 1), retargeted.** `statistical/tests.rs` now uses `check_sampled`/`SampledRequest` and `SampleHalt::NotMarkov(Box<..>)`. The quantile witness tests now assert `WitnessValue::Measure(Value(latency))` against the recorded latency per trace, which is stricter than the old `Event(false)`.

Layer split, checked against what is blocked:
- FR-193-AC-2..4 (replay, conversion to a counterexample) and the FR-189/FR-192-AC-4 manifest are declared next-PR. Both are genuinely blocked. Replay needs the SM-1 reading reachable from qsl-replay, which cannot depend on qsl-analyze (an architecture decision). The manifest needs `Capability` labels and `BackendDescriptor.evidence_kinds`, which are route-layer work.
- FR-189-AC-3 (two initial states and symmetry orbits), FR-189-AC-5 (undecided successor on a real model) and FR-189-AC-9/10/13/14 on real models are in the PR's Remainder with no owning PR. AC-9/10/13/14 are seam criteria that PR 1 already covers. AC-8 is blocked by a spec defect (`1 / (1 - z.v)` does not type in a model unit, as noted on #1122). The symmetry rule and AC-3's two-initial-state fixture are not blocked (FND-001).

## Verdict

**No major in the gap analysis.** Every tag binds to a test that asserts its AC's clauses with real, seeded verdicts, and no tag over-claims. The blocking defect of this PR is in SR-12245 (FND-001, bounded event not closed at a terminal state). It is a coverage gap too: no test drives a bounded event into a terminal state (FND-003 below). The remaining gaps are mediums and lows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Unblocked FR-189 work has no owning PR. The "Tests" symmetry rule (one test per orbit, `m` counts orbits, ADR-021 SYM-5) has no code. FR-189-AC-3's two-initial-state variant needs only a second `prob_world` snapshot, test code. FR-189-AC-5's undecided-successor clause on a real model is untested at the model level. QSL-570 declares two PRs, and only FR-193-AC-2..4 and the manifest are named for the next one. Name these in part 2b, or file a ticket, so that FR-189 does not close with them silently open. | qsl-analyze/src/statistical/model.rs:124-134; qsl-analyze/tests/it/prob_world.rs |
| FND-002 | low | FR-189-AC-1's "each of at most 11 positions" is asserted only on `accepted.witnesses` (at most 8 samples), not on the 23,026 samples. A sampler that over-drew on a sample that is not a witness would pass. A per-test draw check is not possible through the verdict. A unit check on `Shared::sample`'s step count for a horizon-10 measure would close it. | qsl-analyze/tests/it/statistical_model.rs:124-126 |
| FND-003 | medium | No test drives a bounded event (FR-189 "Per sample", ADR-018 SM-4) into a terminal model state inside its horizon, so SR-12245 FND-001 is not caught. Add a model-backed test over a model with a terminal state (for example `process` or the `Stop` fixture in `prob_world`). It should assert the closed-boundary value, and agree with EN-5's value for the same claim. | qsl-analyze/tests/it/statistical_model.rs |
| FND-004 | low | The disclosed choices have no test: a probability bound over a measure that is not activated (choice 1), a zero-total weighted fraction (choice 2), a weighted fraction's `h + 1` draws (choice 3) and a bounded event over the automaton budget (choice 4). Once the owner rules on SR-12245 FND-002 to FND-004, each needs a test that pins the ruling. | qsl-analyze/src/statistical/model.rs:694-758 |
| FND-005 | low | FR-192 "estimate" for a quantile claim reports "the empirical nearest-rank quantile of the counted samples beside it". This PR is the first to produce real measure values (`Sample::Measured.value`), but the tally keeps none, and the Remainder does not list the item. FR-192 has no AC on it, so it is a low. Add it to part 2b, or to QSL-571. | qsl-analyze/src/statistical.rs:530-547 |

## Dispositions

Round 1, reviewed at 640e6fdff (fix commit fdb1ee815, then a merge of origin/main).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | deferred | Coordinator: the symmetry orbits and FR-189-AC-5's undecided successor on a real model move to QSL-570 part 2b / QSL-571 and will be noted on QSL-570. The two-initial-state clause of FR-189-AC-3 was done in fdb1ee815 (`World::with_initials`, `two_initial_states_run_two_interleaved_tests`: m = 2, α' = β' = 1/200, trace stride 2, witnesses of test j at trace ≡ j mod 2). |
| FND-002 | fixed | fdb1ee815. FR-189-AC-1 now runs `Zero` (`<= 0`, which every activated sample violates) over the same seed and trace indices with `max_witnesses` `u32::MAX`. It asserts 23,026 witnesses, each with at most 10 steps, so every sample's position count is checked. |
| FND-003 | fixed | fdb1ee815. `a_bounded_event_closes_at_a_terminal_state` drives a bounded event into `Process`'s terminal state inside the horizon and asserts the closed value (Rejected, estimate near 9/16, no stutter step after the terminal state). |
| FND-004 | fixed | fdb1ee815. Choice 1 is pinned by `an_unactivated_measure_under_a_probability_bound_counts_against_it`, choice 2 by `a_weightless_fraction_is_an_undefined_sample`, and choice 4 by `an_automaton_over_its_limit_stops_and_a_refused_read_refuses` and the settle test. Choice 3 (the weighted `h + 1`-th draw) is covered only indirectly: the weightless test reads position 0's weight from the step that leaves it. That is acceptable for a low. |
| FND-005 | deferred | Coordinator: FR-192's empirical nearest-rank quantile beside a quantile estimate moves to QSL-570 part 2b / QSL-571 and will be noted on QSL-570. FR-192 has no AC on it. |
