---
id: SR-13230
title: "Spec review of PR #1037: FR-250 solver-time and solver-steps units"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@25a16c9df; git diff origin/main...HEAD: FR-250 Behavior (solver-time, solver-steps), FR-250-AC-3, TC-705 step 3"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-705
    type: reviews
---

## Summary

Ticket: STD-207. Base checklist review of `git diff origin/main...HEAD` at 25a16c9df: two new FR-250 Behavior bullets (the `solver-time` budget as whole wall-clock seconds rounded up, the `solver-steps` budget as the provider's own step count, never converted to time), new FR-250-AC-3 and TC-705 step 3. ID formats hold (FR-250-AC-3 follows AC-2 with no gap; TC-705 scope widened to AC-3), AC-3 has a verification method and a TC step with expected results, and no hash, digest or pin is introduced. The ticket asks for the units confirmed with an AC and a rounding vector (1.5 s, direction stated); the PR does that (1.5 -> 2, 2 -> 2, 0.2 -> 1, round up).

Not done by this PR, for the planner: STD-207 also asks to state the units in the QSpec counterpart if it is silent. QSpec (agent-ix/quire-specification main 2c2918b) is silent: FR-193's verdict map says only "`limit-reached` naming the budget, its value and the request setting that raises it", and no QSpec file names `solver-time`, `solver-steps`, `max_solver_time` or `max_solver_steps`. That needs a QSpec ticket and PR (FR-193 and the FR-331 limit spellings); it is not a defect of this diff.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-250 Behavior: solver-time bullet | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md | examined |
| FR-250 Behavior: solver-steps bullet | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md | examined |
| FR-250-AC-3 | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md | examined |
| TC-705 Scope, step 3 and its expected result | spec/test-cases/TC-705-hybrid-solver-results-map-to-verdicts.md | examined |
| FR-250 verdict map `Stopped(limit)` row | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md | context_only |
| ADR-018 V-7 | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md | context_only |
| QSpec FR-193 verdict map | agent-ix/quire-specification: spec/functional/analysis/FR-193-analyze-continuous-hybrid-models.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Where the fractional duration comes from, and what the solver is given, is unstated. The bullet makes the request member `max_solver_time` itself a count of whole seconds, yet AC-3 requests "a limit of 1.5 s", so some input other than the member must carry the fraction (a CLI flag, an API duration, the provider's `Stopped` report?) and the spec names none. It also leaves open whether rounding happens when the request is written (the solver then runs 2 s and the record is exact) or only when the stop is reported (the solver runs 1.5 s and the record says 2). The rationale "a stop SHALL NOT report a budget smaller than the time the solver was allowed" reads as the second, which makes `value` differ from "the value the run used" that ADR-018 V-7 and `ReachedLimit.value` require. Name the input that takes a fractional duration and say the rounding is applied when writing `max_solver_time`, so the solver is given, and the record reports, the same whole-second budget. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:82-86,99; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:152 |
| FND-002 | medium | The PR introduces two new run limits, `max_solver_time` and `max_solver_steps`, with no published default. ADR-018 V-7: "Every limit has a published default, the request setting raises it, and every terminal record states the values used". A record for an unset limit has no value to state, and AC-3 only covers explicitly set limits. State both defaults (or that each is unbounded unless set, and what the record then states). | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:82-89; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:152 |

## Verdict

Findings: FND-001 (medium), FND-002 (medium). The units the ticket asked for are stated and the rounding direction is pinned by a vector a test can fail (2 s exact, 1.5 s and 0.2 s up); AC-3 traces to TC-705 step 3 with matching expected values. See the integrity review (SR-13232) for the conflict with FR-293 and the duplication with PR #1031.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | high | The fix names "The request's deadline duration (FR-293)" as the input that takes fractional seconds. No such request member exists: FR-293 names no deadline duration, and FR-276 makes the deadline a timer the frontend owns, which cancels the run with `Cancelled(Deadline)` ("The library reads no clock"); FR-292 keeps the wall-clock deadline out of the cache key. Tying the solver budget to that deadline also makes AC-3's time row unreachable: a 1.5 s deadline cancels at 1.5 s with `Cancelled{source: deadline}`, before the solver's rounded 2 s budget can end with `TimedOut`, and FR-293-AC-1 requires those two causes to stay distinct. Give `max_solver_time` its own input that accepts a fractional duration (for example the request member itself, in decimal seconds, rounded up at admission), independent of the FR-276 deadline. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:83-88,108 |

## Dispositions

Round 1, against head 594d3e3e1 (fix commit 594d3e3e1).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-250: "If it has a fractional part, then the request writer SHALL round it up to the next whole second when the request is admitted, give the solver that rounded budget, and state it as `max_solver_time` in the record." AC-3: "is admitted with `max_solver_time` 2 and the solver is given 2 s". The input it names is a new defect, FND-003. |
| FND-002 | fixed | FR-250: "`max_solver_time` has the published default 60 seconds"; "with the published default 1,000,000 steps"; AC-3 checks both. The values are the author's picks; the planner should confirm them. |

Round 2, against head 6a7c400df (fix commit 6a7c400df).

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | fixed | FR-250: "`max_solver_time` SHALL be a request setting of its own, independent of FR-276's deadline, that accepts a possibly fractional duration in seconds, with the published default 60 seconds." AC-3: "with no `Cancel` raised, settles `incomplete` with cause `TimedOut`, distinct from `Cancelled(Deadline)` raised on a run with an FR-276 deadline". |
