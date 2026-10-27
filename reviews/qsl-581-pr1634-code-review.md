---
id: SR-1634001
title: "code-review and rust-review of PR #1634 (QSL-581d, FR-237 timed replay and FR-235 timed settlement)"
type: SpecReview
analysis: code-review
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1634 git diff origin/qsl-581c-bounded-window...6c026dace: qsl-replay/src/model_check/replay/timed.rs, qsl-replay/src/timed/{dbm.rs,explore.rs}, qsl-replay/src/model_check/{replay.rs,counterexample.rs,limits.rs}, qsl-replay/src/{proof_result.rs,outcome.rs}, qsl-analyze/src/zone_check/settle.rs, qsl-analyze/tests/it/zone_replay.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-237
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-235
    type: reviews
---
# SR-1634001: code-review and rust-review of PR #1634 (QSL-581d, FR-237 timed replay and FR-235 timed settlement)

## Summary

Ticket: QSL-581. Head reviewed: 6c026dace (`qsl-581d-timed-replay`), scoped to the diff against `origin/qsl-581c-bounded-window`. The review was led by soundness: can replay accept a counterexample the model cannot produce, and can the FR-235 map settle more strongly than its evidence allows?

What I traced and found correct:

- **Prefix replay.** Every clock starts at 0. A delay is checked by `TimedSystem::delay` against urgency, the tick source and the state's invariants (upper bounds, so the end point is enough). The guard is checked as a convex part at the delayed valuation. The successor is chosen by the FR-236 digest, clocks included. Target invariants are checked after the resets, and stamps are summed exactly.
- **Time-lock arms.** The local arm needs no positive admissible delay, no enabled step (targets' invariants included), and a dense source. Everything else goes to the fresh exploration.
- **DBM (`qsl-replay/src/timed/dbm.rs`).** `constrain` does the emptiness test `bound + D(j,i) < (<=0)`, then an O(n²) tightening through the new edge, which keeps the matrix canonical. `up` drops the `D(i,0)` cells. `reset` computes `D(c,j) = v + D(0,j)` and `D(j,c) = D(j,0) - v`. `Extra_M` is the textbook rule followed by a Floyd–Warshall close. `Bound`'s order and `add` are right on strictness. Emptiness stays sticky through `(0,0)`. There is no inclusion test (FND-011).
- **Exploration (`explore.rs`).** Under a dense source it builds the zone graph from the point zone, with Tripakis's `z` clock reset after each unit (`M_z = 1`). One global `M` covers every guard, invariant and reset constant and the start's clock values. A reachable quiescent state, or an SCC holding a `z`-reset edge, counts as divergence. This is exact for diagonal-free models, so both `Locked` (time-lock reproduces) and `Divergent` (endpoint admitted) can be trusted. A probe over `Lock` with `ping` guarded by `x >= 1`, `x >= 2`, `x > 1` and `x < 1` (with a reset, under `x <= 1`) gave reproduced, inconclusive, inconclusive and reproduced, all correct.
- **Wiring.** `replay_model_trace` sends a payload to `timed::settle` whenever the recompiled package has a time source with a timed model (replay.rs:646-654). Only the end-to-end test waits on QSL-580.
- **Dependency.** qsl-replay depends on no qsl-analyze code and has its own DBM, so there is no cycle and FR-237's "never EN-6's" holds.
- **The STD-493 gap 2 claim.** It is confirmed and conservative. Every positive stutter delay at a quiescent state is a real behaviour, so a unit-delay idle tail never reproduces a violation the model cannot make. It can only miss one.
- **FR-235 map.** `settle_zone_check` matches with no `_` arm. `Proved` needs the check to return `Accepted`, `Refuted` needs a `Reproduced` replay against the same `package_id`, and a time-lock gets `closed-scope`.

Two soundness defects let replay accept a lasso that is unfair over time (FND-001, FND-002). I reproduced both with temporary probe tests in the review worktree, then reverted them. Over a quiescent state at `x = 0` where `retry` (guard `x >= 5 ms`) becomes enabled and stays enabled, the idle-tail counterexample to `eventually holds(c.retried)` gives these results:

- under `fair weak retry`: `UnfairLasso`, which is right;
- under `fair strong retry`: `Reproduced`, which is wrong.

With `go` posting `phase = 1 or phase = 2` (one transition identity, two successors) and `fair weak retry` enabled at phase 2, both idle-tail payloads reproduce. The phase-2 one is weakly unfair.

Gates, run with locked-build at jobs 4 and `--all-features`: `zone_replay` 10/10 passed. `clippy -p qsl-replay -p qsl-analyze --all-targets -D warnings` exited 0. The qsl-replay lib `timed` tests passed 3/3.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Strong fairness over a terminal-stutter idle tail is checked only in the first stutter window. `enabled_somewhere` compares the step's enabling interval's lower end with one `stutter_delay` (1 or the period), but on the idle tail the clocks keep growing, so a step enabled later (for example guard `x >= 5`) is enabled infinitely often and never taken. Repro: idle tail at `x = 0`, `fair strong retry; eventually holds(c.retried)` gives `Reproduced`, while the same claim under `fair weak` correctly gives `UnfairLasso`. Replay therefore accepts an unfair counterexample, and FR-235 settles it `refuted`. Fix: for a terminal stutter, test whether any interval is unbounded above or reaches past the clocks' shift beyond the model's constants (as `weakly_fair` does with `past`). In practice, a constrained step whose enabling interval is non-empty at some delay `>= 0` is enabled on the idle tail. | qsl-replay/src/model_check/replay/timed.rs:446-478 |
| FND-002 | high | `unfair` rebuilds the behaviour with `TimedBehaviour::advance`, which takes the first enabled step with the transition identity. It does not take the one whose post-state digest is recorded. An operation with a nondeterministic post has several successors under one identity, so fairness is judged on a different path from the one replayed (the untimed path uses the digest-selected positions). Repro: `go` posts `phase = 1 or phase = 2`, and `retry` is enabled at phase 2 under `fair weak retry`. Both idle-tail payloads reproduce, but the phase-2 one is weakly unfair. Fix: build the fairness behaviour from `replay_prefix`'s positions (keep the selected `GuardedStep` per position), or give `advance` the recorded post digest. | qsl-replay/src/model_check/replay/timed.rs:362-386; qsl-eval/src/simulation/model/timed.rs (TimedBehaviour::advance) |
| FND-003 | medium | `close_loop` requires the loop's last timed state to equal its entry exactly (the digest includes the clocks). FR-236's timed lasso only requires the entry's discrete state, reset clocks at their entry values, and unreset clocks above the largest constant at entry, so a clock that runs free across the loop is allowed. A conformant liveness lasso with a free-running clock would be refused `LoopOpen` and settle `ReplayRefused`. This is latent today, because EN-6 emits only terminal-stutter lassos, but it lands with QSL-583. Fix: implement FR-236's closure (discrete digest equal; reset clocks equal; unreset clocks above `M` at entry), and evaluate and check fairness with the shift. | qsl-replay/src/model_check/replay/timed.rs:307-347; spec/functional/FR-236-carry-exact-rational-delays-in-a-timed-counterexample.md |
| FND-004 | medium | `timed_subject` takes the first time source in the package that has a timed model, not the one for the model the replayed subject is over. In a package with more than one model, an untimed subject's counterexample is replayed as timed (and refused `MissingDelay`), or one timed model's payload is replayed under another's clocks. Fix: select the time source by the subject's model alias, carried in `ModelTraceSubject` or derived from the resolved clause. | qsl-replay/src/model_check/replay/timed.rs:50-65 |
| FND-005 | medium | `replay_timed_counterexample` is a public, re-exported entry point that skips FR-128's recompile, identity, fairness-set and admission checks, and it returns no `package_id`. Its only callers are tests, which build the `ModelTraceReplayResult` with a made-up `package_id`, so `settle_zone_check`'s stale-package check is vacuous on that path. A production caller using it would settle `refuted` with no identity binding. Fix: make it `#[doc(hidden)]` and gate it behind `test-support`, or have it take and check the recompiled package. | qsl-replay/src/model_check/replay.rs:659-687; qsl-replay/src/lib.rs:207 |
| FND-006 | medium | The map adds rows and a wire cause that FR-235 does not list. `InconclusiveCause::DeadlockAfterDelay` (`deadlock-after-delay`) is not among FR-235's (or QSpec FR-331's) causes. `Undecided(NoInitialState)` and `Undecided(UndecidedSuccessor)` map to rows FR-235's table does not have. A record carrying `deadlock-after-delay` may not validate against QSpec. Fix: file the cause as a spec gap on STD-493 with a QSpec owner, or map that outcome to an existing cause until the spec names one. | qsl-replay/src/proof_result.rs:662-671,833-835; qsl-analyze/src/zone_check/settle.rs:193-199 |
| FND-007 | low | `enabled_somewhere` ignores the open/closed lower end (`(d, ...)` with `d` equal to the window is treated as enabled). Under a tick source, the window delay includes the tick period although only delay 0 is admissible. Both over-report enabledness, so they give false `UnfairLasso` refusals. Fix: compare with the interval's closedness and subtract the tick, as `weakly_fair` does. | qsl-replay/src/model_check/replay/timed.rs:459-474 |
| FND-008 | low | FR-237 says the delay refusal names "the time invariant or urgency that forbids it" and `NotEnabled` names "the guard". `DelayNotAdmissible.bar` is a stringly `&'static str` category with no invariant name, and `NotEnabled` names only the transition. Fix: make `bar` an enum that carries the invariant name, and add the guard clause to `NotEnabled` on the timed path. | qsl-replay/src/model_check/replay/timed.rs:275-301; qsl-replay/src/model_check/replay.rs:160-170 |
| FND-009 | low | On the timed path, a `final_delay` on a non-`TimeLock` kind and a `lasso` on a `TimeLock` are silently ignored. FR-236 says `final_delay` is `None` on every other kind, and the untimed path refuses a stray `final_delay` (`UntimedDelay { step: None }`). Fix: refuse both as `invalid_runtime_input`. | qsl-replay/src/model_check/replay/timed.rs:99-114,673-676 |
| FND-010 | low | The timed path always returns no FR-133 strong-fairness hints (`Vec::new()`), even for a reproduced lasso, while the untimed path computes them. Either compute them over the timed loop or record why they do not apply. | qsl-replay/src/model_check/replay/timed.rs:99-171 |
| FND-011 | low | `Dbm` has no inclusion test, so the exploration keys nodes on exact zone equality. That is sound, and `Extra_M` keeps it finite, but the graph can be larger than with subsumption, and FR-245's checker (QSL-582), which the module doc says builds on this DBM, will need inclusion. Add `includes` (cell-wise `<=` on canonical forms) with a unit test now, or note it on QSL-582. | qsl-replay/src/timed/dbm.rs:102-246 |
| FND-012 | low | Small Rust nits. `ModelTraceRefusal::TimedSubject(String)` stringifies a typed `TimedSubjectRefusal`. `stopped` re-defines `widen`, which `explore.rs` already has. `explore_ticks` calls a LIFO stack `queue`. `delay_by` picks its label with `is_urgent(..).unwrap_or(false)`, which hides a stop. | qsl-replay/src/model_check/replay.rs:190-193; qsl-replay/src/model_check/replay/timed.rs:288,722-724; qsl-replay/src/timed/explore.rs:174-178 |

## Verdict

**Not mergeable** until FND-001 and FND-002 are fixed. Each one lets replay reproduce a lasso that the model's fairness excludes, so FR-235 would settle a claim `refuted` that holds under its fairness set. Add a regression test for each one (the probes above). FND-003 to FND-006 are medium: fix them in the same round, or route FND-003 to QSL-583 with a note on the ticket. FND-007 to FND-012 are low.

Rust lane: production paths have no `unwrap`, `expect` or panicking index (`positions.len() - 1` is guarded by the initial position). Integer conversions go through `try_from`, and the exploration is bounded by `max_states` and `max_transitions`, checked before insertion. Exact arithmetic throughout: `scaled` divides exactly, because the factor is the LCM of every denominator. The non-recursive Tarjan is correct, and a unit test covers it.

## Coverage

Examined: FR-237 Behavior (prefix, lasso, admitted endpoint, evaluation, undefined evaluation, arm selection, non-local time-lock, determinism); FR-235 Behavior table rows V-1, V-4, V-10, V-6, V-7, V-8 and the `category` and method rules; FR-236 timed lasso and `final_delay` rules (context); FR-232 fairness over time (context). Gap analysis is SR-1634002. Probes ran in the reviewer's detached worktree and were reverted (`git status` clean).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | low | `Zone::extrapolate`, added to the certificate checker's zone in 2ebdc17a8, is the one `Zone` operation with no Kani harness and no differential test against `zone/reference.rs`; it is covered by one unit test. It also reports a `max` of the wrong length as `ZoneError::LuLength { lower: max.len(), upper: max.len(), .. }`, an error named for the aLU bounds. The existing harnesses and reference are unchanged (the added `Ord`/`PartialOrd` derive is additive), so nothing is weakened. Fix: add `extrapolate` to the reference model and a Kani harness (or a differential proptest), and give it its own error variant. | qsl-replay/src/certificate/zone.rs:402-445; qsl-replay/src/certificate/zone/tests.rs:244-290 |

## Dispositions

Round 1, final head 2ebdc17a8 (fix commits 258cb294c, 47446f27e and 2ebdc17a8 on 581c cc42ace8c). I checked each fix in the code in a fresh detached worktree, with its own target, under locked-build at jobs 4 and `--all-features`. I re-ran both round-0 probes at 47446f27e and again at 2ebdc17a8, then reverted them:

- **Idle tail at `x = 0`, `retry` guarded `x >= 5`:** strong fairness gives `UnfairLasso`, weak gives `UnfairLasso`, and no fairness gives `Reproduced`.
- **`Lock` with `ping` guarded `x >= 1` / `x >= 2` / `x > 1` / `x < 1`:** Reproduced, Inconclusive, Inconclusive and Reproduced, unchanged after the DBM swap.

A third probe tried a claim that reads a free-running clock past the model's largest constant; S3 refused it at check time (`ill_typed`/`type-mismatch`), and FR-230 forbids reading a clock outside a constraint, so a model-only `max_constant` is sound for the relaxed loop closure.

At 2ebdc17a8:

- `zone_replay`: 18/18 passed (19 with the probe).
- qsl-replay lib tests: 889/889 passed.
- `clippy -p qsl-replay -p qsl-analyze -p qsl-eval --all-targets -D warnings`: exit 0.
- `fmt --check`: exit 0.

Kani was not run, because `proofs.rs` and `reference.rs` are unchanged.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `TimedSystem::strongly_fair` reads the steady repetition (never-reset clocks shifted past every constant, as `weakly_fair` does), so the idle tail whose `retry` opens at 5 is unfair. Regression test: `strong_fairness_reads_the_whole_idle_tail`. |
| FND-002 | fixed | Fairness is now judged on `behaviour_of`, which rebuilds each step with `advance_where`, matching the transition and the recorded post-state digest. Regression test: `fairness_is_judged_on_the_recorded_successor`. |
| FND-003 | fixed | `close_loop` now applies FR-236's closure: equal discrete digest, each reset clock at its entry value, and each never-reset clock above `max_constant` at entry. Test: `a_loop_closes_with_a_free_running_clock` (closes at `y = 2`, refuses at `y = 1`). |
| FND-004 | fixed | `ModelTraceSubject.timed: Option<TimedReplay { model, exploration }>` names the model alias, and `replay_model_trace` builds `TimedSystem::new(system, &reading.model)`. A runtime test through `replay_model_trace` waits on QSL-580 (580d), as already stated. |
| FND-005 | fixed | `replay_timed_counterexample` is `#[cfg(feature = "test-support")] #[doc(hidden)]`, and only qsl-analyze's dev-dependencies enable the feature. |
| FND-006 | fixed | `DeadlockAfterDelay` is removed from `InconclusiveCause`. EN-6's `DeadlockAfterDelay` now settles `LassoNotConcretized`, and `InsufficientData` settles `UndecidedSuccessor`. Both are inconclusive, so neither is stronger than the evidence, and both are filed on STD-493 (comment 25a29616). `NoInitialState` and `UndecidedSuccessor` are FR-127's V-6 rows. |
| FND-007 | fixed | The window instants come from `enabled_by_window`, which intersects each closed/open interval with `[0, delay]`, or with `[0, 0]` under a tick source. |
| FND-008 | fixed | `DelayBar` is now an enum that names the invariant (`broken_invariant`), and `TargetInvariant` names the invariant. The new `GuardFails` names the precondition clauses. It is backed by the spec: FR-237 says "refusing `invalid_runtime_input`/`invalid-value` naming the step and the guard". |
| FND-009 | fixed | A stray `final_delay` is refused `StrayFinalDelay`, and a time-lock lasso is refused `TimeLockLasso`. Both are tested in `malformed_timed_counterexamples_are_refused`. |
| FND-010 | fixed | `timed_hints` reads FR-133's hints over time through `loop_enabled`. The test expects one strong `retry` hint at position 0. |
| FND-011 | fixed | Moot. `timed/dbm.rs` is deleted, and the exploration now uses the FR-245 checker's `certificate::zone::Zone` (which has `includes` and Kani harnesses). Its new `extrapolate` is correct (`c > M_i` becomes `∞`, `c < -M_j` becomes `(-M_j, <)`, then close, and an empty zone stays in its one form), and `qsl_eval::fairness::components` returns every SCC, singletons included. The residual gap is FND-013. |
| FND-012 | fixed | `TimedSubject` now carries the typed `TimedSubjectRefusal`. There is one `widen`, the variable is named `stack`, and the `unwrap_or(false)` is gone. |

Round 2, head 93f9832a8 (fix commit 93f9832a8 on 2ebdc17a8). I verified this round in a fresh detached worktree with its own target.

- **Kani:** under `locked-build --kani`, `extrapolate_dim_1`, `extrapolate_dim_2` and `extrapolate_dim_3` all verified (3/3, 0 of 1,809 checks failed).
- **Tests:** zone lib tests passed 4/4, including the 600-zone differential test.
- **Clippy:** `-p qsl-replay --all-targets -D warnings` exited 0.

What the harness actually proves. The reference `extrapolate` restates the same entry-wise Extra_M rule on plain bounds, so pointwise agreement shows the encoding and the canonical closure keep the rule's exact point set. It does not prove the rule independently. The independent parts are these:

- **Soundness.** No point of the zone is lost, so extrapolation only widens, which is the property the exploration relies on.
- **Canonicity.** Closing the result changes nothing.
- **Scope.** Points lie on a 1/6 grid up to ±32, so strict and non-strict bounds are told apart. Dimensions go up to 3, with bounds in [-8, 8].

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-013 | fixed | `reference::extrapolate` was added, and the Kani harnesses `extrapolate_dim_1`, `extrapolate_dim_2` and `extrapolate_dim_3` check pointwise agreement with the reference, no lost point and canonicity; I re-ran them and all three verified. `extrapolation_agrees_with_the_reference_on_a_grid` runs the same checks on 600 seeded zones. A wrong-length `max` is now `ZoneError::ExtrapolationLength { max, clocks }`. |
