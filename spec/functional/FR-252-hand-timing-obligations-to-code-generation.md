---
id: FR-252
title: "Hand timing obligations to code generation"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-251
    type: depends_on
---
# FR-252: Hand timing obligations to code generation

## Description

QSL SHALL hand CG, for each tick-based monitor plan (FR-251) and each
contract that relates timestamp parameters, the timing obligations that CG
discharges with Kani (ADR-026 KG-1 to KG-4). These are obligations QSL
writes; how CG emits harnesses is CG's. An obligation about elapsed real
time or WCET is outside Kani's model, so QSL writes it as unsupported, and
schedulability goes to EN-7. QSL has no authored form for a claim about
preemption or interrupt timing, so it writes no obligation for one.

## Use case

An embedded engineer generates a monitor and its code obligations. They
learn that the monitor agrees with QSL's evaluator on every short trace,
that tick arithmetic never overflows across wraparound, and that a claim
about measured execution time was not silently dropped but reported as
unsupported.

## Inputs

- A `TimedMonitorPlan` (FR-251), and checked contracts over integer
  timestamp parameters.
- `TimingObligationSettings { agreement_horizon: u64, timestamp_parameters:
  Map<ClauseId, Set<Name>> }`, method parameters of the request. `agreement_horizon`,
  default 8, is the agreement obligation's horizon in events; it defines what
  the obligation proves, and the obligation states it. `timestamp_parameters`,
  default empty, names, per checked contract, the integer parameters of that
  contract that carry timestamps; a name is read only within the contract that
  keys it, so two contracts that each have a parameter `now` are told apart; the caller states them as a premise, as it states
  `max_reading_gap`, because an integer parameter's type does not say it is a
  timestamp.

## Outputs

```rust
pub enum TimingObligation {
    MonitorAgreement { plan: PlanId, events: u64 },
    TickArithmetic { plan: PlanId, width_bits: u32, buffers: Vec<(SubformulaId, u64)>, rate: ExactRational },
    TimestampContract { contract: ClauseId },
}
```

in the obligations QSL writes for CG, and an `Unsupported` disposition for
each timing obligation outside the code model.

## Behavior

- For each monitor plan QSL SHALL write a `MonitorAgreement` obligation:
  the monitor's step function agrees with the layer-5 reference evaluator
  on every trace of up to `agreement_horizon` events with symbolic integer time
  stamps. The obligation SHALL carry `agreement_horizon` as its stated horizon,
  and a discharged obligation holds for traces up to that length only.
- For each monitor plan QSL SHALL write a `TickArithmetic` obligation: tick
  arithmetic never panics or overflows, modular differences are correct
  across wraparound, and no buffer exceeds its capacity under the plan's
  rate.
- When a clause of a contract relates two or more of the integer parameters
  that `timestamp_parameters` names for that contract, QSL SHALL write a
  `TimestampContract` obligation for it over symbolic integers. A contract
  with no such clause gets no `TimestampContract` obligation, including a
  contract whose clauses compare one named parameter only with a constant or
  with an unnamed parameter.
- A **timing clause** is a comparison `m <op> d` whose left operand `m` is
  `elapsed_wall_time(h)`, the elapsed real time of handler `h`, or `wcet(f)`,
  the worst-case execution time of function `f`, whose right operand `d` is a
  time quantity, and whose `<op>` is one of `<`, `<=`, `=`, `>=` and `>`.
- QSL SHALL recognise `elapsed_wall_time(h)` and `wcet(f)` in a timing clause
  before name resolution: it takes `h` and `f` as written, resolves neither
  against the unit's declarations, types the left operand as a time quantity,
  and refuses nothing at S1 to S3 for either form. This recognition applies
  only to the left operand of a timing clause and wins over a function of
  the same name that the unit declares. Anywhere else (the right operand,
  or inside another expression) `elapsed_wall_time` and `wcet` are ordinary
  names that `Application` resolves (FR-065): a unit's own declaration of
  that name is called, and an undeclared one is refused as `missing-name`.
  A comparison whose left operand is one of the two forms and whose right
  operand is not a time quantity is refused as `ill_typed`/`type-mismatch`
  at the comparison.
- QSL SHALL dispose a timing clause as `unsupported`,
  `unsupported-requested-capability`, naming its timing kind: a timing clause
  with `elapsed_wall_time` names elapsed real time and one with `wcet` names
  WCET.
- QSL SHALL route a schedulability claim to EN-7 (FR-246), with WCET as a
  stated premise.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-252-AC-1 | FR-251-AC-1's plan yields a `MonitorAgreement` obligation with `events` 8 and a `TickArithmetic` obligation carrying the counter's width, the plan's buffers and its rate; with `agreement_horizon` 4 the agreement obligation carries 4 and states that it covers traces of up to 4 events. | Test (TC-707) |
| FR-252-AC-2 | A contract `when now - start > d then timed_out` over `u32` parameters, with `timestamp_parameters` keying that contract to `{now, start}`, yields a `TimestampContract` obligation; a clause `elapsed_wall_time(handler) <= 50 µs` is disposed `unsupported`, `unsupported-requested-capability`, naming elapsed real time; a clause bounding a function's WCET is disposed the same way, naming WCET; `elapsed_wall_time(handler)` with `handler` declared nowhere is still disposed `unsupported` and not refused at name resolution. | Test (TC-707) |
| FR-252-AC-3 | The contract of FR-252-AC-2 with `timestamp_parameters` empty yields no `TimestampContract` obligation, as does a contract that compares only `now` with a constant while `timestamp_parameters` names `{now, start}`; the clauses `elapsed_wall_time(handler) <= 50 µs` and `wcet(f) < 2 ms` are timing clauses, and a comparison of two `u32` parameters is not. | Test (TC-707) |
| FR-252-AC-4 | A unit that declares its own function `wcet` still has `wcet(f) < 2 ms` taken as a timing clause; in `x = wcet(f)` and in `2 ms > elapsed_wall_time(handler)`, with no unit declaration of either name, the form is refused as `missing-name`; `wcet(f) < 3` is refused as `ill_typed`/`type-mismatch`. | Test (TC-707) |

## Dependencies

- ADR-026 §14 KG-1 to KG-4; ADR-014 B-5.
- [FR-251](FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md).

## References

- The CG obligation shapes: ADR-026 OV-13.
- QSpec half: QSpec FR-415 and FR-416 (Linear STD-139), the timed model
  and claim semantics the obligations carry.
