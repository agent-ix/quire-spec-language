---
id: FR-231
title: "Read a timed subject's behaviours as timed traces"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-090
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-161
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
---
# FR-231: Read a timed subject's behaviours as timed traces

## Description

QSL SHALL extend FR-125's model subject to a **timed subject**, a model
subject whose model has a `time` member (FR-230), and read each of its
time-divergent behaviours as a timed trace for the layer-5 TemporalTrace
evaluator (ADR-026 TS-1 to TS-7). The evaluator's value on one timed trace
is the semantics; the zone engine (FR-239), replay (FR-237) and the
statistical sampler (FR-254) all answer to it.

## Use case

A verification operator asks whether every reply comes within 3 ms. They
need one definition of a timed behaviour: when time may pass, what a
position's time stamp is, which behaviours count, and how a model that sits
idle forever is read, so that a counterexample the engine reports evaluates
false when they replay it.

## Inputs

- A `ModelSubject` (FR-125) whose checked package has a `TimedModel`
  (FR-230).
- A timed trace for evaluation: a finite prefix or a lasso of positions,
  each with its discrete state, clock valuation, time stamp and anchor.

## Outputs

- `TimedState { discrete: ModelState, clocks: ClockValuation }`, with
  `ClockValuation` an exact non-negative rational per clock of the
  universe, in the model's unit.
- `ModelSystem` over a timed subject exposes the delay and step relations
  below, with exact rational arithmetic.
- For each position: its FR-125 observation, its anchor, its time stamp
  `τ_i` and its clock valuation.

## Behavior

### Timed states and moves

- The clocks of a timed subject SHALL be one per object of the universe per
  clock field (FR-230), each 0 in every initial state. FR-106 snapshots
  carry no clock value.
- **Delay.** A delay by an exact rational `d >= 0` SHALL take `(s, v)` to
  `(s, v + d)`. It SHALL be admissible when `d = 0`, or when `s` is not
  urgent and every time invariant whose predicate holds at `s` holds at
  `v + d`. A state `s` is urgent when an `urgent when P` predicate holds at
  `s`, or an operation named by `urgent O` is enabled at `s`.
- **Discrete step.** A step by transition identity `t` SHALL take `(s, v)`
  to `(s', v')` when `t`'s data precondition holds at `s`, a convex part of
  its guard holds at `v`, `s'` is an FR-120 post-state, `v'` is `v` with
  `t`'s resets applied, and every time invariant whose predicate holds at
  `s'` holds at `v'`.

### Behaviours, positions and time stamps

- A timed behaviour SHALL alternate one delay, possibly 0, and one discrete
  step, from an initial state at time 0.
- Positions SHALL be FR-125's: position 0 is the initial state, position
  `i > 0` is the post-state of the `i`-th discrete step with that step's
  anchor. Each position SHALL carry its time stamp `τ_i`, the sum of the
  delays before steps 1 to `i`, and its clock valuation. Delays SHALL not be
  positions, and steps with equal time stamps SHALL keep their step order.
- **Time divergence.** A behaviour SHALL be admitted only when its time
  stamps grow beyond every bound.
- **Quiescent states and the idle tail.** A discrete state SHALL be
  quiescent when no time invariant applies at it and it is not urgent. A
  behaviour that ends its discrete steps at a quiescent state SHALL be
  extended under infinite-trace and the timed profile by FR-125's terminal
  stutter step, repeated forever, each after a positive delay. Under a
  bounded profile, an idle tail SHALL close the execution at its last
  discrete step, as FR-125 closes a terminal state.
- **Digital time.** Under a `Tick` source, every delay SHALL be 0, the time
  stamp of position `i` SHALL be the period times the number of `O` steps
  among steps 1 to `i`, the terminal stutter step SHALL count as an `O`
  step, and divergence SHALL mean infinitely many `O` steps.
- **Untimed reading.** An ADR-018 claim under event-position
  false-extension or infinite-trace over a timed subject SHALL read these
  positions with the clock valuation as part of each position's state, over
  the admitted behaviours.

### Evaluation

- A `holds` atom SHALL read the discrete state through FR-107; an atomic
  clock constraint in a claim SHALL read the position's clock valuation.
- Over a timed lasso, the evaluator SHALL read a position past the
  represented trace as the loop position it wraps to, with time stamps
  growing by the loop's total delay on each repetition.
- Every arithmetic operation on clocks and time stamps SHALL be exact.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-231-AC-1 | Over `Rpc` with `T = 4 ms` (universe `{c}`): from `(Idle, x = 0)` a delay of 7 is admissible (quiescent); after `send`, a delay of 3 is admissible and a delay of `3 + 1/1000` is not (`inflight` bounds `x <= 3`); `reply` is enabled after delay `1` and not after delay `999/1000`. The trace `send` at 0, `reply` after delay `5/2` has time stamps `0, 0, 5/2` and `x = 5/2` at position 2. | Test (TC-686) |
| FR-231-AC-2 | Over the `Rpc` unit with `urgent when self.phase = Waiting` added, no positive delay is admissible after `send`. A behaviour of `Rpc` that ends at `Replied` reads under infinite-trace as the stutter step repeated after positive delays, and `always holds(c.phase != Waiting)` evaluated from position 2 over it is `true`; under event-position false-extension the execution closes at position 2. | Test (TC-686) |
| FR-231-AC-3 | A digital model `time tick tick period 5 ms` whose operations are `tick` and `work` gives the trace `work, tick, work, tick` the time stamps `0, 0, 5, 5, 10`. A behaviour of it that ends at a terminal state diverges through the stutter step counted as `tick`. | Test (TC-686) |
| FR-231-AC-4 | An ADR-018 claim `always holds(not c.late)` under infinite-trace over the timed `Rpc` subject evaluates on a timed trace exactly as on the same positions with time stamps removed, and a claim atom `c.x <= 3 ms` reads the position's clock valuation. | Test (TC-686) |

## Dependencies

- ADR-026 §3 TS-1 to TS-7; ADR-018 SM-1 to SM-5 as amended by ADR-026.
- [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md),
  [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md),
  [FR-230](FR-230-check-time-declarations-clocks-and-clock-constraints.md).
- QSpec FR-090, FR-161 and FR-181 own the timed subject, delay moves, the
  time-divergence premise, quiescent states and the idle tail as normative
  semantics (ADR-026 OV-5).

## References

- QSpec half: Linear STD-139 (the timed subject in QSpec FR-161 and FR-181);
  this requirement cites it until those QSpec FRs merge.
