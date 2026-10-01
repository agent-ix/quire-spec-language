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

QSL SHALL admit delay distributions on the operations of a `time dense`
model and define the race that turns the model, with a workload, into a
probability measure over timed behaviours (ADR-026 SD-1 to SD-5). Windows
are computed exactly; every identity whose window is non-empty draws a
delay conditioned on its window; the smallest draw wins and ties are
ordered by the workload. States where the race is undefined settle
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

- `DelayDistribution` on each checked operation: `Uniform`,
  `UniformRange{a, b}`, `Exponential{rate}`, `Discrete(Vec<(ExactRational,
  ExactRational)>)`, all exact, in the model's unit.
- For a timed state: each scheduled identity's window as a finite union of
  intervals with exact rational ends and openness, and the race's
  conditioned distributions.
- The disposition `Unsupported(NotStochastic{state, identity})`.

## Behavior

- The checker SHALL admit `delay ~ D` only on an operation of a `time
  dense` model, refusing it elsewhere with `unsupported_construct`/
  `expression-form`.
- If a distribution parameter is not an exact rational, `λ` is not
  positive, `uniform[a, b]` does not have `0 <= a <= b`, or a `discrete`
  distribution repeats a delay, has a negative delay or has a weight that
  is not positive, then the checker SHALL refuse `invalid_model_binding`/
  `malformed-declaration` at its span.
- An operation with no `delay` member SHALL have `Uniform`.
- **Windows.** At `(s, v)`, the window of a scheduled identity SHALL be the
  set of `d` such that the delay to `v + d` is admissible (FR-231) and the
  identity's data precondition and guard hold at `v + d`, computed exactly.
- **Race.** Every scheduled identity whose window is non-empty and whose
  distribution has positive mass in it SHALL draw a delay from its
  distribution conditioned on the window. Uniform over a set SHALL be
  uniform in length, and over a set of zero length uniform over its points.
  The smallest draw SHALL win: the model delays by it and takes the winner.
  Identities that tie SHALL be ordered by the workload, without
  replacement by weight then uniformly within an operation, and taken in
  that order at the same instant, each only while still enabled at its
  turn. Every identity SHALL redraw at the new state.
- **Conditions.** A timed state where an identity with an unbounded window
  has `Uniform`, or where no identity races and delay is bounded, SHALL be
  disposed `Unsupported(NotStochastic{state, identity})`. A quiescent state
  where no identity races SHALL take FR-231's idle tail.
- ADR-018 claims and timed claims SHALL read a stochastic model as a timed
  one and ignore distributions.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-253-AC-1 | ADR-026 §11's stochastic variant checks: `reply` `Discrete[(1, 90), (3, 10)]`, `send` and `reset` `Exponential{1/1000}` per ms. In the state just after `send`, `reply`'s window is `[1, 3]` and `timeout`'s (with `T = 3 ms`) is `[3, 3]`. | Test (TC-708) |
| FR-253-AC-2 | In that state the race gives `reply` at 1 with probability `9/10`; at 3 it ties with `timeout`, ordered evenly by the workload `Even`, so `reply` then `timeout` and `timeout` then `reply` each have probability `1/20`, and in the second order `reply` is still enabled at its turn. | Test (TC-708) |
| FR-253-AC-3 | Refusals at the span: `delay ~ exponential(0)`; `discrete { 1 ms: 1, 1 ms: 2 }`; `uniform[3 ms, 1 ms]`; `delay ~ uniform` in an untimed model. A state where an identity with unbounded window and `uniform` races is disposed `Unsupported(NotStochastic)` naming the state and identity. | Test (TC-708) |

## Dependencies

- ADR-026 §10 SD-1 to SD-5.
- [FR-230](FR-230-check-time-declarations-clocks-and-clock-constraints.md),
  [FR-231](FR-231-read-a-timed-subject-s-behaviours-as-timed-traces.md).
- QSpec FR-142 and FR-205 (exact time quantities).
- The statistical design (ADR-024 PM-1 to PM-4, SV-5), which the race
  extends.

## References

- QSpec half: Linear STD-139 owns the `delay ~ D` grammar and its four
  families, the race beside the workload, and `NotStochastic` on the wire
  (ADR-026 OV-7, OV-8); this requirement cites it until those QSpec FRs
  merge.
- A. David et al., 2011 (ADR-026 References).
