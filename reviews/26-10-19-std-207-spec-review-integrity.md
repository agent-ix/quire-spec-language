---
id: SR-13232
title: "Integrity review of PR #1037: FR-250 solver budgets against FR-293 and PR #1031"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@25a16c9df; git diff origin/main...HEAD: FR-250 Behavior (solver-time, solver-steps), FR-250-AC-3; compared with PR #1031 head f39f2a1f1"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-293
    type: reviews
---

## Summary

Ticket: STD-207. Consistency of the new FR-250 solver budget text with the rest of the QSL spec and with open PR #1031 (QSL-586, head f39f2a1f1), which adds its own solver budget bullet to the same FR-250 Behavior list. `git merge-tree` of the two heads merges with no textual conflict (the hunks are a few lines apart), so git will not flag the overlap.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-250 Behavior: solver-time bullet | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md | examined |
| FR-250 Behavior: solver-steps bullet | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md | examined |
| FR-250-AC-3 | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md | examined |
| FR-293 Behavior | spec/functional/FR-293-never-cache-failed-cancelled-timed-out-or-plugin-results.md | context_only |
| FR-293-AC-2 | spec/functional/FR-293-never-cache-failed-cancelled-timed-out-or-plugin-results.md | context_only |
| QSpec FR-306 never-stored rule | agent-ix/quire-specification: spec/functional/tooling/FR-306-compile-and-cache-exactly.md | context_only |
| PR #1031 FR-250 Behavior bullet (solver budgets) | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md@f39f2a1f1 | context_only |
| `IncompleteCause::TimedOut`, `LimitReached` | qsl-replay/src/proof_result.rs | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Declaring `solver-time` a wall-clock budget and settling its stop as `LimitReached{limit: solver-time, value: 2, ...}` contradicts FR-293. FR-293 requires an `incomplete` record's typed cause to "distinguish cancellation, a time limit and a deterministic budget", and says "A QSL `analyze` record shall hold no member that records wall time" (FR-293-AC-2: "hold no member that records a time"), because QSpec FR-306 never stores "a time-limited `incomplete`" and stores a deterministic-budget `incomplete` as a function of the key. `LimitReached` is the deterministic-budget cause (ADR-018 V-7; `IncompleteCause::TimedOut` is QSL's time-budget cause), and AC-3's `value: 2` is a member recording seconds of wall time. The hybrid solver is a native engine, not a plugin (FR-250: "a solver QSL runs is a native engine"), so the plugin never-stored rule does not cover it: a solver-time stop would be cached as if deterministic. Either settle a `solver-time` stop with the time-limit cause and keep the seconds out of the stored record, or state that `solver-time` is the documented exception and amend FR-293 and QSpec FR-306 to match. `solver-steps` is a deterministic budget and fits `LimitReached` as written. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:74,82-86,99; spec/functional/FR-293-never-cache-failed-cancelled-timed-out-or-plugin-results.md:29,38,43,60-61 |
| FND-002 | medium | PR #1031 (QSL-586, code lane) adds a bullet to the same Behavior list: "A `Stopped(limit)` result SHALL name its budget as `solver-time` (the request member `max_solver_time`, in whole seconds of solver wall time) or `solver-steps` (`max_solver_steps`, in solver steps), and the record's `value` SHALL be the budget's value for the run in that unit." Both PRs merge cleanly, leaving FR-250 with two statements of each unit that differ: #1031's has no rounding and says `value` is "the budget's value for the run", #1037's rounds a fractional duration up and words the time as wall-clock. #1031's code (`qsl-replay/src/hybrid.rs`) passes `ReachedLimit.value: u64` through, with no fractional input and no rounding, so AC-3 has no implementation yet. Reconcile in whichever merges second, and prefer #1037 (STD-207, the spec owner) first: #1031 then drops its solver budget bullet on rebase (its `ExactRational` horizon bullet is separate) and the AC-3 test and rounding go to the code lane as follow-up work. If #1031 merges first, #1037 must delete #1031's bullet when it rebases. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:82-89 |

## Verdict

Findings: FND-001 (high), FND-002 (medium). The `solver-steps` statement agrees with FR-293 and the deterministic-budget cause; AC-3's vectors agree with the Behavior bullets; TC-705 step 3 agrees with AC-3.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The fix routes a solver-time stop to `TimedOut` in a new bullet, but FR-250's verdict map still sends every `Stopped(limit)` to `Incomplete(LimitReached{limit, value, setting})`, and AC-2 still says "`Stopped` settles `incomplete`, `LimitReached`". FR-250 now gives two results for a solver-time stop. Split the map row (and AC-2) into a solver-steps stop, `LimitReached`, and a solver-time stop, `TimedOut`, or narrow both to the steps budget. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:74,89-92,107 |

## Dispositions

Round 1, against head 594d3e3e1 (fix commit 594d3e3e1).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The cause is fixed ("SHALL settle `incomplete` with the time-limit cause `IncompleteCause::TimedOut` (FR-293), not `LimitReached`"), but the record half is not: the bullet before it says the request writer SHALL "state it as `max_solver_time` in the record", a member holding seconds, while this bullet says "The stored record SHALL hold no member that records wall time" (FR-293-AC-2: "no member that records a time"). Pick one: keep the seconds out of the record, or state why a budget in seconds is not a member that records a time and amend FR-293 to match. |
| FND-002 | deferred | Nothing for #1037 to change. PR #1031 (head f39f2a1f1, unchanged) still has its "`solver-time` ... the record's `value` SHALL be the budget's value" bullet, which now also contradicts #1037's `TimedOut` rule; `git merge-tree` of the two heads is still clean. Whichever merges second reconciles: with #1037 first, #1031 drops that bullet on rebase. Left for the lead and planner on STD-207 and QSL-586. |
