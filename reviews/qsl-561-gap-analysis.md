---
id: SR-12258
title: "Gap analysis of QSL-561 E15d: counted protocol steps for interval distance (PR #1142)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@8b2d0d516c8fab432009c599951894515c3e4591; FR-213, TC-658; qsl-analyze/tests/it/protocol_counted_steps.rs, qsl-eval/tests/it/{automaton_kernels.rs,property_automata.rs,temporal_trace.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-213
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-658
    type: reviews
---
# SR-12258: Gap analysis of QSL-561 E15d (PR #1142)

## Summary

Ticket: QSL-561. PR quire-spec-language#1142, head 8b2d0d516. Plan completion: not assessed.

`quoin matrix --json` did not finish within 300 s, so the bindings were found by searching for `#[trace("TC-658", ...)]` and reading each test body against FR-213 and TC-658 on the PR head. FR-213 has nine tagged tests: AC-1 has 5, AC-2 has 1 and AC-3 has 3. The focused run is recorded in SR-12257.

## Verdict

**No major gap.** FR-213-AC-1 and AC-2 are asserted on the spec's own vectors. FR-213-AC-3's refutation half is asserted, including the counted-distance check on the prefix and a replay to `Refuted`. Its `proved` half is not met, and that is disclosed but not tracked (FND-001).

Bindings checked:

- `counted_steps_decide_the_interval_over_the_repaired_fill` → FR-213-AC-1: correct. It asserts the anchors at positions 1 to 5, the step kinds, the counted flags, both verdicts, and a contrast: with every step counted, `[0,2]` fails.
- `counted_steps_decide_the_interval_over_the_compensation_paths` → FR-213-AC-2: correct for the AC as written (but see FND-004).
- `the_model_checker_counts_steps_over_the_repaired_fill` → FR-213-AC-3: partial. It asserts `Holds` and not `proved`, and the refutation half is strong.
- `a_model_subject_counts_every_step` → FR-213-AC-3: partial (FND-003).
- `a_counted_clause_translates_to_a_monitor_that_answers_to_counted_distance` → FR-213-AC-3: supporting. It compares the automaton with the evaluator on every word of up to 4 or 5 letters, which is a strong oracle, but it is not AC-3's Fill vector (FND-002).
- `an_uncounted_step_moves_the_position_and_not_the_distance`, `a_deadline_counter_measures_counted_distance_on_every_short_trace` and `an_extent_counter_measures_counted_distance_on_every_short_trace` → FR-213-AC-1: these test the mechanism, not AC-1's vector (FND-002).
- `a_lasso_with_an_uncounted_step_is_refused` → FR-213-AC-1: wrong. AC-1 says nothing about lassos (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-213-AC-3 requires `eventually[0,2] holds(k.v >= 2)` to "settle `proved`". The test asserts `ModelCheckOutcome::Holds` only, and its doc comment says the `proved` label is not covered, because FR-338's TP-2 closure certificate is not built. The test is still tagged FR-213-AC-3, so the matrix shows AC-3 as fully covered when one of its three clauses fails. No ticket is named for the remainder. Fix: assert the label that `settle_protocol_check` actually returns today, so the gap is visible and flips when the certificate lands. Record `Remaining work: QSL-878` (or the ticket that owns the TP-2 certificate) in the test doc and the PR body, and add a Linear blocks edge, so that QSL-561 is not closed as satisfying AC-3 in full. | qsl-analyze/tests/it/protocol_counted_steps.rs:412-428 |
| FND-002 | low | Some trace tags claim criteria their tests do not assert. `a_lasso_with_an_uncounted_step_is_refused` is tagged FR-213-AC-1, but AC-1 is about the repaired `Fill` and has no lasso clause. The evaluator test and the two counter kernels test FR-213's Behavior mechanism, not AC-1's vector, and the property-automata comparison is tagged AC-3 although it is not AC-3's Fill check. Fix: drop the AC tag from the lasso test, keeping it as a plain test. For the mechanism tests, either drop the tags or keep them only where the test asserts the AC's own claim. | qsl-eval/tests/it/temporal_trace.rs:386-404, qsl-eval/tests/it/temporal_trace.rs:349-351, qsl-eval/tests/it/automaton_kernels.rs:540-542, qsl-eval/tests/it/automaton_kernels.rs:579-581, qsl-eval/tests/it/property_automata.rs:374-379 |
| FND-003 | low | FR-213-AC-3's last clause, and TC-658 step 3 ("the same claim forms over a model subject"), cover both claim forms. The model-subject test checks only `eventually[0,1] holds(k.v = 3)` (`Violated`, prefix length 1). Nothing checks `eventually[0,2] holds(k.v >= 2)` over the model subject. Fix: assert FR-126's verdict for the second form in `a_model_subject_counts_every_step`. | qsl-analyze/tests/it/protocol_counted_steps.rs:466-493 |
| FND-004 | low | FR-213-AC-2's vector does not tell counted steps from positions. On the failure path the uncounted `cend` and `finish` come after the refund, so `[0,2]` holds at position 2 and fails at position 1 under either measure, and the success path fails everywhere under either. The test passes even if `counts_step` is ignored. This is a weakness of the spec vector, not of the test. Fix: a spec amendment (FR-213-AC-2 or TC-658) that adds a position whose verdict depends on an uncounted step, for example `eventually[0,1]` from a position just before `cend`. Route it to the FR-213 spec ticket (QSL-396). No code change is needed in this PR. | qsl-analyze/tests/it/protocol_counted_steps.rs:361-410 |

## Dispositions

- FND-001: fixed. The AC-3 test asserts the label `settle_protocol_check` returns today (`Failed`), cites QSL-878, and QSL-878 now blocks QSL-561.
- FND-002: fixed. The lasso test is untagged by AC; the mechanism tests carry `TC-658` only.
- FND-003: fixed. The model-subject test also checks `eventually[0,2] holds(k.v >= 2)`.
- FND-004: added `an_uncounted_cend_and_finish_end_the_window_after_the_refund`, and commented on QSL-396 for the spec amendment.
