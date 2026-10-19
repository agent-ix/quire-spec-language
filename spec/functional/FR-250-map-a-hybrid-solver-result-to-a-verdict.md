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
  - target: ix://agent-ix/quire-specification/FR-193
    type: depends_on
---
# FR-250: Map a hybrid solver result to a verdict through the QSpec FR-193 provider

## Description

QSL SHALL lower a model's continuous variables, flows, guards and resets
in the form QSpec FR-193 admits, route a claim over them by negotiation to
a registered external solver provider, and map the provider's result to a
verdict (ADR-026 HY-1 to HY-3). EN-6 checks clocks, the variables whose
rate is 1 everywhere; every other rate and every flow goes to the provider.
A solver result settles `proved` only where its method is sound for the
claim, otherwise `inconclusive`, and never `refuted`.

## Use case

A robotics engineer models a vehicle's braking distance with a
differential equation and asks whether it never reaches an obstacle within
10 s. A δ-complete solver answers `unsat`, and the claim is proved within
that horizon and jump bound. A flowpipe that touches the unsafe set leaves
the claim inconclusive, because no solver witness replays exactly.

## Inputs

- A checked model with continuous variables and flows (QSpec FR-193's
  source form) and a claim over it.
- The provider's result: `SolverResult { method: SolverMethod, outcome:
  SolverOutcome, horizon: ExactRational, jumps: u64 }`,
  where `SolverOutcome` is `Unsat`, `DeltaSat`, `EnclosureDisjoint`,
  `EnclosureMeetsUnsafe`, `Unknown` or `Stopped(limit)`.

## Outputs

The FR-331 terminal record of the claim.

## Behavior

- The checker SHALL classify a variable as a clock when its rate is 1 in
  every location, and every other continuous variable as hybrid dynamics.
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
| `DeltaSat`, `EnclosureMeetsUnsafe`, `Unknown`, or a result from a method not sound for its outcome | `inconclusive` | `unsettled` | `Inconclusive(SolverInconclusive)` | inconclusive |
| `Stopped(limit)` | `incomplete`, execution `resource-incomplete` | `unavailable` | `Incomplete(LimitReached{limit, value, setting})` (ADR-018 V-7) | incomplete |

- No provider result SHALL settle `refuted`.
- A `BoundedSolver` proof SHALL carry the label `uncertified`: a solver
  QSL runs is a native engine with no core certificate checker (ADR-018
  PC-1, ADR-026 HY-2).
- A `BoundedSolver` proof SHALL hold for the claim within its horizon and
  jump bound, and the record SHALL carry both.
- The `solver-time` budget of a `Stopped(limit)` result, request member
  `max_solver_time`, SHALL be a count of whole seconds of wall-clock time. A
  duration with a fractional part SHALL round up to the next whole second, so
  a stop SHALL NOT report a budget smaller than the time the solver was
  allowed.
- The `solver-steps` budget, request member `max_solver_steps`, SHALL be a
  count of solver steps: whole, non-negative, as the provider counts its own
  steps, which QSL compares and reports unchanged and does not convert to time.
- A further hybrid engine SHALL be one more negotiated backend whose
  results take this map.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-250-AC-1 | A model with a variable `v` of rate `-a` and a clock `t` classifies `v` as hybrid and `t` as a clock; its claim routes to the registered fixture provider. With no provider registered it settles `unsupported`, `unsupported-requested-capability`. | Test (TC-705) |
| FR-250-AC-2 | Fixture results map as the table states: a δ-complete `Unsat` with horizon 10 and jumps 3 settles `Proved{BoundedSolver{…, horizon: 10, jumps: 3}, Uncertified}`; `DeltaSat` settles `inconclusive`, `SolverInconclusive`; `EnclosureDisjoint` from a flowpipe method without outward rounding settles `inconclusive`, `SolverInconclusive`; `Stopped` settles `incomplete`, `LimitReached`. No fixture result settles `refuted`. | Test (TC-705) |
| FR-250-AC-3 | A `Stopped` result for the `solver-time` budget with a requested limit of 1.5 s settles `incomplete`, `LimitReached{limit: solver-time, value: 2, setting: max_solver_time}`; a limit of exactly 2 s reports 2; a limit of 0.2 s reports 1. A `Stopped` result for the `solver-steps` budget with a limit of 1,000 steps reports `value: 1000`, `setting: max_solver_steps`, in steps and unchanged. | Test (TC-705) |

## Dependencies

- ADR-026 §13 HY-1 to HY-3; ADR-013 O-16.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md),
  [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md),
  [FR-230](FR-230-check-time-declarations-clocks-and-clock-constraints.md),
  [FR-235](FR-235-settle-a-timed-verdict-as-a-terminal-record.md).
- QSpec FR-193 (the solver contract and its source form).

## References

- QSpec half: QSpec FR-193 (Linear STD-139) owns the source form of
  continuous variables and flows and the verdict map with `BoundedSolver`
  and `SolverInconclusive` on the wire (ADR-026 OV-11).
- S. Gao, S. Kong and E. Clarke, 2014; T. A. Henzinger, P. W. Kopke, A. Puri
  and P. Varaiya, 1998 (ADR-026 References).
