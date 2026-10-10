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
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-029
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
  uncertainty: ExactRational, max_event_rate: ExactRational,
  max_reading_gap: u64 }`: the counter's width in bits; the rate in events
  per counter unit, which defaults to one event per microsecond, converted
  to the counter unit, and the caller may replace; and the required maximum
  gap in ticks between consecutive readings, a premise of the target that
  the caller states.

## Outputs

```rust
pub struct TimedMonitorPlan {
    pub subject: MonitorSubject,            // exact checked package and clause occurrence
    pub binding: ClockBindingKey,           // QSpec FR-252: counter, unit, width
    pub uncertainty: ExactRational,
    pub max_reading_gap: u64,
    pub program: MonitorProgram,            // closed executable topology and transitions
    pub intervals: Vec<(SubformulaId, TickInterval)>,
    pub buffers: Vec<(SubformulaId, u64)>,  // capacity per buffered subformula
    pub rate_limit: ExactRational,
}
```

This is the logical contract, not a new Rust dependency from RT to QSL or
a separate serialization format. Its checked payload crosses
[ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
E5 through the checked-package wire and its verified binding; IR owns S5
lowering, and the driver with CG/RT owns generation under
[ADR-029](../decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md)
OP-3 and EB-6. QSL owns the program's temporal meaning. Transport and
lowering SHALL preserve the logical fields below.

### Executable program and ownership

The chosen contract is **a plan carrying executable topology**. A successful
plan SHALL contain the following closed program, including every reachable
dependency; intervals and buffer capacities alone are not an executable plan.

| Program component | Required content |
| --- | --- |
| Subject | `MonitorSubject`: exact checked package identity, authored clause occurrence, checked formula root, activation and capture bindings, and timestamped-event profile. Existing checked identities are retained; a clause display name is not a selection. |
| Atoms | Each `AtomId` maps to its checked predicate body, ordered input/capture slots with admitted types and declaration identities, and source occurrence. The activation guard is identified separately from formula atoms. No atom is an unresolved name or caller-supplied Boolean callback. |
| Formula topology | Each `SubformulaId` identifies one node with its checked operator tag, ordered operand IDs, optional atom ID and its interval reference. One root is named. All references resolve inside this program; node IDs are scoped to its subject. Operator tags and operand order survive lowering. |
| State | Typed persistent state slots, initial values and buffer ownership by subformula, including active obligation instances, their captured bindings and their origin positions. All capacities come from this plan. |
| Transitions | Complete target-neutral executable instruction/control-flow bodies for initialization, admitting an observed event, and advancing an admitted watermark. Bodies explicitly compute atoms, activate obligations, update state/buffers, expire windows and emit typed obligation results. Every read, write, call target, branch and return is resolved; calling an unspecified temporal operator is not an instruction body. |
| Entry bindings | Each transition entry names its input slots, state slots and result slots. Event inputs hold an integer counter reading and the clause's admitted snapshot/invocation values; watermark inputs hold an admitted progress assertion under QSpec FR-094/FR-160. An ordinary clock poll is not a watermark and creates no event position. |

The retained closure SHALL include the clock binding's counter, exact unit
scale and width, so `ClockBindingKey` requires no ambient lookup at build or
execution. The plan's uncertainty and maximum gap remain available to every
transition that reads time.

Instruction bodies SHALL express the transitions through the existing checked
expression and S5 execution contracts, with exact arithmetic and typed
outcomes. QSL SHALL determine activation, atom meaning, operator updates,
deadline settlement and result propagation before RT build. IR/CG may lower
or generate these bodies, but RT SHALL NOT reconstruct them from the formula
graph, source text, interval table or buffer IDs. A graph with missing bodies
is not a supported executable plan. An instruction or predicate that the
selected backend cannot execute SHALL receive the existing per-item
`Unsupported` disposition before build, with no partial monitor.

### Build inputs and identity

`build(plan) -> Result<Monitor, MonitorBuildFailure>` takes one admitted,
immutable executable plan in the RT representation supplied by the driver.
The driver's admission SHALL bind that representation to the independently
selected checked package/clause and all target fields above. Admission SHALL
require the complete program derived from that exact checked selection and
target, including bodies and state initializers; structural validity alone
grants no execution authority. Raw bytes or a
caller-authored record cannot assert admission. The caller SHALL NOT supply
a second checked clause, formula, transition function, replacement clock
binding or atom evaluator to complete the plan.

Build SHALL allocate fresh state and run the plan's initialization entry
without consuming an event or producing an obligation verdict. The monitor
SHALL own or retain immutable ownership of the complete plan for its lifetime;
dropping the driver's preparation objects SHALL NOT invalidate it. Two builds
of the same plan SHALL have independent mutable state. Hardware acquisition
and conversion of observed values into admitted inputs belong to the caller;
they do not grant the caller authority to change predicate meaning.

The monitor SHALL retain the exact subject, clock binding, uncertainty,
maximum reading gap and rate limit with each result's obligation instance,
origin position and subformula identity. `SubformulaId` and `AtomId` from
another subject SHALL NOT resolve locally, even if their numeric indices
match. Changing any checked subject, executable body, activation, target
premise or capacity requires a newly admitted plan and a fresh monitor;
live state SHALL NOT be rebound. [FR-252](FR-252-hand-timing-obligations-to-code-generation.md)'s
`PlanId` identifies this complete immutable plan in the driver's ownership
scope, including its executable program and target premises, never just its
buffer table. No new content digest or persistent identity scheme is introduced.

### Fault boundary

QSL SHALL refuse plan derivation if the checked clause/target cannot yield
the complete program; that refusal creates neither an RT monitor nor a
verdict. An invalid package/clause selection, unresolved or foreign program
reference, inconsistent interval/buffer association or altered target field
SHALL fail driver/IR admission before build as an invalid plan, without
executing initialization. A backend capability refusal remains `Unsupported`.
RT resource failure during allocation or initialization SHALL return a build
failure and expose no usable partial monitor. These are build/admission
results, not truth values or observed property violations.

After build, RT SHALL validate event/progress inputs against the plan's entry
bindings before applying the transition. Invalid inputs SHALL cause no state
change or verdict. Valid inputs SHALL execute the supplied transition body
once, preserving its typed result. Event-rate overflow and counter gaps
retain the `Incomplete` and `Failed` settlements below. An executor invariant
failure SHALL be `Failed`, never `false`, pending or a successful verdict;
after such a failure the monitor SHALL accept no further input. RT owns
execution faults and input admission; a disagreement between the prescribed
program and the QSL reference evaluator is a QSL plan-generation defect,
while incorrect execution of that program is an RT/CG defect. Neither owner
may repair such a disagreement by inventing another evaluation rule.

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
  the obligations it affects SHALL settle `Incomplete(LimitReached{limit, value, setting})`
  naming the event-rate limit and its value.
- **Counter width.** If `max_reading_gap` is at least `2^width_bits`, then
  the plan SHALL refuse with `MonitorPlanRefusal::GapExceedsWidth{
  max_reading_gap, width_bits}`, since wraparound could not be decided.
- **Counter faults.** The monitor SHALL compute each tick difference
  modulo `2^width_bits`. If that difference exceeds `max_reading_gap`, the
  reading ran backwards or a reading was missed, and the affected
  obligations SHALL settle `Failed` with cause `ReadingGapExceeded{gap,
  max_reading_gap}`, never a verdict.
- QSL's reference evaluator over a tick-stamped trace SHALL give, for each
  obligation, the decision the plan specifies.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-251-AC-1 | For `always (holds(req) implies eventually[0 ms, 3 ms] holds(ack))` on a 20-bit 1 MHz counter with uncertainty 0: `req` at tick 0 and `ack` at tick 2998 decides true; `ack` first at tick 3001 decides false; `ack` first at tick 3000 stays indeterminate. With uncertainty 2 µs, `ack` at 2998 stays indeterminate. | Test |
| FR-251-AC-2 | `once[0 ms, 10 ms] holds(p)` with `max_event_rate` 2 events per ms gets a buffer capacity of 21; a trace with 22 events inside one 10 ms window settles the affected obligation `Incomplete(LimitReached{limit, value, setting})` naming the event-rate limit and 2 per ms, and drops no event. | Test |
| FR-251-AC-3 | With a 16-bit counter and `max_reading_gap` 100: readings `65530` then `4` give a difference of 10 ticks; readings `65530` then `65500` give a modular difference of 65,506 and settle `Failed`, `ReadingGapExceeded{gap: 65506, max_reading_gap: 100}`, with no verdict. A 16-bit target with `max_reading_gap` 65,536 refuses `GapExceedsWidth`. | Test |
| FR-251-AC-4 | AC-1's admitted plan alone builds a monitor after the producer/preparation objects are dropped. Its closed program contains both atom bodies, the ordered implication/eventually/always topology, activation and capture bindings, state initializers and executable event/watermark transitions. No source parsing, clause argument, atom callback or RT temporal translation occurs. Removing a transition body refuses admission with no initialization or monitor. | Test |
| FR-251-AC-5 | Replace an atom, operand reference, transition body, state initializer, interval association, buffer owner, activation binding, uncertainty or maximum gap independently with one from a different selected clause/target while retaining the original selection: each candidate refuses admission without initialization. Matching numeric subformula IDs from two subjects do not alias. A correctly derived plan for the changed target is admitted as a distinct plan. | Test |
| FR-251-AC-6 | Build two monitors from the same plan; a request observed only by the first does not activate an obligation in the second. Attempts to rebind either live monitor to another program/target fail without changing either state. Each emitted result retains its selected subject, target premises and obligation origin. | Test |
| FR-251-AC-7 | Executing the plan's transition entries and the QSL reference evaluator on the same admitted inputs gives identical per-obligation outcomes for AC-1 through AC-3, a clause with reversed implication operands, an activation guard that is false, and open versus closed interval endpoints. A progress assertion past a deadline settles according to QSpec FR-094/FR-160; polling the counter without that assertion adds no event and settles no deadline. | Test |
| FR-251-AC-8 | An unsupported predicate/instruction yields `Unsupported` and no monitor; failed state allocation yields build failure and no monitor; an event missing a required admitted value is rejected without state change; an executor invariant failure yields `Failed` with no verdict and prevents further input. None of these controls returns `false` as a property result. | Test |

## Dependencies

- [ADR-026](../decisions/ADR-026-dense-time.md) §14 MN-1 to MN-7; ADR-014 B-2.
- [FR-234](FR-234-check-timed-intervals-and-classify-timed-property-forms.md).
- QSpec FR-090 (timestamped-event profile), QSpec FR-160 (clock
  uncertainty and three-valued obligations), QSpec FR-252 (clock binding).

## References

- The monitor contract handed to the runtime repository: ADR-026 OV-13.
- Linear QSL-685 fixes this executable handoff for RT's IR-519. The plan
  producer (QSL-587), executable lowering/generation and RT implementation
  remain downstream work; these criteria specify their acceptance and do
  not claim an implemented monitor.
- QSpec half: QSpec FR-416 (Linear STD-139), the timed claim semantics the
  monitor plan evaluates.
- D. Basin, F. Klaedtke and E. Zălinescu, 2011; H.-M. Ho, J. Ouaknine and
  J. Worrell, 2014 (ADR-026 References).
