---
id: FR-254
title: "Sample timed runs for statistical checking"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-231
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-234
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-237
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-253
    type: depends_on
---
# FR-254: Sample timed runs for statistical checking

## Description

The statistical engine EN-4 SHALL sample a stochastic timed model by
repeating the race (FR-253) from the initial state, with every delay an
exact rational drawn on a grid of `2^-q` in the uniform value, and read
timed events and measures over the samples (ADR-026 SS-1 to SS-7). A
`measured` verdict is a measurement of the quantised model and records `q`.
Each sample replays by FR-237 plus draw recomputation. Exact probabilistic
checking over a timed model routes to EN-5.

## Use case

A verification operator asks whether a late reply has probability at most
1%, the 99th percentile of reply latency, and the fraction of time a
server is available. Each estimate comes with its confidence and the grid
`q`, and any sample can be replayed exactly.

## Inputs

- A probabilistic claim over a stochastic timed subject, its workload, a
  seed, the method settings of the statistical design (its ST-8), and
  `TimedSampling { q: u32, max_sample_steps: u64 }`: `q`, the grid
  exponent, default 64, a caller-set method parameter; `max_sample_steps`,
  the steps one sample may take before passing its horizon, default
  16_777_216 (2^24), an ADR-014 B-5 budget.

## Outputs

- The statistical design's `measured` verdict, with `q` in its basis, or
  its `undecided` and `unsupported` dispositions; sampled witnesses
  carrying their delays.

## Behavior

- **Events and measures.** Under the timed profile, an event SHALL be a
  `BoundedWindow` formula on origin, decided by the positions with time
  stamp up to its horizon. A measure `within h` under `model-time` SHALL
  take `h` as a time quantity. `elapsed from holds(A) until holds(B) within
  h` SHALL be the time-stamp difference of the two positions. `weighted by
  delay` SHALL weight each position by the delay of the step that leaves
  it.
- **Given delays only.** If an identity with a free delay (FR-253) races
  at a sampled timed state, then EN-4 SHALL settle the item `unsupported`,
  `NotStochastic{state, identity}`, at the first such state.
- **Draws.** A `discrete` draw SHALL select by exact integer weights. A
  `uniform`, `uniform[a, b]` or `exponential` draw SHALL take a uniform value `u` on a grid
  of `2^-q` from the generator and return the conditioned inverse
  distribution function at `u`, computed by the statistical design's
  rational method and rounded up, an exact rational. Each delay draw and
  each tie-break SHALL be one more choice with its own choice index.
- A sample SHALL end when its next position's time stamp passes the
  horizon, or at an idle tail.
- If a sample reaches a local time-lock before its horizon, where no
  positive delay is admissible and no transition identity is enabled, then
  EN-4 SHALL settle the item `unsupported`, `TimeLockedSample{state}`,
  naming the state, and SHALL keep no witness from it: the measure is over
  time-divergent behaviours only (ADR-026 TD-1), and the time-lock-freedom
  item reports the time-lock.
- **Budgets.** If a sample takes more than `max_sample_steps` steps without
  passing its horizon, then EN-4 SHALL stop the run as the statistical
  design's budgets state, naming `max_sample_steps`, its value and the
  request member that raises it; every budget and `q` SHALL be a caller-set value with a
  published default.
- **Regeneration.** The regenerative method SHALL cut a run at each return
  to the initial discrete state with every clock 0; a model that never
  returns SHALL stop when a cycle reaches `max_cycle_steps` and settle
  `incomplete`, `limit-reached` naming `statistical.max_cycle_steps`, as
  ADR-024 ST-7 does.
- **Witnesses.** A sampled witness SHALL carry each step's delay and replay
  by FR-237, plus recomputation of each draw from the seed, trace index,
  step, choice index and `q`.
- **Exact route.** The request writer SHALL route a request for `exact`
  evidence over a timed subject to EN-5, whose digital route (ADR-028 TA-1,
  TA-2) decides which constraints and delay distributions it admits.
- EN-4 SHALL compute the verdict as a function of the claim, the subject,
  the workload, the seed and the settings.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-254-AC-1 | `RareLate` (`probability <= 0.01 [eventually[0 ms, 3 ms] holds(c.late)]`) over FR-253-AC-1's model, whose exact event probability is `1/20`, settles `measured`, `Rejected`, with `q` 64 in its basis; with `reply` under `uniform` it settles `measured`, `Accepted`. Two runs with one seed give equal verdicts. | Test (TC-709) |
| FR-254-AC-2 | Every sampled delay is an exact rational; a sampled witness replays by FR-237 and its draws recompute from the seed; the same witness with one delay changed fails draw recomputation. With `q` 8, the basis records 8 and each sampled `uniform` or `exponential` delay equals the rounded-up conditioned inverse distribution function at a multiple of `2^-8`. | Test (TC-709) |
| FR-254-AC-3 | `elapsed from holds(c.phase = Waiting) until holds(c.phase != Waiting) within 3 ms` reads each sample's time-stamp difference; a model that never returns to its initial discrete state with every clock 0 settles `incomplete`, `limit-reached` naming `statistical.max_cycle_steps` under the regenerative method; an `exact` request over the strict-guard `Rpc` variant settles `unsupported`. `RareLate` with no `delay` member on `timeout` settles `unsupported`, `NotStochastic`, naming the state just after `send` and `timeout`. `RareLate` with `max_sample_steps` 1 stops `Incomplete(LimitReached{limit, value, setting})` naming `max_sample_steps`, 1 and `TimedSampling.max_sample_steps`. | Test (TC-709) |
| FR-254-AC-4 | A stochastic `Lock` variant (one object `c` with `phase: {A, B}` and a clock `x` never reset; `go` moves `A` to `B` with `delay ~ discrete { 1/2 ms: 1 }`; `time invariant when phase = B { x <= 1 ms }`; no operation enabled in `B`) samples `go` at `1/2`, delays to `(B, x = 1)`, where no positive delay is admissible and no identity is enabled, and settles `unsupported`, `TimeLockedSample` naming that state, with no witness kept. | Test (TC-709) |

## Dependencies

- ADR-026 §10 SS-1 to SS-7.
- [FR-231](FR-231-read-a-timed-subject-s-behaviours-as-timed-traces.md),
  [FR-234](FR-234-check-timed-intervals-and-classify-timed-property-forms.md),
  [FR-237](FR-237-replay-a-timed-counterexample.md),
  [FR-253](FR-253-check-delay-distributions-and-define-the-race.md).
- The statistical design (ADR-024 PF-1 to PF-6, ST-1 to ST-9, SV-1 to
  SV-8) and the exact probabilistic engine (ADR-028), which this extends and
  routes to.

## References

- QSpec half: QSpec FR-420 (Linear STD-139) owns timed events and
  measures, free delays and the sampler's delay draws on the `2^-q` grid
  (ADR-026 OV-8); QSpec FR-408 (Linear STD-137) owns the
  statistical methods.
