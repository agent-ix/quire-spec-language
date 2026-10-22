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
time, WCET, preemption or interrupt timing is outside Kani's model, so QSL
writes it as unsupported, and schedulability goes to EN-7.

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
  Set<Name> }`, method parameters of the request. `agreement_horizon`,
  default 8, is the agreement obligation's horizon in events; it defines what
  the obligation proves, and the obligation states it. `timestamp_parameters`,
  default empty, names the integer parameters of a checked contract that carry
  timestamps; the caller states them as a premise, as it states
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
- When a contract's clauses relate integer parameters that
  `timestamp_parameters` names, QSL SHALL write a `TimestampContract`
  obligation for it over symbolic integers. A contract whose clauses name
  none of them gets no `TimestampContract` obligation.
- A **timing clause** is a comparison `m <op> d` whose left operand `m` is
  `elapsed_wall_time(h)`, the elapsed real time of handler `h`, or `wcet(f)`,
  the worst-case execution time of function `f`, whose right operand `d` is a
  time quantity, and whose `<op>` is one of `<`, `<=`, `=`, `>=` and `>`. QSL
  SHALL dispose a timing obligation that reads elapsed real time, WCET,
  preemption or interrupt timing as `unsupported`,
  `unsupported-requested-capability`, naming its timing kind; a timing clause
  with `elapsed_wall_time` names elapsed real time and one with `wcet` names
  WCET.
- QSL SHALL route a schedulability claim to EN-7 (FR-246), with WCET as a
  stated premise.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-252-AC-1 | FR-251-AC-1's plan yields a `MonitorAgreement` obligation with `events` 8 and a `TickArithmetic` obligation carrying the counter's width, the plan's buffers and its rate; with `agreement_horizon` 4 the agreement obligation carries 4 and states that it covers traces of up to 4 events. | Test (TC-707) |
| FR-252-AC-2 | A contract `when now - start > d then timed_out` over `u32` parameters, with `timestamp_parameters` `{now, start}`, yields a `TimestampContract` obligation; a clause `elapsed_wall_time(handler) <= 50 µs` is disposed `unsupported`, `unsupported-requested-capability`, naming elapsed real time; a clause bounding a function's WCET is disposed the same way, naming WCET. | Test (TC-707) |
| FR-252-AC-3 | The contract of FR-252-AC-2 with `timestamp_parameters` empty yields no `TimestampContract` obligation; the clauses `elapsed_wall_time(handler) <= 50 µs` and `wcet(f) < 2 ms` are timing clauses, and a comparison of two `u32` parameters is not. | Test (TC-707) |

## Dependencies

- ADR-026 §14 KG-1 to KG-4; ADR-014 B-5.
- [FR-251](FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md).

## References

- The CG obligation shapes: ADR-026 OV-13.
- QSpec half: QSpec FR-415 and FR-416 (Linear STD-139), the timed model
  and claim semantics the obligations carry.
