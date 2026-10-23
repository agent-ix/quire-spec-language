---
id: FR-250
title: "Map a hybrid solver result to a verdict through the QSpec FR-193 provider"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-235
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-450
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: references
  - target: ix://agent-ix/quire-specification/FR-193
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
---
# FR-250: Map a hybrid solver result to a verdict through the QSpec FR-193 provider

## Description

QSL SHALL lower a model's continuous variables, flows, guards and resets
in the form QSpec FR-193 admits (FR-447), route a claim over them by
negotiation to a registered external solver provider, and map the provider's
result to a verdict (ADR-026 HY-1 to HY-3). EN-6 checks clocks, the variables whose
rate is 1 everywhere; every other rate and every flow goes to the provider.
A solver result settles `proved` only where its method is sound for its
outcome and its proof covers the claim, otherwise `inconclusive`, and
`refuted` only when QSL replays its trace exactly (FR-450).

## Use case

A robotics engineer models a vehicle's braking distance with a
differential equation and asks whether it never reaches an obstacle within
10 s. A δ-complete solver answers `unsat`, and the claim is proved within
that horizon and jump bound. A flowpipe that touches the unsafe set leaves
the claim inconclusive, because its trace is an enclosure that QSL replays
only statically (FR-450); the path the solver reports to the unsafe set
stays on the record for the engineer to read, with its replay outcome.

## Inputs

- A `CheckedHybridModel` and a `CheckedHybridClaim` over it (FR-447).
- The provider's result: `SolverResult { method: SolverMethod, outcome:
  SolverOutcome, coverage: SolverCoverage }`, where `SolverMethod` is one
  kind of QSpec FR-193's closed method table ("Solver methods"):

  ```rust
  pub enum SolverMethod {
      DeltaComplete,           // "delta-complete"
      FlowpipeOutwardRounded,  // "flowpipe-outward-rounded"
      FlowpipeUnrounded,       // "flowpipe-unrounded"
      InductiveEnclosure,      // "inductive-enclosure"
      Unclassified,            // "unclassified": any other method
  }
  pub enum SolverCoverage {
      Bounded {
          horizon: ExactRational,            // seconds
          jumps: u64,                        // jump count
          exhaustive: BTreeSet<BoundDimension>, // the dimensions the solver established complete
      },
      Inductive,               // every time and every jump count
  }
  pub enum BoundDimension {
      Time,   // "time": no behaviour takes more than `jumps` jumps by `horizon`
      Jumps,  // "jumps": no behaviour takes one of its first `jumps` jumps after `horizon`
  }
  ```

  `SolverOutcome` is `Unsat`, `DeltaSat(Option<HybridTrace>)`,
  `EnclosureDisjoint`, `EnclosureMeetsUnsafe(Option<HybridTrace>)`,
  `Unknown(Option<HybridTrace>)`, `NoInitialState` (the solver establishes that the model's initial set, QSpec FR-193 "Initial states", is empty) or `Stopped(limit)`, and

  ```rust
  pub struct HybridTrace {
      pub start: TraceState,
      pub jumps: Vec<TraceJump>,
      pub delta: Option<ExactRational>,        // the solver's tolerance; None reads 0
      pub loop_to: Option<u64>,                // the state a lasso's last state returns to
  }
  pub struct TraceJump {
      pub transition: u64,
      pub before: Option<TraceBefore>,         // the state just before the jump
      pub state: TraceState,
  }
  pub struct TraceBefore { pub time: ExactInterval, pub enclosure: Vec<ExactInterval> }
  pub struct TraceState {
      pub mode: ModeIndex,
      pub time: ExactInterval,                 // seconds
      pub enclosure: Vec<ExactInterval>,       // enclosure[i] encloses variables[i]
  }
  ```

  `SolverResult` is the QSL type an in-process hybrid provider (FR-447)
  returns. Its wire form is the FR-331 result entry of QSpec FR-193
  "Solver result on the wire", with `HybridTrace` in QSpec FR-193's
  "Hybrid trace" form, names in place of indices. A process hybrid provider
  (FR-447) returns that result entry, and a result entry a reader reads takes
  this map through that form.
- A provider overrun of its `deadline_ms` (QSpec FR-462) is not a
  `SolverOutcome`: the host ends the provider at `deadline_ms` and settles
  each item still open as `TimedOut`, and no `SolverResult` is read.

## Outputs

The FR-331 terminal record of the claim.

## Behavior

- The request writer SHALL read each continuous variable's class, clock or
  hybrid dynamics, from the checked hybrid model, where S3 records it by
  FR-447 "The clock line", and SHALL NOT classify again.
- The request writer SHALL give a claim whose subject has hybrid dynamics
  the requirement QSpec FR-193 "Routing a hybrid item" names, the item
  member `"requirements": ["hybrid-dynamics"]` (FR-447), for negotiation
  (FR-075) to route to a registered provider whose descriptor `dynamics`
  holds `hybrid`.
- If negotiation finds no candidate for such a claim, then QSL SHALL settle
  it `unsupported`, `unsupported-requested-capability`.
- QSL SHALL decide whether a result's method is sound for its outcome from
  the method kind by QSpec FR-193's method table, and SHALL read no
  soundness claim the provider makes about itself. A result SHALL be sound
  exactly when it is `(DeltaComplete, Unsat, Bounded{..})`,
  `(FlowpipeOutwardRounded, EnclosureDisjoint, Bounded{..})` or
  `(InductiveEnclosure, EnclosureDisjoint, Inductive)`; every other triple
  of method, outcome and coverage, `FlowpipeUnrounded` and `Unclassified`
  with any outcome included, is not sound.
- QSL SHALL decide whether a sound `Bounded` result covers the claim by
  QSpec FR-193 "Proof coverage", from the `CheckedHybridClaim`'s clock and
  horizon (FR-447). A bounded proof read the states a behaviour reaches by
  time `horizon` with at most `jumps` jumps, and covers the claim only when
  those include every state the claim reads, so both bounds count: under
  `ModelTime` a horizon `Some(Seconds(r))` is covered exactly when
  `horizon >= r` and `exhaustive` holds `Time`; under `ModelSteps` a horizon
  `Some(Positions(n))` is covered exactly when `jumps >= n` and either `n`
  is 0 or `exhaustive` holds `Jumps`, a position count never converted to
  seconds; and a horizon `None` is never covered. An `Inductive` result
  covers every claim.
- The map SHALL be:

| Provider result | QSpec FR-360 label | QSpec FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- |
| A sound `Bounded{horizon, jumps, exhaustive}` result that covers the claim | `proved`, labelled `uncertified` | `closed-scope` | `Proved{basis: BoundedSolver{method, horizon, jumps, exhaustive}, certification: Uncertified}` | success |
| A sound `Inductive` result | `proved`, labelled `uncertified` | `closed-scope` | `Proved{basis: InductiveSolver{method}, certification: Uncertified}` | success |
| A sound `Bounded{horizon, jumps, exhaustive}` result that does not cover the claim | `inconclusive` | `unsettled` | `Inconclusive(InsufficientHorizon{horizon, jumps, exhaustive})`, never `Proved` | inconclusive |
| `DeltaSat`, `EnclosureMeetsUnsafe`, `Unknown`, or a result that is not sound | `inconclusive` | `unsettled` | `Inconclusive(SolverInconclusive{trace, replay})`, `trace` the outcome's `Option<HybridTrace>`, `None` for an outcome that carries none, and `replay` its FR-450 static replay outcome, `None` with no trace | inconclusive |
| `NoInitialState` | `inconclusive` | `unsettled` | `Inconclusive(NoInitialState)`, never `Proved`: a model with no behaviour proves nothing (QSpec FR-193 "Initial states") | inconclusive |
| `Stopped(max_solver_steps)`, or any deterministic budget | `incomplete`, execution `resource-incomplete` | `unavailable` | `Incomplete(LimitReached{limit, value, setting})` (ADR-018 V-7) | incomplete |
| `Stopped(max_solver_time)`, or a provider overrun of its `deadline_ms` | `incomplete`, execution `resource-incomplete` | `unavailable` | `Incomplete(TimedOut)` (FR-293) | incomplete |
| A run the caller's frontend ends at its own deadline (FR-276) | `incomplete`, execution `resource-incomplete` | `unavailable` | `Incomplete(Cancelled{source: deadline})`, never `TimedOut` | incomplete |

- A horizon SHALL be a non-negative exact rational number of seconds, and
  the record SHALL write it in the `ExactRational` wire form.
- A provider result SHALL settle `refuted` only through FR-450's exact
  replay of its trace, and every other provider result SHALL take the map
  above.
- A `DeltaSat`, `EnclosureMeetsUnsafe` or `Unknown` outcome SHALL carry the
  path the solver reports to the possible violation as `Some(trace)`, and
  `None` when the solver reports no path. QSL SHALL keep that trace on the
  item's `inconclusive` record as `SolverInconclusive{trace, replay}`, as
  QSpec FR-193-AC-2 retains it.
- A kept trace is unverified evidence in O-16 category inconclusive
  (ADR-013) until replay: QSL SHALL replay it by FR-450, keep its static
  replay outcome as `replay`, and make it a `Counterexample` and a
  `counterexamples` entry only when FR-450's exact replay reproduces it.
- QSL SHALL check a trace against the claim's `CheckedHybridModel` before
  keeping it, and SHALL refuse the result `invalid_result`/`invalid-value`
  at the member as QSpec FR-193 "Hybrid trace" states: a mode or transition
  the model does not have, a `start` outside every initial mode, a jump to a
  mode other than its transition's target, an enclosure that does not have
  exactly one interval per continuous variable, and an interval whose lower
  bound exceeds its upper bound.
- A `BoundedSolver` or `InductiveSolver` proof SHALL carry the label
  `uncertified`: a solver QSL runs is a native engine with no core
  certificate checker (ADR-018 PC-1, ADR-026 HY-2).
- A `BoundedSolver` proof SHALL hold for the claim within its horizon and
  jump bound, every state the claim reads included, and the record SHALL
  carry the horizon, the jump bound and `exhaustive`.
- An `InductiveSolver` proof SHALL hold for the claim at every time and
  after any number of jumps.
- QSL SHALL check the coverage of every `bounded-solver` proof it reads
  from a result entry, a plugin's `trusted` proof included (FR-290), against
  the claim's `CheckedHybridClaim`, and SHALL settle a proof that does not
  cover the claim `Inconclusive(InsufficientHorizon{horizon, jumps, exhaustive})` with
  the entry's own horizon, jumps and `exhaustive`. Coverage reads the proof's bounds and
  the claim, not the method's soundness, so it is not a second soundness
  decision.
- QSL's result-entry reader SHALL refuse a first-party `uncertified`
  `bounded-solver` entry whose `method` is `flowpipe-unrounded` or
  `unclassified` with `invalid_result`/`invalid-value` at `/basis/method`
  (QSpec FR-193 "Solver result on the wire"), since no outcome of those
  kinds is sound.
- A hybrid item SHALL carry its budgets as `hybrid.limits.max_solver_steps`
  and `hybrid.limits.max_solver_time`, each a whole positive count, and their
  setting names are `hybrid.max_solver_steps` and `hybrid.max_solver_time`
  (QSpec FR-331).
- If a hybrid item has an empty `hybrid`, a `hybrid` with an empty `limits`,
  a zero budget, or a `hybrid.limits` member other than these two, then the
  hybrid provider SHALL refuse the request before consumption, as QSpec
  FR-331 refuses any out-of-domain value.
- `max_solver_time` SHALL be a request setting of its own, independent of
  FR-276's deadline, that the caller may give as a possibly fractional
  duration in seconds. When a request is admitted with a fractional
  `max_solver_time`, the request writer SHALL round it up to the next whole
  second and write that whole-second integer as the wire value; the run uses
  the rounded value.
- If a wire `max_solver_time` is fractional, then the hybrid provider SHALL
  refuse the request before consumption, as QSpec FR-331 refuses any
  out-of-domain value.
- `max_solver_time` is a wall-clock setting, so it SHALL be outside the
  canonical request key (FR-292), no member of a stored record SHALL record
  wall time, and a result it limits SHALL NOT be stored.
- A solver that exhausts `max_solver_time`, and a provider that overruns its
  `deadline_ms`, SHALL settle `incomplete` with the cause
  `IncompleteCause::TimedOut` (FR-293), not `LimitReached`.
- A run the caller's frontend ends at its own deadline (FR-276) SHALL settle
  `incomplete`, execution `resource-incomplete`, with the cause
  `Cancelled{source: deadline}`, so no event has two causes.
- When the caller sets no value for a budget, the request writer SHALL omit
  the member and the provider SHALL apply its descriptor `bounds`, so an
  unset budget adds no member to the canonical key bytes. QSL has no
  built-in hybrid solver (ADR-026 HY-3): the defaults are those each
  registered hybrid provider publishes, and every descriptor whose
  `dynamics` holds `hybrid` SHALL carry both `max_solver_steps` and
  `max_solver_time` in its `bounds`, a descriptor that lacks either being
  refused `invalid_capability`/`invalid-dynamics` (FR-075, QSpec FR-193
  "Routing a hybrid item"). TC-705's in-process fixture hybrid provider
  publishes `max_solver_time` 60 and `max_solver_steps` 1,000,000.
- The `max_solver_steps` request member SHALL be a count of solver steps:
  whole and positive, as the provider counts its own steps. QSL SHALL compare
  and report a `max_solver_steps` value as the provider counts it, and SHALL
  NOT convert it to time. A solver that exhausts it settles `incomplete`,
  `LimitReached{limit: max_solver_steps, value, setting:
  hybrid.max_solver_steps}`.
- A further hybrid engine SHALL be one more negotiated backend whose
  results take this map.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-250-AC-1 | A model with a variable `v` of rate `-a` and a clock `t` classifies `v` as hybrid and `t` as a clock; its claim's item carries `"requirements": ["hybrid-dynamics"]` and routes to the registered fixture provider, whose descriptor `dynamics` is `["hybrid"]`, and not to EN-6 registered beside it. With no hybrid provider registered, EN-6 alone included, it settles `unsupported`, `unsupported-requested-capability`. | Test (TC-705) |
| FR-250-AC-2 | Fixture results over FR-447's `SlowEnough` (horizon `Seconds(10)`) map as the table states: a δ-complete `Unsat` with `Bounded{horizon: 10, jumps: 3, exhaustive: {Time}}` settles `Proved{BoundedSolver{…, horizon: 10, jumps: 3, exhaustive: {Time}}, Uncertified}`; `DeltaSat` settles `inconclusive`, `SolverInconclusive`; `EnclosureDisjoint` from a flowpipe method without outward rounding settles `inconclusive`, `SolverInconclusive`; a `max_solver_steps` stop settles `incomplete`, `LimitReached{limit: max_solver_steps, ...}`; a `max_solver_time` stop and a provider `deadline_ms` overrun each settle `incomplete`, `TimedOut`; a frontend deadline settles `incomplete`, `Cancelled{source: deadline}`. No fixture result settles `refuted`. | Test (TC-705) |
| FR-250-AC-3 | A request with `max_solver_time` 1.5 s is admitted with a 2 s solver budget and the run uses 2; 2 s stays 2 and 0.2 s becomes 1; a request that sets none uses the fixture provider's published `max_solver_time`, 60. A solver that exhausts the time budget, with no `Cancel` raised, settles `incomplete` with cause `TimedOut`, distinct from `Cancelled{source: deadline}` raised on a run whose frontend FR-276 deadline fires; a wire `max_solver_time` of 1.5, a zero budget, an unknown `hybrid.limits` member, an empty `hybrid` and a `hybrid` with an empty `limits` are each refused by the hybrid provider before consumption; a request that sets no budget omits both members; the stored record holds no wall-time member, and two requests that differ only in `max_solver_time` have equal canonical key bytes, a `TimedOut` result of the shorter one being stored by neither. A solver that stops at 1,000 steps settles `incomplete`, `LimitReached{limit: max_solver_steps, value: 1000, setting: hybrid.max_solver_steps}`, unchanged and in steps; a request that sets none uses the fixture provider's published `max_solver_steps`, 1,000,000. | Test (TC-705) |
| FR-250-AC-4 | A fixture `EnclosureMeetsUnsafe` over FR-447's `Braking` carrying a trace that starts in `Moving` and takes transition 0 to `Stopped` settles `inconclusive`, `SolverInconclusive{trace: Some(..), replay: Some(..)}`, the trace equal to the fixture's and `replay` its FR-450 static replay outcome, with no `counterexamples` entry; a `DeltaSat` carrying none settles `SolverInconclusive{trace: None}`; that trace with transition 1, with an enclosure missing `t`, and with a time interval whose lower bound is 3 and upper bound 2, is each refused `invalid_result`/`invalid-value` at the member. Through the plugin reader (FR-290), a `solver-inconclusive` entry carrying that trace in QSpec FR-193's wire form on the `Braking` claim reads `SolverInconclusive{trace: Some(..)}` with the same trace, and the same entry on an FR-468 refinement item is `Malformed` at `/cause/trace`. | Test (TC-705) |
| FR-250-AC-5 | Over FR-447's `Braking` with `initial { mode Moving when { v = 30 }; }`, whose condition `Moving`'s invariant `v <= 20` excludes, a fixture `NoInitialState` settles `inconclusive`, `NoInitialState`, never `Proved`, with no `counterexamples` entry; through the plugin reader a `no-initial-state` entry on that claim reads the same. | Test (TC-705) |
| FR-250-AC-6 | Over `SlowEnough`, each with `Bounded{horizon: 10, jumps: 3, exhaustive: {Time}}` unless stated: `(DeltaComplete, Unsat)` settles `Proved{BoundedSolver{DeltaComplete, 10, 3, {Time}}, Uncertified}`; `(FlowpipeOutwardRounded, EnclosureDisjoint)` settles `Proved{BoundedSolver{FlowpipeOutwardRounded, 10, 3, {Time}}, Uncertified}`; `(InductiveEnclosure, EnclosureDisjoint, Inductive)` settles `Proved{InductiveSolver{InductiveEnclosure}, Uncertified}`; `(DeltaComplete, EnclosureDisjoint)`, `(FlowpipeOutwardRounded, Unsat)`, `(InductiveEnclosure, EnclosureDisjoint, Bounded{10, 3, {Time}})`, `(DeltaComplete, Unsat, Inductive)`, and `FlowpipeUnrounded` and `Unclassified` with `Unsat` and with `EnclosureDisjoint` each settle `inconclusive`, `SolverInconclusive{trace: None}`. Each `Proved` record writes the basis `{"type": "bounded-solver", "method": "delta-complete", "horizon": {"numerator": "10", "denominator": "1"}, "jumps": 3, "exhaustive": ["time"]}` or `{"type": "inductive-solver", "method": "inductive-enclosure"}` with its spelling. A first-party `bounded-solver` entry with `method` `flowpipe-unrounded` is refused `invalid_result`/`invalid-value` at `/basis/method`. | Test (TC-705) |
| FR-250-AC-7 | A sound `(DeltaComplete, Unsat)` result settles, over FR-447's `Braking`, which has no `initial` clause, so a behaviour starts in `Moving` with any `v0` in `[0, 20]` and `a` in `[1, 2]` and takes its one jump at time `v0 / a`, anywhere in `[0, 20]`, and no second jump (`Recent` and `Slows` are over `Braking2`): over `SlowEnough` (horizon 10 s) with `Bounded{horizon: 10, jumps: 1, exhaustive: {Time}}` or `horizon` 21/2, `Proved`; with `horizon` 1, `Inconclusive(InsufficientHorizon{horizon: 1, jumps: 1, exhaustive: {Time}})`; with `Bounded{10, 0, {}}`, `InsufficientHorizon`, a behaviour possibly jumping at any time in `[0, 10]`; with `Bounded{20, 3, {Jumps}}`, `InsufficientHorizon`; over the claim `always [0, 10] eventually [0, 5] holds(h.v < 30)` (horizon 15 s) with `Bounded{10, 1, {Time}}`, `InsufficientHorizon`; over `Recent` (`ModelSteps`, horizon `Positions(0)`) with `Bounded{1, 0, {}}`, `Proved`; over `eventually [0, 1] holds(h.v = 0)` under `clock "model-steps"` (horizon `Positions(1)`) with `Bounded{20, 1, {Jumps}}`, `Proved`, with `Bounded{10, 1, {}}`, `InsufficientHorizon{horizon: 10, jumps: 1, exhaustive: {}}`, the jump possibly occurring at time 20 (`v0` 20, `a` 1), and with `Bounded{100, 0, {Jumps}}`, `InsufficientHorizon`; over `Slows` (horizon `None`) and over `always holds(h.v < 30)` with any `Bounded` coverage, `InsufficientHorizon`, and as `(InductiveEnclosure, EnclosureDisjoint, Inductive)`, `Proved{InductiveSolver{..}}`. Through the plugin reader, a plugin's `trusted` `bounded-solver` proof with `horizon` 1, `jumps` 1 and `exhaustive` `["time"]` on `SlowEnough` settles `inconclusive`, `InsufficientHorizon{horizon: 1, jumps: 1, exhaustive: {Time}}`, and with `horizon` 10 `proved`, `trusted`. No `InsufficientHorizon` record carries a trace or a `counterexamples` entry. | Test (TC-705) |

## Dependencies

- ADR-026 §13 HY-1 to HY-3; ADR-013 O-16.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md),
  [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md),
  [FR-230](FR-230-check-time-declarations-clocks-and-clock-constraints.md),
  [FR-235](FR-235-settle-a-timed-verdict-as-a-terminal-record.md).
- QSpec FR-193 (the solver contract and its source form).
- QSpec FR-331 (the `hybrid.limits` budgets and their refusals, and the
  `solver-inconclusive{trace}` cause).
- [FR-447](FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md)
  (the checked hybrid model, its classification and the provider request).
- [FR-450](FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md)
  (trace replay and the hybrid refutation).

## References

- QSpec half: QSpec FR-193 (Linear STD-139) owns the source form of
  continuous variables and flows and the verdict map with `BoundedSolver`
  and `SolverInconclusive` on the wire (ADR-026 OV-11).
- S. Gao, S. Kong and E. Clarke, 2014; T. A. Henzinger, P. W. Kopke, A. Puri
  and P. Varaiya, 1998 (ADR-026 References).
