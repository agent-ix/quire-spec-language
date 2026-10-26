---
id: SR-1599001
title: "code-review and rust-review of PR #1599 (QSL-581a, FR-239 zone safety search, FR-241 finite concretisation)"
type: SpecReview
analysis: code-review
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1599 git diff cb95a247a...7ac32b8d7: qsl-analyze/src/zone_check/{search.rs,concretise.rs,dbm.rs,dbm/tests.rs,zone_check.rs}, qsl-eval/src/simulation/model/{timed.rs,model.rs}, qsl-analyze/tests/it/{zone_search.rs,timed_world.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-239
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-241
    type: reviews
---
# SR-1599001: code-review and rust-review of PR #1599 (QSL-581a, FR-239 zone safety search, FR-241 finite concretisation)

## Summary

Ticket: QSL-581. I traced the passed/waiting search, aLU coverage (`alu_covered_by`, which also checks pairs with `x = 0`; that is stricter than the criterion, so it is sound), `down`, `universe`, zone subtraction for deadlocks, the Tarjan SCC Zeno test, and the step-time DBM of the concretiser. The `Locked` verdict is sound: a concrete infinite run maps, through the aLU simulation, to a graph path with the same discrete states and steps, so an SCC with an invariant-bounded clock that no inner edge resets is Zeno. Two defects were reproduced with probe tests, run in my own detached worktree and reverted, never pushed. (1) Tick-source vacuity: over a tick subject whose tick step is disabled forever and whose `work` step loops, `always holds(c.phase = 0)` returns `Holds` (one node, one covering self-loop). (2) Time-locked endpoint: with `go` guarded `x > 2 ms`, `B`'s invariant `x <= 4 ms` and the only escape guarded `x < 3 ms`, the counterexample is `go` after delay 3, ending at `(B, x = 3)`, which has no time-divergent continuation. Coder note on `Violated` for undecided divergence: under ADR-026 TV-1 the final verdict stays sound only because FR-235 settles `Violated` through FR-237 replay, and FR-237's admitted-endpoint arm settles a non-diverging endpoint `inconclusive`, `ReplayParity`. That replay arm is not built yet. The outcome itself breaks FR-239's SHALL and ends the search early (FND-003). Coder note on the new `timed-state` digest and causes: QSpec FR-181 fixes only that the post-state digest covers the discrete state and the exact valuation, not its byte form. The QSL-local `timed-state {state, clocks}` form is consistent because producer and replay share `TimedSystem::state_digest`, but it needs a spec'd form before replay across implementations (FND-006). `DivergenceUnknown` and `DeadlockAfterDelay` have no FR-235 row (FND-005). Focused tests at 7ac32b8d7: qsl-analyze `zone_search`, `zone_check` and `time_lock_item`, and qsl-eval `timed_subject`, all passing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Under a tick source, `divergence()` returns `Admitted` for every node, so the vacuity check always passes. Reproduced: a tick subject whose tick operation is never enabled and whose `work` step loops has no time-divergent behaviour (FR-231 "Digital time": divergence is infinitely many tick steps). Yet `always holds(c.phase = 0)` returns `Holds` instead of `Undecided(NoAdmittedBehaviour)`. That is a vacuous proof reported as proved (TD-6). Fix: under tick, admit only a reachable terminal state (its stutter counts as a tick) or an SCC with a tick-operation edge, or refuse tick subjects at EN-6. | qsl-analyze/src/zone_check/search.rs:856-861 |
| FND-002 | medium | Divergence is judged per symbolic node, but the concretiser targets `Target::Any` for `Formula` and `UndefinedEvaluation`. So the chosen last valuation can be a time-locked point of a node whose other points diverge. Reproduced: `go` after delay 3 ends at `(B, x = 3)`, where the escape `x < 3` is gone and only Zeno `ping`s remain. FR-241 says never a time-locked point (FR-241-AC-5). Constrain the target to the valuations that reach the quiescent or divergent SCC, or check the chosen point and try the next. | qsl-analyze/src/zone_check/search.rs:975; qsl-analyze/src/zone_check/concretise.rs:119-123 |
| FND-003 | medium | `Divergence::Unknown` (an SCC not proven Zeno) ends the search with `Violated`. FR-239 says a violating state SHALL end the search only when a point has a time-divergent continuation by FR-240's test. Node-level `Admitted` through covering edges is also an over-approximation. The settled verdict stays sound only if FR-237's admitted-endpoint arm runs before settlement, and it is not built. Meanwhile a real violation later in BFS order is never reported, and a claim whose only violations are time-locked settles `inconclusive`, not `proved`. Implement FR-240's divergence test (positive-delay cycle), or keep searching past an unknown violation. | qsl-analyze/src/zone_check/search.rs:700-720,856-891 |
| FND-004 | medium | Nodes created by a closure exploration (`pending`) are judged right after the current node's children, ahead of shallower nodes still in the queue. An admitted violation found there is reported before an earlier one in canonical breadth-first order. That breaks FR-239's first-in-BFS rule for `UndefinedEvaluation`, which ranks like any other violation. The result stays deterministic, but the wrong one is chosen. Queue the pending nodes in BFS position, and judge them when they are dequeued. | qsl-analyze/src/zone_check/search.rs:514-527,856-880 |
| FND-005 | medium | `ZoneUndecided::DivergenceUnknown` and `DeadlockAfterDelay` have no counterpart in FR-239's `Undecided(InconclusiveCause)` or in FR-235's exhaustive map. `DeadlockAfterDelay` exposes a real spec gap: FR-236 and QSpec FR-417 give `final_delay` only to time-lock counterexamples, so a deadlock reached by waiting at a quiescent state cannot be stated as a counterexample. That needs a spec ticket (a `final_delay` on `Deadlock`). `DivergenceUnknown` exists only because FR-240's test is not implemented, and should go away with FND-003 rather than gain a spec row. | qsl-analyze/src/zone_check/search.rs:110-128 |
| FND-006 | low | `state_digest` encodes every clock place of the universe, including places of objects absent from the state. QSpec FR-181 defines the valuation on present clocks only, so two states equal on present clocks can get different digests. The `timed-state` byte form is also QSL-local: QSpec FR-181 fixes no encoding (spec gap, raise to QSpec). | qsl-eval/src/simulation/model/timed.rs:760-798 |
| FND-007 | low | The `UndefinedEvaluation` site puts the timed-state digest in `state_key`, and `UndefinedSite` has no time stamp and no clock valuation. TV-1 and FR-239-AC-5 say `where` names the position, its time stamp, the discrete state key, the exact clock valuation and the locus. | qsl-analyze/src/zone_check/search.rs:995-1006 |

## Verdict

**Not mergeable** until FND-001 (the tick vacuous proof) is fixed. FND-002 to FND-005 should be fixed in the same round. FND-003 and FND-005 can instead be deferred to 581b with a ticket, if 581b lands FR-240's divergence test and FR-237's admitted-endpoint replay before any settlement consumes EN-6. Correct: the aLU coverage criterion, `down`/`universe`, the sound `Locked` test, the iterative Tarjan, budgets that name their setting, cancellation, deterministic concretisation (least, midpoint or plus-one-unit, exact) and the FR-241-AC-1/AC-2 delays. Rust lane: no recursion except `uncovered`, whose depth is bounded by the number of enabling zones; `widen` saturates; no `unwrap` in production paths; the two `type_complexity` allows carry reasons.

## Coverage

Methods: code-review with the rust-review lane. Gap analysis is SR-1599002. Probe tests were run in the reviewer's own worktree and discarded.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | A finite path whose backward target is infeasible, and whose least-delay endpoint does not diverge, returns `Undecided(LassoNotConcretized)`, a cause FR-241 defines for lassos only. No other divergent point of the last zone or other path is tried. It is sound (settles inconclusive), but it names the wrong cause and gives up early. Use a finite-path cause, or search the last zone's divergent valuations before giving up. | qsl-analyze/src/zone_check/search.rs:995-1002 |
| FND-009 | low | Each `escape` call (one per violating node, per root and per point check) builds a fresh divergence graph. It is charged against `max_symbolic_states` per call only, never cumulatively, and never against `max_symbolic_transitions` or `max_zone_bytes`. A run's total work can exceed the caller's budgets. Charge the divergence graphs to the run's counters. | qsl-analyze/src/zone_check/divergence.rs:133-141 |

## Dispositions

Round 1, against #1599 fix commit 1c963df6f (head; 69990aada..fb3b8137f are `=` in `git range-diff cb95a247a..7ac32b8d7 3880da60a..fb3b8137f`).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | deferred | Half fixed, half deferred: `DivergenceUnknown` is gone with FND-003. `DeadlockAfterDelay` stays, citing STD-493 (Backlog), because FR-236 and QSpec FR-417 give `final_delay` to time-lock counterexamples only. It needs an FR-235 row or a spec'd counterexample form from STD-493. |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
