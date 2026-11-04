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
  - target: ix://agent-ix/quire-spec-language/FR-195
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
integer value per clock and, for a deadline event, the deadline clock, plus
FR-196's complete tail control. Each edge has the mandatory closed
ProductAction choice. Delay/tail observation operation projections remain
`action: None`; this projection cannot identify the policy choice. Delay
choices carry one digital unit of elapsed time; IdleObserve carries zero. The results of
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
  admissible UnitDelay and enabled scheduled identities in running mode.
  A TS-5 quiescent running state additionally admits EnterIdleDelay: a
  positive unit delay committing irreversibly to idle-delayed. In idle-ready
  or idle-delayed, IdleDelay applies the actual delay guards/invariants,
  advances one unit and yields idle-delayed. IdleObserve is enabled only in
  idle-delayed, advances no clock/time, emits the current clock-valued
  stutter letter and yields idle-ready. Idle mode has no operation/reset
  action. Distinct literal action bodies and exact source modes are FR-196;
  no fictitious ScheduledIdentity names these choices.
- Under a workload, each racing identity SHALL draw its `discrete` delay
  conditioned on its window, the smallest draw winning with ties ordered by
  the workload's weights (ADR-026 SD-3); each race outcome SHALL be one
  edge, delaying by the winning amount and taking the winning identity, with
  its exact probability. A delay no distribution gives SHALL be an action of
  the scheduler. A state where no identity races and delay is bounded SHALL
  settle `Unsupported(NotStochastic)`.
- Under a workload, a state whose racing identities all have given delays
  (`discrete` distributions) SHALL resolve as above, each race edge carrying
  `Identity(r)` with `r` the first racing identity in canonical order.
- Under a workload, a state whose racing identities all have free delays
  SHALL offer `UnitDelay` and a single `WorkloadResolve{identity, delay: 0}`
  whose `identity` is the first enabled identity and whose probability mass
  splits over every enabled identity in proportion to the workload's weights.
- Under a workload, a state in which given-delay identities `G` and free-delay
  identities `F` race together SHALL NOT be refused. The given vectors `g` are
  the product of each given identity's `discrete` support conditioned on its
  window, in digital units, each with the product of the conditioned
  probabilities `P(g)`. The state SHALL offer the scheduler's choice of every
  free delay after the given delays are drawn (ADR-026 SD-3) as the MDP action
  `FreeDelays` (FR-196): one action per function `f` that maps each given
  vector `g` to an assignment `f(g)` of one delay in its window, in digital
  units, to every identity in `F`. One row of the action is one `g` with its
  `f(g)`; a row names every free racing identity, not one.
- The race of a row SHALL be ADR-026 SD-3's race over the delays `g` and
  `f(g)` together: the least delay `m` over all racing identities wins, and
  among the identities tied at `m` the winner is chosen by the workload's
  weights. The action is one distribution of total probability 1, the sum
  over rows of `P(g)` times the row's race outcome, each outcome delaying by
  `m` and taking its winner.
- `WorkloadResolve{identity, delay}` names one free identity and is made
  before the draw. In a mixed state with one free identity it is exactly the
  `FreeDelays` action whose rows all assign that delay to that identity; with
  two or more free identities it names no SD-3 choice, because the race needs
  a delay for every free identity at once. A row is not a sequence of
  `WorkloadResolve` choices: the race is one step that delays once. The mixed
  state SHALL therefore offer `FreeDelays` only; `WorkloadResolve` remains the
  choice of a state whose racing identities all have free delays, where no
  given draw exists and before and after the draw coincide.
- For every objective EN-5 decides (finite-horizon backward induction, `Reach`,
  expected rewards), the one-step operator at a FreeDelays state is linear in
  the successor distribution, so for a successor value vector `V` and
  `Q(g, a) = Σ_o race(g, a)(o) · V(o)`, optimizing over functions factors by
  row: `opt_f Σ_g P(g) Q(g, f(g)) = Σ_g P(g) opt_a Q(g, a)`, because each
  term depends on its own row only and every `P(g) > 0`. This is the value of
  ADR-028 SCH-2's intermediate-state form, one decision state per drawn `g`.
  The action form therefore loses nothing, adds no ProductKey member, and is
  the representation; a randomized choice over functions induces the same
  per-row distributions as a randomized choice per row.
- The witness policy at a FreeDelays state SHALL take FR-198's attaining
  choice and, among the assignments that attain a row's optimum, the least
  one, comparing the free delays as unsigned integers in `free` order.
- An every-scheduler run SHALL carry `Identity` where a workload run carries
  `WorkloadResolve` or `FreeDelays`, because there the scheduler chooses the
  identity itself.

### Evidence

- A witness scheduler SHALL name FR-202's closed ProductAction choices
  for every digital evidence/continuation state: actual enabled identity or
  residual PostState, WorkloadResolve, FreeDelays, UnitDelay, EnterIdleDelay, IdleDelay or IdleObserve
  according to the source mode. Under a workload it resolves only actual
  free-delay/observation choices; fixed race weights/distributions remain
  unchanged. A timed path names the same choice, exact elapsed amount and
  complete post-key; a race edge of a mixed given and free state names
  its `FreeDelays` action, and the witness scheduler's entry at that state is
  that action, one row per given vector. Neither action None nor a local node/index chooses an
  observation. Randomized exact-rational entries are allowed for derived
  TS-5 even without an authored fairness declaration.
- A timed witness path SHALL carry ADR-026 CT-1 delays and replay by CT-3
  through FR-202; replay SHALL check that its last timed state has no
  admissible discrete step before the event's time horizon passes.
- A certificate SHALL name digital product states, and FR-201's checker
  SHALL re-enumerate the digital MDP. Optimization and evidence SHALL
  apply ADR-028 §13a/FR-198 infimum/supremum rules over almost-surely
  TS-5-admissible schedulers, including positive finite-wait components
  with admissible exits. Original continuation graphs, observation edges
  and actual authored fairness are checked separately from objective
  absorption. No unqualified deterministic-memoryless attainment, finite
  waiting bound or new fairness premise decides elapsed expectations.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-204-AC-1 | §15.5 `Deadline` over every scheduler: `x` takes the values 0 to 2 and the deadline clock 0 to 5; the minimum is `99/100` (send at `x = 2`) and the maximum `9999/10000` (send at `x = 1`); the item settles `proved`, `ExactValue{99/100}`. At threshold 0.995 it settles `refuted` with a path of two losses at 2 ms and 4 ms of probability `1/100`, whose replay checks that the next `send` needs `x >= 1`, after 4 ms. | Test (TC-639) |
| FR-204-AC-2 | §15.5 `MeanTime` settles `proved`, `ExactValue{20/9 ms}` (maximum; the minimum is `10/9 ms`). The no-operation x>=1/cap2 oracle of FR-196 has elapsed infimum1 and supremum+infinity despite eventual observation almost surely; at threshold3 its geometric IdleDelay3/4/IdleObserve1/4 policy has exact value5 and a checked infinite observation continuation. A finite maximum3 or delay-only witness fails. With the guard `self.x > 1 ms` the items settle `unsupported`, `StrictClockConstraint`, naming the guard. | Test (TC-639) |
| FR-204-AC-3 | `Retx` with `delay Msg::send ~ discrete { 1 ms: 1, 2 ms: 1 };`, under a workload `One` with weight 1 on `send`, is a DTMC over digital timed states; `probability >= 0.99 [eventually[0 ms, 4 ms] holds(m.delivered)]` settles `proved`, `ExactValue{159129/160000}`: after every reset `send` waits 1 or 2 ms with probability 1/2 each, both inside its window `[1, 2]`, loses with `1/10`, and attempts at time stamps up to 4 ms inclusive count. The same claim `under every scheduler` settles `unsupported`, `DelayDistribution`, naming `send`; with `delay Msg::send ~ exponential(1 per ms);` under `One` it settles `unsupported`, `DelayDistribution`. | Test (TC-639) |
| FR-204-AC-4 | A variant of `Retx` whose `send` has no `x >= 1 ms` guard and no reset settles `unsupported`, `ZeroDelayCycle`, naming the cycle's states. `probability >= 0.99 [always[0 ms, 4 ms] holds(not m.delivered)]` over `Retx` settles `unsupported`, `TimedFormShape`. | Test (TC-639) |
| FR-204-AC-5 | §15.5 `Retx`, whose `send` has no `delay` declaration, under the workload `One` (weight 1 on `send`): the workload resolves the scheduled identity and `send`'s delay stays the scheduler's choice in `[1, 2]` ms after each reset, with no distribution assumed for it. `probability >= 0.99 [eventually[0 ms, 4 ms] holds(m.delivered)]` under `One` settles `proved`, `ExactValue{99/100}`, its entry marked a minimum. `probability <= 0.999` over the same event under `One` settles `refuted` on the maximum `9999/10000`, its entry marked a maximum and its witness scheduler choosing `UnitDelay` at `x = 0` and `WorkloadResolve{send, 0}` at `x = 1`. At the same quiescent D=1 idle-delayed key, IdleObserve reads success at D with elapsed0 while IdleDelay crosses D with elapsed1; replay/certificates preserve the distinct actions, post-keys and predicate positions. | Test (TC-642) |
| FR-204-AC-6 | The model `Race` (`time dense unit ms`) has `phase` (`Idle` or `Done`) and `b`; the operation `A` is guarded by `phase = Idle`, sets `Done` and declares `delay Race::A ~ discrete { 2 ms: 1 }`; the operation `B` has no `delay` declaration, is guarded by `phase = Idle and x >= 1 ms` under the time invariant `x <= 3 ms` while `Idle`, sets `Done` and `b`; the workload `Both` weights `A` and `B` 1. Under `Both`, `probability >= 0.5 [eventually[0 ms, 5 ms] holds(r.b)]` settles `refuted` on the minimum `ExactValue{0}`, its witness scheduler's `FreeDelays` entry holding the one row given `[2]` to `B` delay 3 (`A` wins at 2 ms, mass 1); `probability <= 0.5` settles `refuted` on the maximum `ExactValue{1}`, its witness entry holding the row given `[2]` to `B` delay 1 (`B` wins at 1 ms, mass 1). The row given `[2]` to `B` delay 2 is the tie at `m = 2`: `B` and `A` each win with probability `1/2`. The state offers the three `FreeDelays` actions and no `WorkloadResolve`. | Test (TC-642) |
| FR-204-AC-7 | With `B`'s guard changed to `phase = Idle and x >= 2 ms`, the choices of `B` are the delays 2 and 3, so the maximum is `1/2` (the tie) and the minimum `0`: `probability <= 0.5 [eventually[0 ms, 5 ms] holds(r.b)]` under `Both` settles `proved`, `ExactValue{1/2}`, its entry marked a maximum, and `probability <= 0.4` settles `refuted` on that maximum with the `FreeDelays` row given `[2]` to `B` delay 2 as its witness choice. The same claim `under every scheduler` settles `unsupported`, `DelayDistribution`, naming `A`, and not a mixed-delay refusal. | Test (TC-642) |
| FR-204-AC-8 | In `Race` with `A` declaring `delay Race::A ~ discrete { 1 ms: 1, 3 ms: 1 }`, `B` guarded by `phase = Idle and x >= 1 ms` under the invariant `x <= 3 ms` while `Idle`, and `Both` as before: `probability <= 0.7 [eventually[0 ms, 5 ms] holds(r.b)]` settles `refuted` on the maximum `ExactValue{3/4}` (with the draw `1`, `B` at delay 1 ties for `1/2`; with the draw `3`, `B` at delay 1 or 2 wins), and `probability >= 0.3` settles `refuted` on the minimum `ExactValue{1/4}` (with the draw `1`, `B` at delay 2 or 3 loses; with the draw `3`, `B` at delay 3 ties for `1/2`). | Test (TC-642) |
| FR-204-AC-9 | Over AC-8's `Race` the initial state offers nine `FreeDelays` actions (two rows, given `[1]` and `[3]`, each assigning `B` one of 1, 2, 3) and no `WorkloadResolve`. The witness scheduler for the maximum `3/4` holds the `FreeDelays` entry with the rows given `[1]` to `B` delay 1 and given `[3]` to `B` delay 1, and for the minimum `1/4` the rows given `[1]` to `B` delay 2 and given `[3]` to `B` delay 3 (least attaining delay per row). A single race whose payoff falls as `B`'s delay rises gives equal optima before and after the draw, so AC-8 cannot separate them; AC-10 does. | Test (TC-642) |
| FR-204-AC-10 | Separating vector. The model `Detour` (`time dense unit ms`) has `phase` (`Idle`, `Half` or `Done`) and `b`. `A`, guarded by `phase = Idle`, sets `Done` and `b` and declares `delay Detour::A ~ discrete { 1 ms: 1, 3 ms: 1 }`; `B` has no `delay` declaration, is guarded by `phase = Idle and x >= 1 ms` under the time invariant `x <= 3 ms` while `Idle`, and sets `Half`; `G` and `L` are guarded by `phase = Half` and declare `delay Detour::G ~ discrete { 1 ms: 1 }` and `delay Detour::L ~ discrete { 1 ms: 1 }`; `G` sets `Done` and `b`, `L` sets `Done`. The workload `All` weights `A`, `B`, `G` and `L` 1. For `eventually[0 ms, 2 ms] holds(d.b)`, write `Q(g, d)` for the probability after the draw `g` with `B` at delay `d`: `Q(1, 1) = 3/4` (tie: `A` wins `1/2`; `B` wins `1/2`, then `G` wins the tie at 2 ms with `1/2`), `Q(1, 2) = Q(1, 3) = 1` (`A` wins at 1 ms); `Q(3, 1) = 1/2` (`B` at 1 ms, then `G` at 2 ms), `Q(3, 2) = Q(3, 3) = 0` (nothing sets `b` by 2 ms). After the draw the maximum is `(1 + 1/2)/2 = 3/4` and the minimum `(3/4 + 0)/2 = 3/8`; a choice before the draw gives `5/8`, `1/2` and `1/2` for `B` at 1, 2 and 3, so maximum `5/8` and minimum `1/2`. Under `All`, `probability <= 0.7` settles `refuted` on the maximum `ExactValue{3/4}`, its witness entry the `FreeDelays` rows given `[1]` to `B` delay 2 and given `[3]` to `B` delay 1, and with `max_witness_paths` at least 2 its path set (`A` at 1 ms, `1/2`; `B` at 1 ms then `G` at 2 ms, `1/4`) replays `refuted`; `probability >= 0.45` settles `refuted` on the minimum `ExactValue{3/8}`, its witness entry the rows given `[1]` to `B` delay 1 and given `[3]` to `B` delay 2. A product that chose before the draw would prove both claims. | Test (TC-642) |
| FR-204-AC-11 | Every free identity in each row. AC-6's `Race` with a second free operation `C`, guarded by `phase = Idle and x >= 1 ms` under the same invariant, setting `Done` and not `b`, and `Both` weighting `A`, `B` and `C` 1: the initial state offers nine `FreeDelays` actions, each one row given `[2]` assigning both `B` and `C` a delay in `[1, 3]`. The maximum is `ExactValue{1}` with the row `B` delay 1, `C` delay 2; the minimum is `ExactValue{0}` with the row `B` delay 2, `C` delay 1 (`C` wins at 1 ms). A witness row naming `B` alone refuses `invalid_runtime_input`/`invalid-value` at replay. | Test (TC-642) |

## Rulings

Lead rulings of 2026-11-04 on the post-draw choice (QSL-1667), relayed from
the QSL plan lead (not an owner answer; Peter may veto). Each row is decided;
the last column names the fact that reopens it.

| Question | Ruling | Reason | Reopen when |
| --- | --- | --- | --- |
| How the post-draw choice is represented | As the MDP action `FreeDelays` (FR-196) at the pre-draw state, one action per function from given vectors to free assignments, not as an intermediate chance-then-decision state with a new ProductKey member. | ADR-026 SD-3 names the choice; the factorization in Behavior shows the action form has the value of ADR-028 SCH-2's intermediate-state form for every objective EN-5 decides, so no new ProductKey tag is needed. | An objective whose one-step operator is not linear in the successor distribution is admitted at a mixed race, or the enumerated actions exceed `max_transitions` on a model the product must decide; then a pending given-draw ProductKey member with canonical bytes replaces the enumeration. |
| Which free identities a row covers | Every free racing identity at the state; a row naming a subset refuses. `WorkloadResolve` names one free identity before the draw; in a mixed state with one free identity it equals the constant `FreeDelays` action, with two or more it names no SD-3 choice, and the mixed state offers `FreeDelays` only (author decision under this ruling). | SD-3's race delays once over every racing identity, so it needs every free delay in one step; keeping both names in a mixed state would give one decision two spellings. | SD-3 is amended to let the workload exclude free identities from a race. |
| Witness types in scope | `WitnessScheduler` and `WitnessEntry` (FR-202) are specified here; FR-202's separate `WitnessChoice` wrapper is removed, the choice is FR-196's `ProductAction`, of which `FreeDelays` is a member (author decision under this ruling). | One closed choice type gives one canonical body for the product edge, the witness entry and the certificate policy; a second enum with a second `free-delays` spelling would duplicate it. | A witness choice is needed that is not a product action. |

## Dependencies

- The post-draw `FreeDelays` action (AC-6 to AC-11) is implemented under
  QSL-1667, in the lane of the EN-5 provider ticket QSL-1498. That change
  replaces the mixed-state `WorkloadResolve` path in the digital product in
  the same PR; it is not a Layer 4 deletion-table unit.
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
