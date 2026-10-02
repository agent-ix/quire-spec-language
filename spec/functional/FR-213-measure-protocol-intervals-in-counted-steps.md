---
id: FR-213
title: "Measure interval operators over a protocol subject in counted steps"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-091
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-092
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-431
    type: depends_on
---
# FR-213: Measure interval operators over a protocol subject in counted steps

## Description

Over a protocol subject, the TemporalTrace evaluator SHALL read every step
as a position, observed with its operation's anchor for an `attempt` or
`cattempt` and a `protocol` anchor otherwise (ADR-027 PB-2). The evaluator
and the model checker SHALL measure interval distance, and a bounded
profile's horizon, in **counted** steps: operations (`attempt`,
`cattempt`), events (`event`, `activate`), `send` and `receive`, and the
terminal stutter step (ADR-027 PB-3). Both SHALL treat every other step,
`fence` and memory steps included, as uncounted (QSpec FR-431).

## Use case

A verification operator writes "within two steps of the failure, the
balance is restored". The steps they mean are the protocol's operations and
events, not the bookkeeping of forking, joining and finishing. The bound
reads the same whether or not the protocol forks, so adding a `parallel`
around two attempts does not change what "two steps" means.

## Inputs

- A behaviour of a protocol subject (FR-206 to FR-209), with each step's
  kind.
- A checked temporal clause with interval operators (FR-123).

## Outputs

- For each position, its anchor and its counted distance from position 0.
- The TemporalTrace evaluator's reading of interval operators over that
  distance, and FR-126's interval expansion over counted steps.

## Behavior

- The TemporalTrace evaluator SHALL read positions and anchors over a
  protocol subject as QSpec FR-431 states, with a memory step carrying the
  anchor its model gives.
- A `holds` atom SHALL evaluate over the model observation at its position,
  each location read through the memory model's atom values (FR-215).
- The evaluator SHALL measure interval distance in QSpec FR-431's counted
  steps, and unbounded operators SHALL range over every position.
- FR-126's expansion of an interval operator into next-position steps SHALL
  advance its offset only on counted steps.
- Over a model subject the evaluator SHALL count every step, which is
  ADR-018's reading.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-213-AC-1 | Over ADR-027 §7's repaired `Fill` (`TwoPre: self.v <= 1`), on the behaviour that takes `B` before `A`, positions 1 (`fork`), 4 (`join`) and 5 (`finish`) carry `protocol` anchors and positions 2 and 3 the anchors of `setTwo` and `inc`; the counted steps are `attempt(B)` and `attempt(A)`, and `eventually[0,2] holds(k.v = 3)` holds at position 0. `eventually[0,1] holds(k.v = 3)` does not hold at position 0. | Test (TC-658) |
| FR-213-AC-2 | Over ADR-027 §7.1, on the failure path, `eventually[0,2] holds(k.bal = 0)` holds at position 2 and fails at position 1; on the success path t0, t1, t2, t3, t5 it fails at every position from 1. | Test (TC-658) |
| FR-213-AC-3 | The model checker reproduces AC-1 and AC-2: over the repaired `Fill`, `eventually[0,2] holds(k.v >= 2)` under event-position false-extension settles `proved`, and `eventually[0,1] holds(k.v = 3)` settles `refuted` with a prefix on which no position at counted distance 0 or 1 from position 0 has `k.v = 3`. Over a model subject the same claim forms give FR-126's verdicts. | Test (TC-658) |

## Dependencies

- ADR-027 §3.1 PB-2 and PB-3, §2.5 SE-3; ADR-018 §2 SM-3, §11 IV-2 and
  IV-3.
- [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  (positions and the evaluator),
  [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (interval expansion), FR-206.
- QSpec owns counted and uncounted steps, interval distance over a protocol
  subject and the `protocol` anchor kind (ADR-027 QS-6, QS-12): QSpec
  FR-431, over FR-091, FR-092 and FR-161.

## References

- Owning ticket: Linear QSL-396. QSpec half: QSpec FR-431 (Linear STD-140).
