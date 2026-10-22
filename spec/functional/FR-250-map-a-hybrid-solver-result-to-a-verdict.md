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
A solver result settles `proved` only where its method is sound for the
claim, otherwise `inconclusive`, and never `refuted`.

## Use case

A robotics engineer models a vehicle's braking distance with a
differential equation and asks whether it never reaches an obstacle within
10 s. A δ-complete solver answers `unsat`, and the claim is proved within
that horizon and jump bound. A flowpipe that touches the unsafe set leaves
the claim inconclusive, because no solver witness replays exactly; the path
the solver reports to the unsafe set stays on the record for the engineer to
read, marked unverified.

## Inputs

- A `CheckedHybridModel` and a `CheckedHybridClaim` over it (FR-447).
- The provider's result: `SolverResult { method: SolverMethod, outcome:
  SolverOutcome, horizon: ExactRational, jumps: u64 }`,
  where `SolverOutcome` is `Unsat`, `DeltaSat(Option<HybridTrace>)`,
  `EnclosureDisjoint`, `EnclosureMeetsUnsafe(Option<HybridTrace>)`,
  `Unknown(Option<HybridTrace>)` or `Stopped(limit)`, and

  ```rust
  pub struct HybridTrace { pub start: TraceState, pub jumps: Vec<TraceJump> }
  pub struct TraceJump { pub transition: u64, pub state: TraceState }
  pub struct TraceState {
      pub mode: ModeIndex,
      pub time: ExactInterval,                 // seconds
      pub enclosure: Vec<ExactInterval>,       // enclosure[i] encloses variables[i]
  }
  ```

  `SolverResult` is the QSL type an in-process hybrid provider (FR-447)
  returns. Its wire form is the FR-331 result entry of QSpec FR-193
  "Solver result on the wire", with `HybridTrace` in QSpec FR-193's
  "Hybrid trace" form, names in place of indices; a result entry a reader
  reads takes this map through that form.
- A provider overrun of its `deadline_ms` (QSpec FR-462) is not a
  `SolverOutcome`: the host ends the provider at `deadline_ms` and settles
  each item still open as `TimedOut`, and no `SolverResult` is read.

## Outputs

The FR-331 terminal record of the claim.

## Behavior

- S3 SHALL classify a continuous variable as a clock when its rate is the
  literal 1 in every mode, and every other continuous variable as hybrid
  dynamics, and SHALL record the class on the checked hybrid model (FR-447
  "The clock line"). The request writer SHALL read the class from the
  checked model and SHALL NOT classify again.
- The request writer SHALL give a claim whose subject has hybrid dynamics
  the requirement QSpec FR-193 names, for negotiation (FR-075) to route to a
  registered provider.
- If negotiation finds no candidate for such a claim, then QSL SHALL settle
  it `unsupported`, `unsupported-requested-capability`.
- QSL SHALL decide whether a result's method is sound for its outcome from
  the method kind by QSpec FR-193's classification, and SHALL read no
  soundness claim the provider makes about itself.
- The map SHALL be:

| Provider result | QSpec FR-360 label | QSpec FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- |
| `Unsat` or `EnclosureDisjoint` from a method sound for that outcome | `proved`, labelled `uncertified` | `closed-scope` | `Proved{basis: BoundedSolver{method, horizon, jumps}, certification: Uncertified}` | success |
| `DeltaSat`, `EnclosureMeetsUnsafe`, `Unknown`, or a result from a method not sound for its outcome | `inconclusive` | `unsettled` | `Inconclusive(SolverInconclusive{trace})`, `trace` the outcome's `Option<HybridTrace>`, `None` for an outcome that carries none | inconclusive |
| `Stopped(max_solver_steps)`, or any deterministic budget | `incomplete`, execution `resource-incomplete` | `unavailable` | `Incomplete(LimitReached{limit, value, setting})` (ADR-018 V-7) | incomplete |
| `Stopped(max_solver_time)`, or a provider overrun of its `deadline_ms` | `incomplete`, execution `resource-incomplete` | `unavailable` | `Incomplete(TimedOut)` (FR-293) | incomplete |
| A run the caller's frontend ends at its own deadline (FR-276) | `incomplete`, execution `resource-incomplete` | `unavailable` | `Incomplete(Cancelled{source: deadline})`, never `TimedOut` | incomplete |

- A horizon SHALL be a non-negative exact rational number of seconds, and
  the record SHALL write it in the `ExactRational` wire form.
- No provider result SHALL settle `refuted`.
- A `DeltaSat`, `EnclosureMeetsUnsafe` or `Unknown` outcome SHALL carry the
  path the solver reports to the possible violation as `Some(trace)`, and
  `None` when the solver reports no path. QSL SHALL keep that trace on the
  item's `inconclusive` record as `SolverInconclusive{trace}`, as QSpec
  FR-193-AC-2 retains it.
- A kept trace is unverified evidence in O-16 category inconclusive
  (ADR-013), so QSL SHALL neither replay it, make it a `Counterexample` or
  a `counterexamples` entry, nor settle any value from it.
- QSL SHALL check a trace against the claim's `CheckedHybridModel` before
  keeping it, and SHALL refuse the result `invalid_result`/`invalid-value`
  at the member as QSpec FR-193 "Hybrid trace" states: a mode or transition
  the model does not have, a `start` outside the initial mode, a jump to a
  mode other than its transition's target, an enclosure that does not have
  exactly one interval per continuous variable, and an interval whose lower
  bound exceeds its upper bound.
- A `BoundedSolver` proof SHALL carry the label `uncertified`: a solver
  QSL runs is a native engine with no core certificate checker (ADR-018
  PC-1, ADR-026 HY-2).
- A `BoundedSolver` proof SHALL hold for the claim within its horizon and
  jump bound, and the record SHALL carry both.
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
  the member and the provider SHALL apply its descriptor `bounds`; QSL's
  published defaults of 60 seconds and 1,000,000 steps are the descriptor
  bounds of its built-in hybrid solver, so an unset budget adds no member to
  the canonical key bytes.
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
| FR-250-AC-1 | A model with a variable `v` of rate `-a` and a clock `t` classifies `v` as hybrid and `t` as a clock; its claim routes to the registered fixture provider. With no provider registered it settles `unsupported`, `unsupported-requested-capability`. | Test (TC-705) |
| FR-250-AC-2 | Fixture results map as the table states: a δ-complete `Unsat` with horizon 10 and jumps 3 settles `Proved{BoundedSolver{…, horizon: 10, jumps: 3}, Uncertified}`; `DeltaSat` settles `inconclusive`, `SolverInconclusive`; `EnclosureDisjoint` from a flowpipe method without outward rounding settles `inconclusive`, `SolverInconclusive`; a `max_solver_steps` stop settles `incomplete`, `LimitReached{limit: max_solver_steps, ...}`; a `max_solver_time` stop and a provider `deadline_ms` overrun each settle `incomplete`, `TimedOut`; a frontend deadline settles `incomplete`, `Cancelled{source: deadline}`. No fixture result settles `refuted`. | Test (TC-705) |
| FR-250-AC-3 | A request with `max_solver_time` 1.5 s is admitted with a 2 s solver budget and the run uses 2; 2 s stays 2 and 0.2 s becomes 1; a request that sets none uses 60. A solver that exhausts the time budget, with no `Cancel` raised, settles `incomplete` with cause `TimedOut`, distinct from `Cancelled{source: deadline}` raised on a run whose frontend FR-276 deadline fires; a wire `max_solver_time` of 1.5, a zero budget, an unknown `hybrid.limits` member, an empty `hybrid` and a `hybrid` with an empty `limits` are each refused by the hybrid provider before consumption; a request that sets no budget omits both members; the stored record holds no wall-time member, and two requests that differ only in `max_solver_time` have equal canonical key bytes, a `TimedOut` result of the shorter one being stored by neither. A solver that stops at 1,000 steps settles `incomplete`, `LimitReached{limit: max_solver_steps, value: 1000, setting: hybrid.max_solver_steps}`, unchanged and in steps; a request that sets none uses 1,000,000. | Test (TC-705) |
| FR-250-AC-4 | A fixture `EnclosureMeetsUnsafe` over FR-447's `Braking` carrying a trace that starts in `Moving` and takes transition 0 to `Stopped` settles `inconclusive`, `SolverInconclusive{trace: Some(..)}`, the trace equal to the fixture's, with no `counterexamples` entry and no replay run; a `DeltaSat` carrying none settles `SolverInconclusive{trace: None}`; that trace with transition 1, with an enclosure missing `t`, and with a time interval whose lower bound is 3 and upper bound 2, is each refused `invalid_result`/`invalid-value` at the member. | Test (TC-705) |

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

## References

- QSpec half: QSpec FR-193 (Linear STD-139) owns the source form of
  continuous variables and flows and the verdict map with `BoundedSolver`
  and `SolverInconclusive` on the wire (ADR-026 OV-11).
- S. Gao, S. Kong and E. Clarke, 2014; T. A. Henzinger, P. W. Kopke, A. Puri
  and P. Varaiya, 1998 (ADR-026 References).
