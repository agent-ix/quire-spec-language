---
id: FR-204
title: "Check probabilistic timed automata through digital clocks"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-023
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-197
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-198
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-202
    type: depends_on
---
# FR-204: Check probabilistic timed automata through digital clocks

## Description

EN-5 SHALL decide a probabilistic claim with exact evidence over a timed
subject whose clock constraints are all closed by exploring its digital
MDP (ADR-028 TA-1 to TA-7): integer clocks after scaling every constant,
unit delay moves, and discrete moves as the timed step with integer clocks.
The claim's scheduler SHALL resolve each delay (RU-6): over every scheduler
the scheduler chooses delays and identities; under a workload, the workload
resolves the identity and every delay a `discrete` delay distribution
gives, so the subject is a DTMC over digital timed states; a delay no
distribution gives stays the scheduler's choice and is decided by its
minimum and maximum. Strict constraints, real-valued delay distributions,
zero-delay cycles and other form shapes SHALL settle `unsupported` with a
cause that names them.

## Use case

A protocol designer checks that a retransmitting sender delivers within
4 ms with probability at least 0.99 against any choice of send times inside
the timing constraints. EN-5 explores the digital MDP, finds the minimum
exactly `99/100`, and proves the claim at equality. With the send delay
declared as `discrete {1 ms: 1, 2 ms: 1}` under a workload, it computes the
exact probability of the resulting chain instead.

## Inputs

- FR-196's request over a timed subject (ADR-026 TS-1), with its clocks,
  guards, time invariants, urgency, and any `delay` declarations.

## Outputs

FR-196's `ProbProduct` over digital timed states, whose `ProductKey` adds an
integer value per clock and, for a deadline event, the deadline clock; delay
edges carry `action: None` and one unit of elapsed time. The results of
FR-197 and FR-198 over it, and FR-203's unsupported causes
`StrictClockConstraint{locus}`, `DelayDistribution{operation}`,
`ZeroDelayCycle{states}`, `TimedFormShape` and ADR-026's `NotStochastic`.

## Behavior

### Admission

- Every atomic clock constraint in a guard, every time invariant and every
  timed interval of the claim SHALL be non-strict, or EN-5 SHALL settle
  `Unsupported(StrictClockConstraint{locus})` naming the first strict one in
  source order.
- The admitted forms SHALL be `Reach` over state predicates; deadline events
  `eventually[0, D] holds(P)` and `holds(A) until[0, D] holds(B)` with `D`
  closed; and `expected elapsed until holds(B)`. Another form shape SHALL
  settle `Unsupported(TimedFormShape)`.
- Over every scheduler, a model that declares any `delay` distribution SHALL
  settle `Unsupported(DelayDistribution{operation})` naming the first such
  operation.
- Under a workload, a `uniform`, `uniform[a, b]` or `exponential` delay
  distribution SHALL settle `Unsupported(DelayDistribution{operation})`.

### The digital MDP

- EN-5 SHALL scale every constant to an integer by the least common
  denominator of the model's and the claim's constants, and give each clock
  integer values capped at its largest compared constant plus one.
- A delay move SHALL add one unit to every clock below its cap; it SHALL be
  admissible when the state is not urgent and every applicable time
  invariant holds after it. A discrete move SHALL be ADR-026 TS-2's step with
  integer clocks.
- Over every scheduler, each digital timed state's actions SHALL be its
  admissible delay move and its enabled scheduled identities.
- Under a workload, each racing identity SHALL draw its `discrete` delay
  conditioned on its window, the smallest draw winning with ties ordered by
  the workload's weights (ADR-026 SD-3); each race outcome SHALL be one
  edge, delaying by the winning amount and taking the winning identity, with
  its exact probability. A delay no distribution gives SHALL be an action of
  the scheduler. A state where no identity races and delay is bounded SHALL
  settle `Unsupported(NotStochastic)`.
- A deadline event SHALL add one digital clock that no step resets, capped
  at `D + 1`. `expected elapsed` SHALL carry one unit of time as the reward
  of each delay move.
- EN-5 SHALL decompose the subgraph of discrete moves into SCCs and settle
  `Unsupported(ZeroDelayCycle{states})` when a reachable cycle has no delay
  move.

### Evidence

- A witness scheduler SHALL choose between a unit delay and an identity at
  each digital state it names, and under a workload only the delays no
  distribution gives.
- A timed witness path SHALL carry ADR-026 CT-1 delays and replay by CT-3
  through FR-202; replay SHALL check that its last timed state has no
  admissible discrete step before the event's time horizon passes.
- A certificate SHALL name digital product states, and FR-201's checker
  SHALL re-enumerate the digital MDP.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-204-AC-1 | §15.5 `Deadline` over every scheduler: `x` takes the values 0 to 2 and the deadline clock 0 to 5; the minimum is `99/100` (send at `x = 2`) and the maximum `9999/10000` (send at `x = 1`); the item settles `proved`, `ExactValue{99/100}`. At threshold 0.995 it settles `refuted` with a path of two losses at 2 ms and 4 ms of probability `1/100`, whose replay checks that the next `send` needs `x >= 1`, after 4 ms. | Test (TC-639) |
| FR-204-AC-2 | §15.5 `MeanTime` settles `proved`, `ExactValue{20/9 ms}` (maximum; the minimum is `10/9 ms`). With the guard `self.x > 1 ms` the items settle `unsupported`, `StrictClockConstraint`, naming the guard. | Test (TC-639) |
| FR-204-AC-3 | `Retx` with `delay ~ discrete { 1 ms: 1, 2 ms: 1 }` on `send`, under a workload `One` with weight 1 on `send`, is a DTMC over digital timed states; `probability >= 0.99 [eventually[0 ms, 4 ms] holds(m.delivered)]` settles `proved`, `ExactValue{159129/160000}`: after every reset `send` waits 1 or 2 ms with probability 1/2 each, both inside its window `[1, 2]`, loses with `1/10`, and attempts at time stamps up to 4 ms inclusive count. The same claim `under every scheduler` settles `unsupported`, `DelayDistribution`, naming `send`; with `delay ~ exponential(1 per ms)` under `One` it settles `unsupported`, `DelayDistribution`. | Test (TC-639) |
| FR-204-AC-4 | A variant of `Retx` whose `send` has no `x >= 1 ms` guard and no reset settles `unsupported`, `ZeroDelayCycle`, naming the cycle's states. `probability >= 0.99 [always[0 ms, 4 ms] holds(not m.delivered)]` over `Retx` settles `unsupported`, `TimedFormShape`. | Test (TC-639) |
| FR-204-AC-5 | §15.5 `Retx`, whose `send` has no `delay` member, under the workload `One` (weight 1 on `send`): the workload resolves the scheduled identity and `send`'s delay stays the scheduler's choice in `[1, 2]` ms after each reset, with no distribution assumed for it. `probability >= 0.99 [eventually[0 ms, 4 ms] holds(m.delivered)]` under `One` settles `proved`, `ExactValue{99/100}`, its entry marked a minimum. `probability <= 0.999` over the same event under `One` settles `refuted` on the maximum `9999/10000`, its entry marked a maximum and its witness scheduler sending at `x = 1`. | Test (TC-642) |

## Dependencies

- ADR-028 TA-1 to TA-7, TA-1a, RU-6, XV-4; ADR-026 TS-1, TS-2, TS-4, CK-4,
  CK-7, SD-1, SD-3, SD-4, CT-1, CT-3, EZ-10 (on its own draft branch).
- [FR-196](FR-196-build-the-probabilistic-product.md),
  [FR-197](FR-197-decide-finite-horizon-forms-by-exact-backward-induction.md),
  [FR-198](FR-198-decide-unbounded-reachability-and-expected-rewards.md),
  [FR-202](FR-202-replay-a-probabilistic-witness.md).

## References

- QSpec half, which owns the digital route's semantics: QSpec FR-421
  (Linear STD-139), with FR-420's given and free delays.
- ADR-026 (timed subjects, delay distributions and the race) is on its own
  draft branch: Linear QSL-373.
- Owning ticket: Linear QSL-371.
