---
id: SR-1113
title: "QSL-373 scope and boundary review of ADR-026, FR-230 to FR-254 and their amendments"
type: SpecReview
analysis: scope-boundary
date: 2026-10-02
scope: "agent-ix/quire-spec-language@2b54b7e47b9e99fe95392e5749671333d9025f07; diff 404a5a88..2b54b7e4 (the QSL-373 commits on top of the stale #366 stack); spec/decisions/ADR-026-dense-time.md; ADR-018 amendments; spec/functional/FR-230..FR-254; spec/test-cases/TC-685..TC-710; spec/usecase/US-026; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-234
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-253
    type: reviews
---

## Summary

Ticket: QSL-373 (PR #573). This review covers ADR-026, FR-230 to FR-254, TC-685 to TC-710, US-026, the ADR-018 amendments and the index rows. It checks them against the owner rulings on QSL-373 and RES-54, the team-leader delay decision, the Wave B and Wave C rules, and the QSpec counterpart FR-415 to FR-421 (QSpec branch spec/wave-b-q4-prob-time).

**Consistent:**
- Dense time is specified (RU-1). Both clock bindings are kept, with `model-time` the default for a timed model (CB-5, FR-233).
- A non-local time-lock reuses ADR-022's trap evidence and V-10, and `TimeLock` stays its own kind (TD-3, FR-232, FR-237).
- EN-6 is a native zone engine with Wimmer-style certificates; a proof counts only after `check_zone_certificate` accepts it (CF-1 to CF-6, FR-244, FR-245). FR-245 places the checker in the qualified core.
- Hybrid dynamics go to QSpec FR-193 providers, and no solver result settles `refuted` (HY-1 to HY-3, FR-250).
- Schedulability is native closed-form (RTA, EDF QPA, AMC-rtb) with WCET as a premise in the obligation identity (RT-1 to RT-8, FR-246 to FR-249).
- Monitors run on integer ticks with sound rounding and a caller-set event rate (MN-1 to MN-4, FR-251).
- Delay distributions are specified; exact checking routes to EN-5 through digital clocks (SS-7, FR-254).
- Delay decision applied: an operation with no `delay` member has a free, nondeterministic delay, and a probabilistic bound is read on the min/max over its choices (SD-1, SD-5, FR-253); statistical checking settles a racing free delay `unsupported`, as QSpec FR-420 states.
- An undefined evaluation refutes with `UndefinedEvaluation{where, cause}` (TV-1, FR-235, FR-237, FR-239); see base FND-001/002 for the admitted-behaviour gap.
- No fixed caps: every limit has a published default and names itself when reached. No pins, ledgers, provenance records or compatibility paths. Ticket ids appear only in References.
- The worked examples were rechecked by hand: the §11 counterexample and time-lock, the §9 retry run, RTA `(1, 3, 12)` and the `C = 6` demands `9, 10, 12, 13`, the AMC set `Mc`, the stochastic `1/20`, the monitor ticks and buffer 21.
- Every AC has a TC row that tests behaviour. `quire validate` on the 56 changed spec files exits 0 (4 EARS warnings, ears FND-001/002). `tools/check-index-completeness.sh` passes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The pointwise meaning of every timed operator, and stutter invariance, are normative SHALLs here, but QSpec FR-416 owns them (FR-234's own References say so). Cite QSpec FR-416 and keep only QSL's evaluator obligation to implement it in exact arithmetic. | spec/functional/FR-234-check-timed-intervals-and-classify-timed-property-forms.md:95-102 |
| FND-002 | medium | Windows, the race (given draws, scheduler choice for free delays, tie order, redraw, Markov chain or MDP reading, min/max bound) and the `NotStochastic` conditions restate QSpec FR-420 normatively. Cite FR-420 and keep QSL's checker refusals, output types and exact window computation. | spec/functional/FR-253-check-delay-distributions-and-define-the-race.md:69-90 |

## Verdict

Lane and ownership boundaries are kept: QSpec owns grammar and wire, CG and the runtime own harnesses and monitors, and ticket ids stay in References. Two FRs restate QSpec semantics normatively (FR-234's pointwise meaning, FR-253's race) instead of citing QSpec FR-416 and FR-420.
