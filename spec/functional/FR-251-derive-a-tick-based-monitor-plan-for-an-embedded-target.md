---
id: FR-251
title: "Derive a tick-based monitor plan for an embedded target"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-234
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-090
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-160
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-252
    type: depends_on
---
# FR-251: Derive a tick-based monitor plan for an embedded target

## Description

QSL SHALL derive, for a timed claim monitored on an embedded target, a
monitor plan that the runtime repository builds into a monitor (ADR-026
MN-1 to MN-4). The plan reads the claim under the timestamped-event
profile with integer ticks of a monotonic hardware counter, converts each
dense constant to ticks with sound rounding, sizes each buffer from a
caller-set maximum event rate, and fixes how overflow and counter faults
settle. QSL's reference evaluator gives the same three-valued decision over
a tick-stamped trace as the plan specifies.

## Use case

An embedded engineer deploys "every request is answered within 3 ms" to a
microcontroller with a 1 MHz counter and a stated clock uncertainty. The
monitor never turns an uncertain boundary into a verdict, never drops an
event, and uses a buffer computed in advance from the event rate the
engineer states.

## Inputs

- A checked timed clause (FR-234).
- `MonitorTarget { counter: Name, unit: TimeUnit, width_bits: u32,
  uncertainty: ExactRational, max_event_rate: ExactRational }`, the rate in
  events per counter unit; `max_event_rate` defaults to one event per
  microsecond, converted to the counter unit, and the caller may replace it.

## Outputs

```rust
pub struct TimedMonitorPlan {
    pub binding: ClockBindingKey,           // QSpec FR-252: counter, unit, width
    pub intervals: Vec<TickInterval>,       // each bound as ticks with its rounding
    pub buffers: Vec<(SubformulaId, u64)>,  // capacity per buffered subformula
    pub rate_limit: ExactRational,
}
```

## Behavior

- The plan SHALL read the clause under the timestamped-event profile, with
  positions as observed events and time stamps as integer ticks; its clock
  binding SHALL name the counter, its unit and its width (QSpec FR-252).
- **Sound rounding.** Each observed instant SHALL be the interval its tick
  covers, widened by the declared uncertainty (QSpec FR-160). An interval
  bound SHALL decide true or false only when every pair of instants in the
  operands' intervals agrees; otherwise the obligation SHALL stay pending or
  indeterminate as QSpec FR-160 states.
- **Buffers.** For each past-time or bounded-future interval operator, the
  plan SHALL set the buffer capacity to the maximum event rate times the
  window length, rounded up, plus one. The rate SHALL be an ADR-014 B-2 run
  limit of the monitor.
- **Overflow.** An event that would exceed a buffer SHALL never be dropped:
  the obligations it affects SHALL settle `Incomplete(ResourceExhausted)`
  naming the event-rate limit and its value.
- **Counter faults.** The plan SHALL compute tick differences modulo
  `2^width_bits`. A reading that runs backwards, or a gap between
  consecutive readings the width cannot disambiguate, SHALL be a monitor
  fault, `Failed`, and never a verdict.
- QSL's reference evaluator over a tick-stamped trace SHALL give, for each
  obligation, the decision the plan specifies.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-251-AC-1 | For `always (holds(req) implies eventually[0 ms, 3 ms] holds(ack))` on a 20-bit 1 MHz counter with uncertainty 0: `req` at tick 0 and `ack` at tick 2998 decides true; `ack` first at tick 3001 decides false; `ack` first at tick 3000 stays indeterminate. With uncertainty 2 µs, `ack` at 2998 stays indeterminate. | Test (TC-706) |
| FR-251-AC-2 | `once[0 ms, 10 ms] holds(p)` with `max_event_rate` 2 events per ms gets a buffer capacity of 21; a trace with 22 events inside one 10 ms window settles the affected obligation `Incomplete(ResourceExhausted)` naming the event-rate limit and 2 per ms, and drops no event. | Test (TC-706) |
| FR-251-AC-3 | With a 16-bit counter, readings `65530` then `4` give a difference of 10 ticks; a reading lower than its predecessor with no wraparound possible under the rate limit is `Failed`, with no verdict. | Test (TC-706) |

## Dependencies

- ADR-026 §14 MN-1 to MN-4; ADR-014 B-2.
- [FR-234](FR-234-check-timed-intervals-and-classify-timed-property-forms.md).
- QSpec FR-090 (timestamped-event profile), QSpec FR-160 (clock
  uncertainty and three-valued obligations), QSpec FR-252 (clock binding).

## References

- The monitor contract handed to the runtime repository: ADR-026 OV-13.
- QSpec half: QSpec FR-416 (Linear STD-139), the timed claim semantics the
  monitor plan evaluates.
- D. Basin, F. Klaedtke and E. Zălinescu, 2011; H.-M. Ho, J. Ouaknine and
  J. Worrell, 2014 (ADR-026 References).
