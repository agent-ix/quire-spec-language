---
id: SR-1112
title: "QSL-373 integrity review of ADR-026, FR-230 to FR-254 and their amendments"
type: SpecReview
analysis: integrity
date: 2026-10-02
scope: "agent-ix/quire-spec-language@2b54b7e47b9e99fe95392e5749671333d9025f07; diff 404a5a88..2b54b7e4 (the QSL-373 commits on top of the stale #366 stack); spec/decisions/ADR-026-dense-time.md; ADR-018 amendments; spec/functional/FR-230..FR-254; spec/test-cases/TC-685..TC-710; spec/usecase/US-026; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-233
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
| FND-001 | high | Every timed-model refusal disagrees with the QSpec counterpart. QSpec FR-415 refuses under `invalid_timed_model` with causes `duplicate-time-source`, `clock-without-dense-time`, `clock-write`, `clock-read-outside-constraint`, `clock-comparison`, `negative-clock-constant`, `time-invariant-shape` and `urgent-guard-reads-clock`; FR-230 and this AC use `conflicting_declaration`/`duplicate-declaration`, `unsupported_construct`/`expression-form`, `ill_typed`/`operator-ineligible` and `missing_declaration`/`missing-name` for the same cases. FR-230 also lacks the negative-constant refusal. Align FR-230 with QSpec FR-415's codes. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:149; 81-136 |
| FND-002 | medium | A `model-time` binding over an untimed model refuses `invalid_model_binding`/`wrong-model-selection` here, but `invalid_timed_model`/`model-time-on-untimed-model` in QSpec FR-416. Use QSpec's code. | spec/functional/FR-233-bind-a-model-claim-to-model-time-or-model-steps.md:93; 70-72 |
| FND-003 | low | `where` of an undefined timed evaluation is 'the position and its time stamp' in TV-1 (and FR-235/FR-239 examples), while QSpec FR-416 also names the discrete state key, the clock valuation and the locus of the undefined expression. Make TV-1 carry QSpec's fields. | spec/decisions/ADR-026-dense-time.md:177 |
| FND-004 | low | The owner ruled that the certificate checker lives in the qualified core. ADR-026 CF-3 and CF-6 call it a layer-6 facade entry with a small trusted base but never say it is in the qualified core; only FR-245 says so, citing draft ADR-029. Record the placement in CF-3. | spec/decisions/ADR-026-dense-time.md:214 |

## Verdict

ADR-026 and FR-230 to FR-254 agree with each other on structure, ids and index rows. They disagree with the QSpec counterpart on refusal codes: FR-230's timed-model refusals (high) and FR-233's untimed `model-time` refusal. TV-1's `where` is narrower than QSpec FR-416's, and the ADR omits the qualified-core placement of the checker. Not mergeable until FND-001 is fixed.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | FR-245 (lines 125-126), FR-248 (line 101) and FR-239 (lines 186-187) still cite 'ADR-029 CB-2 … (draft)'. Cite ADR-029 with its owning ticket, QSL-390, and drop '(draft)'. | spec/functional/FR-245-check-a-zone-certificate-in-the-qualified-core.md:125-126 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 41f6943a |
| FND-002 | fixed | 41f6943a |
| FND-003 | fixed | 41f6943a |
| FND-004 | fixed | 41f6943a |
