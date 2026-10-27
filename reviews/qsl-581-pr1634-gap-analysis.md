---
id: SR-1634002
title: "gap analysis of PR #1634 (QSL-581d, FR-237 timed replay and FR-235 timed settlement)"
type: SpecReview
analysis: gap-analysis
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1634 git diff origin/qsl-581c-bounded-window...6c026dace; FR-237 AC-1 to AC-6, FR-235 AC-1 to AC-5; TC-690, TC-692, TC-710; qsl-analyze/tests/it/zone_replay.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-237
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-235
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-692
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-690
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-710
    type: reviews
---
# SR-1634002: gap analysis of PR #1634 (QSL-581d, FR-237 timed replay and FR-235 timed settlement)

## Summary

Ticket: QSL-581. Head reviewed: 6c026dace. Method: a manual check of each AC against its tests over the diff, scoped to FR-237 and FR-235 and their TC files. I re-measured the PR's claims in the code and in Linear. Plan completion: not assessed.

| AC | PR claim | Measured |
| --- | --- | --- |
| FR-237-AC-1 | partial | Partial, as claimed. `a_late_reply_replays_with_its_delays` reproduces FR-236-AC-1's counterexample at position 3, and it is correctly tagged `TC-692` only. The `Settles` half needs TT-3/MightyL, and QSL-583 records FR-237-AC-1 in its comments. |
| FR-237-AC-2 | covered | Covered: NotEnabled (5/2), DelayNotAdmissible `time-invariant` (7/2), PostState with `stale_dependency`, LoopZeroDelay, and LoopOpen. |
| FR-237-AC-3 | covered | Covered: the local time-lock reproduces, and the `x >= 3` variant goes to the non-local arm and settles `Inconclusive`. |
| FR-237-AC-4 | deferred to QSL-583 | The deferral is real. QSL-583 ("non-local replay") names FR-237 and records FR-237-AC-4 in a comment. The non-local arm's code runs through AC-3 and AC-6. |
| FR-237-AC-5 | covered | Covered: position 2, stamp 2, `DivisionByZero`, and inconclusive with the last step removed. |
| FR-237-AC-6 | covered | Covered: `Lock` gives inconclusive, the resetting `ping` reproduces, and `max_states` 1 gives MaxStates (V-7). |
| FR-235-AC-1 | partial | Partial, and thinner than claimed (FND-003). `StopwatchRequired` is soundly deferred to QSL-585 (FR-249). |
| FR-235-AC-2 | deferred to QSL-582 | It needs the FR-245 checker, so the deferral is sound, but QSL-582 does not record it (FND-006). |
| FR-235-AC-3 | covered | Covered, through the real replay. |
| FR-235-AC-4 | covered | Covered, with one weak assertion (FND-004). |
| FR-235-AC-5 | covered | Covered: refuted, `decisive-counterexample`, violation, position 2, stamp 2, `DivisionByZero`, and `NoAdmittedBehaviour` for the locked variant. |
| FR-239-AC-4 | deferred to QSL-582/583 | The deferral is recorded on QSL-583 in a comment. |

**Pending end-to-end test.** I confirmed that the core is wired: `replay_model_trace` dispatches to `timed::settle` whenever the recompiled package has a timed model (replay.rs:646-654). Only the test through `replay_model_trace` waits on QSL-580's emission work.

**Spec gap.** STD-493 comments 5b5714c3 (gap 2) and 520d0b26 (gap 2) exist and state the idle-tail stutter delay gap. Its effect on replay is conservative: a unit-delay idle tail is a real behaviour, so a violation that needs another stutter delay settles `inconclusive`. It is never falsely reproduced.

The suite has no test of fairness over time and no tick-source test. That is why SR-1634001 FND-001 and FND-002 (unfair lassos accepted) pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test drives FR-237's "fairness over time" on the timed path: no weak or strong constraint, no `each` granularity, no lasso with a fairness set. The two soundness bugs in SR-1634001 (FND-001: strong fairness on the idle tail; FND-002: nondeterministic successor) pass the suite because of this. Fix: add the two SR-1634001 probes as regression tests, plus a weak and a strong case on a non-stutter timed loop. | qsl-replay/src/model_check/replay/timed.rs:362-478; qsl-analyze/tests/it/zone_replay.rs |
| FND-002 | medium | No test runs under a tick source, so `explore_ticks`, the tick branch of `close_loop`, `DelayNotAdmissible { bar: "tick-source" }` and the tick-period idle tail are all unexercised. SR-1631001 FND-001 showed that the tick path is where timed code diverges. Fix: add a tick-source replay (a reproduced finite violation whose terminal state ticks, and a positive delay refused `tick-source`). | qsl-replay/src/timed/explore.rs:161-207; qsl-replay/src/model_check/replay/timed.rs:281-300,326-329 |
| FND-003 | medium | FR-235-AC-1 says "each table row". These rows have no test: `Undecided(LassoNotConcretized)`; a `Violated` replay that stops (V-7 through `ModelTraceSettlement::Stopped`); and V-4 with `kind: Deadlock`. The V-7 rows assert the value and category but not the FR-360 label `failed`, the execution `resource-incomplete` or the basis `unavailable`. Add the missing inputs to `each_timed_verdict_row_settles_as_stated` and assert label, basis, execution and category on every row. | qsl-analyze/tests/it/zone_replay.rs:413-560; qsl-analyze/src/zone_check/settle.rs:193-264 |
| FND-004 | low | FR-235-AC-4 requires `unsupported`, `PunctualInterval`. The test asserts the label and category but not `record.value == Unsupported(PunctualInterval{..})`, so a wrong cause would pass. | qsl-analyze/tests/it/zone_replay.rs:660-680 |
| FND-005 | low | The `TargetInvariant` and `MissingDelay` refusals (FR-237 "apply the resets and check the target's time invariants" and FR-236's required delay) have no test. Add one tampered payload for each. | qsl-replay/src/model_check/replay/timed.rs:197-202,245-250,673-676 |
| FND-006 | low | FR-235-AC-2's deferral to QSL-582 is not recorded on QSL-582: its description names only FR-245, and no comment mentions FR-235-AC-2. Add a comment on QSL-582 that names FR-235-AC-2, so the AC has an owner of record. | QSL-582 |
| FND-007 | low | The DBM has two unit tests. Neither covers a strict bound in `constrain`'s emptiness test (for example `x < 3` meeting `x >= 3`), `Extra_M`'s lower-bound branch on a two-clock zone, or `reset` to a non-zero value. These are the cells FR-245's checker will rely on. | qsl-replay/src/timed/dbm.rs:248-297 |

## Verdict

**Not mergeable**, together with SR-1634001. FND-001 here is the missing coverage that let SR-1634001 FND-001 and FND-002 through. The author's coverage claims hold for FR-237-AC-2, 3, 5 and 6 and for FR-235-AC-3, 4 and 5 (AC-4 with a weak assertion). FR-237-AC-1 and FR-235-AC-1 are partial; FR-235-AC-1 is thinner than claimed (FND-003). The deferrals to QSL-582, QSL-583 and QSL-585 point at the right owners, except that FR-235-AC-2 is not recorded on QSL-582. The production wiring into `replay_model_trace` is in place, and the STD-493 stutter-delay gap is real and conservative.

## Coverage

Bindings checked from the `#[trace]` attributes:

| Test | Tag | Binding |
| --- | --- | --- |
| `a_late_reply_replays_with_its_delays` | TC-692 | correct (partial AC-1, untagged) |
| `a_counterexample_that_breaks_time_is_refused` | FR-237-AC-2 | correct |
| `a_local_time_lock_reproduces_and_a_live_one_does_not` | FR-237-AC-3 | correct |
| `an_undefined_evaluation_replays_at_its_time` | FR-237-AC-5 | correct |
| `a_violation_on_time_locked_behaviour_does_not_reproduce` | FR-237-AC-6 | correct |
| `each_timed_verdict_row_settles_as_stated` | FR-235-AC-1 | correct |
| `a_timed_refutation_settles_through_its_replay` | FR-235-AC-3 | correct |
| `vacuity_punctuality_and_budgets_settle_as_stated` | FR-235-AC-4 | correct |
| `an_undefined_timed_evaluation_settles_refuted` | FR-235-AC-5 | correct |

## Dispositions

Round 1, final head 2ebdc17a8. I verified each test by reading it and running `zone_replay` (18/18 passed).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Fairness over time on the timed path is now tested by four tests: `strong_fairness_reads_the_whole_idle_tail`, `fairness_is_judged_on_the_recorded_successor`, `a_timed_loop_is_weakly_fair_and_strongly_unfair` (a weak-fair loop that is strongly unfair) and `a_loop_closes_with_a_free_running_clock`. Both round-0 probes are now regression tests. |
| FND-002 | fixed | `a_tick_source_replays_by_its_ticks` reproduces under a 5 ms tick source and refuses delay 1 with `DelayBar::TickSource`. |
| FND-003 | fixed | `the_remaining_timed_rows_settle_as_stated` covers `LassoNotConcretized` (and `DeadlockAfterDelay` mapped to it), a stopped replay (label `failed`, execution `resource-incomplete`, basis `unavailable`) and V-4 `Deadlock` (`refuted`, `decisive-counterexample`). |
| FND-004 | fixed | The test asserts `TerminalValue::Unsupported(UnavailabilityCause::PunctualInterval { span })`. |
| FND-005 | fixed | `malformed_timed_counterexamples_are_refused` covers `MissingDelay` and `TargetInvariant` (naming `Locked`), plus `StrayFinalDelay` and `TimeLockLasso`. |
| FND-006 | fixed | QSL-582 comment a39512f5 takes ownership of FR-235-AC-2, deferred from #1634. |
| FND-007 | fixed | `extrapolation_bounds_the_zones_of_a_growing_clock` now tests the replacement `Zone`'s extrapolation, including the lower-bound branch on two clocks. Strict emptiness, reset and inclusion are covered by `Zone`'s existing tests and Kani harnesses. The new method's own harness gap is SR-1634001 FND-013 (low). |
