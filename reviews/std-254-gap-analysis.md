---
id: SR-13772
title: 'Gap analysis of QSL PR #1182: FR-329-AC-8 to AC-10 against TC-839 rows and tests (STD-254)'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@7d7a924b7; git diff origin/main...7d7a924b7 (PR #1182, branch std-254-spec): spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md, spec/test-cases/TC-839-s6a-evaluates-an-infinite-trace-clause-over-a-lasso.md; tests under qsl-eval, qsl-analyze, qsl-replay; ticket STD-254'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-329
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-839
  type: reviews
---

## Summary

Ticket: STD-254. PR: quire-spec-language#1182, head 7d7a924b7. Method: gap analysis of the new criteria against their test-case rows and the tagged tests, done by hand. The quoin matrix was not run because of the spec-only gate scope. Plan completion: not assessed.

Each new AC has a TC-839 row. Step 6 and its expected result cover AC-8. Step 7 and its expected result cover AC-9 and AC-10. TC-839 Scope names AC-8 to AC-10.

No test is tagged `FR-329-AC-8`, `-AC-9` or `-AC-10`. The behaviour from QSL #1158 is exercised by these tests:

- `qsl-eval/tests/it/temporal_trace.rs:568` `a_bounded_profile_lasso_without_a_counted_step_is_read_in_distance_zero` (TC-658, FR-338-AC-6). It covers AC-8's semantics with stub letters over a counter fixture, not a protocol subject.
- `qsl-eval/tests/it/temporal_trace.rs:436` `a_loop_without_a_counted_step_meets_no_interval_that_needs_one` (FR-338-AC-6). The same semantics under infinite-trace.
- `qsl-analyze/tests/it/protocol_counted_steps.rs:638` `an_uncounted_cycle_that_keeps_an_interval_open_refutes_it_under_adversarial_scheduling` (FR-338-AC-6). A bounded `eventually[0,2]` over a protocol subject, refuted by a duplicate/lose zero-counted lasso that replays.
- `qsl-analyze/tests/it/protocol_counted_steps.rs:619` (FR-338-AC-6). The same cycle under the default scheduler is unfair, and the claim is proved.
- `qsl-eval/tests/it/temporal_trace.rs:1124` `a_lasso_under_a_bounded_profile_is_refused` (untagged). An observed bounded lasso refuses. It matches the variant only and does not check the catalog code.
- `qsl-replay/src/spine/clause/tests/temporal.rs:196` (FR-330-AC-3). The spine refuses an observed bounded lasso at admit with `invalid_runtime_input`/`invalid-value`.
- `qsl-analyze/tests/it/scheduler_fairness.rs:327` (FR-217-AC-4). `UnfairProtocolLasso` naming a scheduler constraint, under infinite-trace only.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-329-AC-8 | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:172 | examined |
| FR-329-AC-9 | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:173 | examined |
| FR-329-AC-10 | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:174 | examined |
| TC-839 steps 6-7 and expected results | spec/test-cases/TC-839-s6a-evaluates-an-infinite-trace-clause-over-a-lasso.md:32-38,50-52 | examined |
| FR-329 Bounded-profile section | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:134-159 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test is tagged FR-329-AC-8, AC-9 or AC-10, and TC-839 steps 6 and 7 are unimplemented. AC-9's counted-step `Lasso::Model` and AC-10 under a bounded profile have no test of any tag. Both were confirmed only by this review's probe (an unfair bounded lasso refuses `UnfairProtocolLasso{Root}` `invalid-value`). AC-8 is covered only with stub letters or a duplicate/lose loop tagged FR-338-AC-6, not with the protocol fork/join loop the AC names. | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:172-174; qsl-eval/tests/it/temporal_trace.rs:568,1124; qsl-analyze/tests/it/protocol_counted_steps.rs:638 |
| FND-002 | low | AC-8's true half, `eventually[0,1] holds(p)` with `p` true at position 0, is decided at position 0 and reads no loop position. An evaluator that mishandled the zero-counted loop would still pass it. A loop-position witness is a stronger oracle: `p` true only at a loop position, at distance 0, gives `Completed(true)`, and the probe returns `Holds` for that case. | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:172 |
| FND-003 | low | Bullet 1 refuses "every lasso over a model subject", but AC-9's `Lasso::Model` with one counted step does not name its subject, and no AC covers a bounded clause over a model-subject `Lasso::Model`. | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:136-142,173 |

## Verdict

Not clean. Every new AC has a TC-839 row, the trace tags on the existing tests are correct for the ACs they name, and AC-8's semantics are already exercised. The tests for AC-8 to AC-10 as stated still have to be written, and AC-9's refusal code disagrees with the evaluator (SR-13770 FND-001), so the TC-839 step 7 test would fail against the current code.

## Dispositions

Disposition pass 1 at `agent-ix/quire-spec-language@9eb06cde9` (fix commit `9eb06cde9`). I checked each outcome against the spec at that head. The PR changes no test.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred: spec-only PR | No test is tagged FR-329-AC-8, AC-9 or AC-10 and TC-839 steps 6 to 8 are still unimplemented at 9eb06cde9. This PR is spec-only by its gate scope, so the tests belong to the implementation work; no follow-up ticket for them is named on STD-254 yet, and the lead should name or file one before STD-254 closes. |
| FND-002 | fixed 9eb06cde9 | AC-8 adds "with `p` true only at a loop position, at distance 0, `eventually[0,1] holds(p)` is `Completed(true)`", and TC-839 step 6 and its expected result ("`false` at 0; `true`; `true`") carry it. The review-pass probe returned `Holds` for this case. |
| FND-003 | fixed 9eb06cde9 | AC-9 now names "a `Lasso::Observed`, a `Lasso::Model` over a model subject, and a protocol-subject `Lasso::Model` whose loop has one counted step", and TC-839 step 7 evaluates each. |

## New findings (disposition pass 1)

These were found at `9eb06cde9`. They are new, not dispositions of the rows above.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | TC-839 step 8 says "evaluate the step 6 lasso when it is unfair for a scheduler constraint", but step 6 defines that lasso as fair (a `repeat` around a `parallel` whose loop is only `fork` and `join` steps), so step 8 names a lasso that cannot be unfair. AC-10 asks for any zero-counted lasso that is unfair. Fix: name an unfair zero-counted lasso for step 8, for example the duplicate/lose cycle of FR-338-AC-6 replayed against the default package, which the review-pass probe refused `UnfairProtocolLasso{Root}`. | spec/test-cases/TC-839-s6a-evaluates-an-infinite-trace-clause-over-a-lasso.md:40-41,56-57; spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:193 |
