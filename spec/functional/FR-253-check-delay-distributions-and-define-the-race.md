---
id: FR-253
title: "Check delay distributions and define the race of a stochastic timed model"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-231
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-142
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-205
    type: depends_on
---
# FR-253: Check delay distributions and define the race of a stochastic timed model

## Description

QSL's S3 checker SHALL admit delay distributions on the operations of a
`time dense` model, and QSL's layer-5 race SHALL implement QSpec FR-420's
race in exact arithmetic, which turns the model, with a workload, into a
probability measure over timed behaviours (ADR-026 SD-1 to SD-5). Windows
are computed exactly, a free delay (an operation with no `delay` member) is
the scheduler's choice, and states with no defined race settle
`unsupported`, `NotStochastic`.

## Use case

A verification operator gives a server's reply a discrete delay
distribution and a user's actions exponential delays, and asks how likely a
late reply is. They need one definition of how delays race, including
ties at a deadline, so that every sampled run is a behaviour of the model.

## Inputs

- Parsed `delay ~ D` members, in the spelling QSpec's shared grammar fixes,
  with `D` one of `uniform`, `uniform[a, b]`, `exponential(λ)`, `discrete {
  d1: w1, …, dn: wn }`.
- A workload (the statistical design's PM-2) and the timed subject.

## Outputs

- `Option<DelayDistribution>` on each checked operation, `None` for a free
  delay: `Uniform`, `UniformRange{a, b}`, `Exponential{rate}`,
  `Discrete(Vec<(ExactRational, ExactRational)>)`, all exact, in the
  model's unit.
- For a timed state: each scheduled identity's window as a finite union of
  intervals with exact rational ends and openness, and the race's
  conditioned distributions.
- The disposition `Unsupported(NotStochastic{state, identity})`.

## Behavior

- The checker SHALL admit `delay ~ D` only on an operation of a `time
  dense` model, refusing it elsewhere with `invalid_probabilistic_model`/
  `delay-on-untimed-model` at its span (QSpec FR-405).
- If a distribution parameter is not an exact rational, `λ` is not
  positive, `uniform[a, b]` does not have `0 <= a <= b`, or a `discrete`
  distribution repeats a delay, has a negative delay or has a weight that
  is not positive, then the checker SHALL refuse `invalid_model_binding`/
  `malformed-declaration` at its span.
- The checker SHALL record an operation with no `delay` member as a free
  delay (`None`), with no distribution.
- **Windows.** At `(s, v)`, the layer-5 race SHALL compute each scheduled
  identity's window as QSpec FR-420 defines it, a finite union of intervals
  with exact rational ends and openness, from the admissible delays
  (FR-231) and the identity's data precondition and guard.
- **Race.** The layer-5 race SHALL implement QSpec FR-420's race in exact
  arithmetic: the given draws conditioned on their windows, the
  scheduler's choice of each free delay, the tie order by the workload and
  the redraw at each new state. It SHALL report whether the subject under
  the workload is a Markov chain or, when a free delay races, a Markov
  decision process, so that a bound is read on the minimum or maximum
  over the free delays' choices (ADR-028 TA-1).
- **Conditions.** When QSpec FR-420 gives a timed state no defined race,
  the race SHALL dispose the item `Unsupported(NotStochastic{state,
  identity})`, naming the state and identity.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-253-AC-1 | ADR-026 §11's stochastic variant checks: `reply` `Discrete[(1, 90), (3, 10)]`, `timeout` `Uniform`, `send` and `reset` `Exponential{1/1000}` per ms. In the state just after `send`, `reply`'s window is `[1, 3]` and `timeout`'s (with `T = 3 ms`) is `[3, 3]`. | Test (TC-708) |
| FR-253-AC-2 | In that state the race gives `reply` at 1 with probability `9/10`; at 3 it ties with `timeout`, ordered evenly by the workload `Even`, so `reply` then `timeout` and `timeout` then `reply` each have probability `1/20`, and in the second order `reply` is still enabled at its turn. | Test (TC-708) |
| FR-253-AC-3 | Refusals at the span: `delay ~ exponential(0)`; `discrete { 1 ms: 1, 1 ms: 2 }`; `uniform[3 ms, 1 ms]`; `delay ~ uniform` in an untimed model (`invalid_probabilistic_model`/`delay-on-untimed-model`). A state where an identity with unbounded window and `uniform` races is disposed `Unsupported(NotStochastic)` naming the state and identity. | Test (TC-708) |
| FR-253-AC-4 | The §11 stochastic variant with no `delay` member on `timeout` checks `timeout` with `None`. In the state just after `send` the race is a Markov decision process: after `reply`'s draw the scheduler chooses `timeout`'s delay in `[3, 3]`, and no distribution is assigned to it. | Test (TC-708) |

## Dependencies

- ADR-026 §10 SD-1 to SD-5; ADR-028 TA-1 and RU-6 (the minimum and maximum
  over free delays).
- [FR-230](FR-230-check-time-declarations-clocks-and-clock-constraints.md),
  [FR-231](FR-231-read-a-timed-subject-s-behaviours-as-timed-traces.md).
- QSpec FR-142 and FR-205 (exact time quantities).
- The statistical design (ADR-024 PM-1 to PM-4, SV-5), which the race
  extends.

## References

- QSpec half: QSpec FR-405 (Linear STD-137) owns the `delay ~ D` grammar
  and its four families; QSpec FR-420 (Linear STD-139) owns given and free
  delays, the race beside the workload, and `NotStochastic` on the wire
  (ADR-026 OV-7, OV-8).
- A. David et al., 2011 (ADR-026 References).
