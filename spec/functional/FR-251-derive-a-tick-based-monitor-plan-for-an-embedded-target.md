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
    pub program: MonitorProgram,            // plan-root entries, atoms, topology and State type
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
| Atoms | Each `AtomId` maps to its checked predicate body, ordered input/capture slots with admitted types and declaration identities, and source occurrence. The activation guard is identified separately from formula atoms. No atom is an unresolved name or caller-supplied Boolean callback. An atom's captured values are the scalar or aggregate values the atom reads, typed per atom capture binding (FR-251-AC-5) and projected from the model value by existing catalog projections; whole model objects are not passed as `State`. Where a model-typed atom has no admitted projection, the monitor reports `Incomplete(LimitReached{limit: atom_projection, value: <AtomId>, setting: monitor.atom_projection})` with replayable evidence, not a refusal and not a dropped atom (QSpec FR-487, "Atoms without an admitted projection", FR-487-AC-12). |
| Formula topology | Each `SubformulaId` identifies one node with its checked operator tag, ordered operand IDs, optional atom ID and its interval reference. One root is named. All references resolve inside this program; node IDs are scoped to its subject. Operator tags and operand order survive lowering. |
| State | One typed `State` record whose fields are the persistent state slots, named by subformula: the unwrapped time (an integer tick, kept in state), each buffered subformula's sequence of entries (unwrapped tick and captured bindings), and the active obligation instances with their captured bindings and origin. An obligation's **origin** is the unwrapped tick of the event that activated it (monotone, so time-based windows read it directly); an "on each" activation's position is `State.event_position` read when `on_event` creates the instance and kept as an ordinary capture; there is no `Activation` field and the plan root has no binding for it. The `init` entry's result is the initial value of every slot, so the state initializer is the `init` body. Buffer capacities are premises of the plan root (below), not part of any sequence type; every fold over a buffer is bounded by its capacity. An operator with no finite window (a root `always` or `eventually`, or a future operator nested without a bounding outer window) keeps no history buffer: each live obligation instance carries a constant-size accumulator (its status and origin tick) as a field of `State`, and the number of live instances is bounded by a plan-declared **activation capacity**, the plan root's `activation_capacity` binding (QSpec FR-487, member 16), bounding the `activations` field of `State`. A future operator nested under a bounded outer window is bounded by that window's buffer capacity. A bounded future interval operator keeps its buffer of the Buffers rule even under an unbounded root; the activation capacity bounds live instances of the unbounded operator only, and an overflow reports the limit of whichever capacity was exceeded. `State` has exactly the fields `now`, `history_start` (the unwrapped tick from which buffered history is trusted, initially the baseline), one `buffer_<n>` per buffer, `activations` and `event_position` (QSpec FR-487), the unwrapped monotone count of admitted events (the event position of QSpec FR-091 and FR-092; `init` sets it to 0 and `on_event` adds 1 per admitted event). |
| Transitions | Four **entries**, each a checked-expression `function` node of the checked-package v2 wire (the existing expression and function contracts; no statement vocabulary): `init() -> State`, `on_event(State, event inputs) -> (State, Results)`, `on_watermark(State, watermark inputs) -> (State, Results)` and `on_gap(State, gap) -> (State, Results)`, where `gap` is the checked gap RT passes on a counter fault. An entry is a state-passing total function: "assignment" is constructing the next state record, "branch" is `if`/`case`, "loop over a buffer" is a fold bounded by the buffer's capacity, "push" and "expire" construct and filter the buffer sequence, and "emit" is a field of `Results`. A checked atom or guard body embeds as an operand of the entry's expression. The entries explicitly compute atoms, activate obligations, update state and buffers, expire windows and emit typed obligation results; every read, call target and branch is resolved, and calling an unspecified temporal operator is not an entry body. |
| Entry bindings | Each entry's parameters are its input slots and `State`, and its result is the next `State` and `Results`. An event's inputs are the **gap-checked tick delta** that RT supplies (below), the clause's admitted snapshot/invocation values and the binding identity; a watermark's inputs are the **unwrapped history-boundary tick** (a non-negative integer in the plan's tick unit, covering every tick up to and including the boundary, QSpec FR-487 and FR-094) and the admitted scope identities of QSpec FR-094/FR-160, which RT supplies. Neither entry receives a raw, possibly wrapping counter reading. An ordinary clock poll is not a watermark and creates no event position. |

**Plan root.** The plan crosses E5 as one **plan-root node** of the
checked-package v2 wire: a `temporal`/`monitor_plan` node with seventeen ordered
bindings (QSpec FR-487, FR-322-AC-308): `clause` (the clause the program
monitors, whose `temporal_bindings` row is the plan's clock binding), `state`,
`inputs`, `scope`, `results`, the entries `init`, `on_event` and `on_watermark`,
`counter_width_bits`, `max_reading_gap`, `max_event_rate` (a
rational, events per tick), `buffers` (capacity per buffered subformula),
`counter` (the counter's name), `tick_unit` (a `Time` unit node whose scale is
the tick scale), `uncertainty` (a rational at least 0, in ticks), the entry `on_gap` (member
15) and, last (member 16), `activation_capacity` (an integer at least 0). Each
premise, including the uncertainty, counter, tick unit, gap, rate and
capacities (buffer and activation), is part of the root's key (QSpec FR-487-AC-6), so changing one
gives a different plan. The tick intervals of the logical fields above are
not bindings: each is derived from the clause's interval bounds and the unit
scales (QSpec FR-487). Every entry, atom body and type the root needs is a
dependency of the root, so its content key (QSpec FR-322) selects the
complete immutable plan and no second identity scheme is introduced. The
logical Rust record above is the producer's in-memory view of that node; the
wire node, not a QSL-owned Rust type, is what IR reads and the driver admits.

The retained closure SHALL include the clock binding's counter, exact unit
scale and width, so `ClockBindingKey` requires no ambient lookup at build or
execution. The plan's uncertainty and maximum gap remain available to every
transition that reads time.

The entries SHALL be expressed in the existing checked expression and
`function` contracts of the v2 wire, with exact arithmetic and typed outcomes;
no new statement, loop or assignment vocabulary exists, and a state-passing
function entry satisfies this requirement's executable-body obligation (QSL-685
called these "instruction and control-flow bodies"). A catalog operation needed to extend a buffer
(a sequence append) is not a new operation: QSpec FR-487 expresses it as
`quire.op.collection.flatten` of a sequence of two sequences, so the closed
operation catalog is unchanged. QSL SHALL determine activation, atom meaning, operator updates,
deadline settlement and result propagation before RT build. IR/CG may lower
or generate these bodies, but RT SHALL NOT reconstruct them from the formula
graph, source text, interval table or buffer IDs. A graph with missing bodies
is not a supported executable plan. An operation or predicate that the
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
buffer table. No new persistent identity scheme is introduced.

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
once, preserving its typed result, unless RT's typed `StepResourceStop` stops
the step: a stopped step is not an execution of the body, leaves the state
unchanged and the input unconsumed and re-presentable, and is a typed incomplete
outcome, neither a refusal nor a verdict. Event-rate overflow and counter gaps
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
- **Tick cover.** A reading `k` covers the **half-open** interval `[k, k+1)`
  of the counter unit (this supersedes ADR-026 MN-2's "closed"), widened at
  each end by the declared uncertainty `u` to `[k-u, k+1+u)`.
- **Sound rounding.** Each observed instant SHALL be the interval its tick
  covers, widened by the declared uncertainty (QSpec FR-160). An interval
  bound SHALL decide true or false only when every pair of instants in the
  operands' intervals agrees; otherwise the obligation SHALL stay pending or
  indeterminate as QSpec FR-160 states.
- **Buffers.** For each past-time or bounded-future interval operator
  `[a, b]`, the plan SHALL set the buffer capacity to the maximum event rate
  (events per tick) times the upper bound `b` plus the tick cover (one tick,
  plus `2u`, the widening of QSpec FR-160 at each end of the cover by the declared uncertainty `u`), all in ticks, rounded up, plus
  one, because entries are kept until the end of the window at `b`; the lower
  bound `a` does not shorten it. The rate SHALL be an ADR-014 B-2 run
  limit of the monitor.
- **Overflow.** An event that would exceed a buffer SHALL never be dropped:
  the obligations it affects SHALL settle `Incomplete(LimitReached{limit, value, setting})`
  with `limit` the monitor limit kind `EventBuffer`, written `event_buffer`,
  `value` the buffer capacity (an unsigned count) the rate gave, and `setting`
  the monitor setting `monitor.max_event_rate`, the monitor target member that
  raises it (FR-255's setting table does not list it, as it is an exact
  fraction). `EventBuffer` and `monitor.max_event_rate` extend FR-127's
  `limit` and `setting` fields for a monitor run, which has no model-check
  request.
- **Activation overflow.** A new obligation instance that would exceed the
  activation capacity SHALL never be dropped silently: the obligations it
  affects settle `Incomplete(LimitReached{limit, value, setting})` with
  `limit` `activation`, `value` the activation capacity and `setting`
  `monitor.max_activations` (QSpec FR-487); the instance is not started.
- **Counter width.** If `max_reading_gap` is at least `2^width_bits`, then
  the plan SHALL refuse with `MonitorPlanRefusal::GapExceedsWidth{
  max_reading_gap, width_bits}`, since wraparound could not be decided.
- **Counter faults.** RT owns the modular counter arithmetic (RT FR-276):
  it SHALL compute each tick difference modulo `2^width_bits`. The first
  admitted reading of a monitor is its baseline: RT makes no gap check and
  passes `on_event` the delta 0, so the unwrapped time stays at the value
  `init` set. When a later difference exceeds `max_reading_gap`, the reading
  ran backwards or a reading was missed: RT SHALL call `on_gap` with the
  checked gap (and not `on_event`), SHALL make the faulting reading the new
  baseline, and the monitor SHALL continue. The baseline moves only on admitted readings (an event
  reading, or a gap-faulting reading as above); a counter poll or a watermark
  never moves it. After a gap
  the true elapsed time is unknown (at least the gap), so `on_gap` SHALL drop
  every history buffer and set `history_start` and `now` to the faulting
  reading's unwrapped tick; a past-time operator whose window reaches before
  `history_start` yields `indeterminate` until the window has refilled past it.
  `on_gap` SHALL return
  `Failed` with cause `ReadingGapExceeded{gap, max_reading_gap}`, never a
  verdict, for every live obligation instance whose verdict is still open
  (pending, its window or deadline spanning the gap), and SHALL drop those
  instances from `State`; decided instances and results already emitted, and
  instances not yet started, are unaffected. Otherwise RT passes the
  gap-checked delta to `on_event`; the entry adds it to the unwrapped time it
  keeps in `State` and never sees a raw wrapping reading. The gap path and its
  wire form are QSpec FR-487 "Counter-gap path" and FR-487-AC-7.
- **Same-tick events.** Two events in the same tick have difference interval
  `(-w, w)` where `w` is the widened cover width; the monitor SHALL NOT clamp
  it to 0 and SHALL NOT substitute ingestion order for it (QSpec FR-160-AC-5).
- **Reporting.** Every interval-operator instance is reported once, when it
  settles (`true`, `false`, `indeterminate` or `Failed`), and every
  still-pending instance is reported at each watermark step. A root verdict
  is per activation; an unbounded root `always` or `eventually`
  reports one folded outcome per activation (pending until resolved, `Incomplete(LimitReached)` if the activation overflowed), not one
  per position of the trace.
- **Declarative semantics.** The meaning of every operator is the declarative
  one of QSpec FR-091 and FR-092 (timestamped rule) over the widened tick
  intervals of QSpec FR-160, and is normative; no separate update algorithm
  is. Endpoints: an interval operator with bound `[a, b]` (or open or
  half-open as written) compares `tau_j - tau_i` with the bound over every
  pair of instants in the two widened covers, and decides only when all pairs
  agree, an open endpoint excluding the bound's value and a closed one
  including it. A result is `pending` iff declared progress (the watermark)
  could still resolve the verdict and `indeterminate` when the evidence stays
  conflicting even with all declared progress applied. A buffered entry
  expires when the end of its window falls before the history boundary. Each generated entry SHALL agree with this
  meaning on every trace (FR-251-AC-7).
- QSL's reference evaluator over a tick-stamped trace SHALL give, for each
  obligation, the decision the plan specifies. It is a module of `qsl-eval`
  that evaluates the declarative semantics above directly, with uncertainty,
  three-valued results, counter width and watermarks; it is not the FR-234
  pointwise evaluator, and generated entries are validated against it.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-251-AC-1 | For `always (holds(req) implies eventually[0 ms, 3 ms] holds(ack))` on a 20-bit 1 MHz counter with uncertainty 0: `req` at tick 0 and `ack` at tick 2998 decides true; `ack` first at tick 3001 decides false; `ack` first at tick 3000 stays indeterminate. With uncertainty 2 µs, `ack` at 2998 stays indeterminate. These hold under the half-open cover: with `req` at 0 covering `[0, 1)`, `ack` at 3001 covers `[3001, 3002)` and the difference lies in `(3000, 3002)`, all above the bound 3000, so false; `ack` at 3000 gives `(2999, 3001)`, which straddles 3000, so indeterminate; `ack` at 2998 gives `(2997, 2999)`, all within the bound, so true; with 2 ticks of uncertainty the `ack` at 2998 difference is `(2993, 3003)`, indeterminate. | Test (TC-706) |
| FR-251-AC-2 | On a 1 MHz counter with uncertainty 0, `once[0 ms, 10 ms] holds(p)` with `max_event_rate` 2 events per ms (0.002 per tick) gets buffer capacity ceil(0.002 x (10000 + 1)) + 1 = 22; a trace with 23 events inside one 10 ms window settles the affected obligation `Incomplete(LimitReached{limit, value, setting})` with `limit` `event_buffer`, `value` 22 and `setting` `monitor.max_event_rate`, and drops no event. | Test (TC-706) |
| FR-251-AC-3 | With a 16-bit counter and `max_reading_gap` 100: readings `65530` then `4` give a difference of 10 ticks; readings `65530` then `65500` give a modular difference of 65,506, so RT detects the gap and calls `on_gap` (AC-11); every obligation instance still open across it settles `Failed`, `ReadingGapExceeded{gap: 65506, max_reading_gap: 100}`, with no verdict. A 16-bit target with `max_reading_gap` 65,536 refuses `GapExceedsWidth`. | Test (TC-706) |
| FR-251-AC-4 | AC-1's admitted plan alone builds a monitor after the producer/preparation objects are dropped. Its closed program contains both atom bodies, the ordered implication/eventually/always topology, activation and capture bindings, state initializers and executable event/watermark transitions. No source parsing, clause argument, atom callback or RT temporal translation occurs. Removing a transition body refuses admission with no initialization or monitor. | Test (TC-1081) |
| FR-251-AC-5 | Replace an atom, operand reference, entry body, `init` body, interval association, buffer owner, activation binding, uncertainty, counter, tick unit, event rate or maximum gap independently with one from a different selected clause/target while retaining the original selection: each candidate refuses admission without initialization. Matching numeric subformula IDs from two subjects do not alias. A correctly derived plan for the changed target is admitted as a distinct plan. | Test (TC-1081) |
| FR-251-AC-6 | Build two monitors from the same plan; a request observed only by the first does not activate an obligation in the second. Attempts to rebind either live monitor to another program/target fail without changing either state. Each emitted result retains its selected subject, target premises and obligation origin. | Test (TC-1081) |
| FR-251-AC-7 | Executing the plan's transition entries and the QSL reference evaluator on the same admitted inputs gives identical per-obligation outcomes for AC-1 through AC-3, a clause with reversed implication operands, an activation guard that is false, and open versus closed interval endpoints. A progress assertion past a deadline settles according to QSpec FR-094/FR-160; polling the counter without that assertion adds no event and settles no deadline. | Test (TC-1081) |
| FR-251-AC-8 | An unsupported predicate/instruction yields `Unsupported` and no monitor; failed state allocation yields build failure and no monitor; an event missing a required admitted value is rejected without state change; an executor invariant failure yields `Failed` with no verdict and prevents further input. None of these controls returns `false` as a property result. | Test (TC-1081) |
| FR-251-AC-9 | The admitted plan of AC-1 crosses E5 as one `temporal`/`monitor_plan` plan-root node (QSpec FR-487, FR-322-AC-308) whose dependencies include the entries `init`, `on_event`, `on_watermark` and `on_gap` (member 15 of QSpec FR-487; FR-487-AC-7) as `function` nodes with the signatures of the Transitions row, and the `State` type; no entry uses a statement, loop or assignment form. Changing any one entry body changes the plan-root's content key and so the plan identity. The `init` result is every slot's initial value, and an obligation's origin in `State` is the unwrapped tick of its activating event. | Test (TC-1081) |
| FR-251-AC-10 | The entries never receive a raw counter reading. On the 16-bit target of AC-3, RT supplies `on_event` the delta 10 for readings `65530` then `4` and the entry's unwrapped time advances by 10; for `65530` then `65500` RT calls `on_gap` with the gap 65506 (not `on_event`) and the faulting reading `65500` becomes the new baseline (AC-11). `on_watermark` receives the unwrapped history-boundary tick and the admitted scope identities; a watermark with history boundary tick 3001, the first tick at which a later `ack` would decide false, settles AC-1's `req` obligation (`ack` never seen) false, and a mere counter poll adds no event and calls no entry. | Test (TC-1081) |
| FR-251-AC-11 | On the 16-bit target of AC-3 with `max_reading_gap` 100, the first admitted reading passes `on_event` the delta 0 and leaves the unwrapped time at `init`'s value. With one obligation instance still open across the fault, one decided before it and one started after it: readings `65530`, `65500` make RT call `on_gap` with 65506; the open instance yields `Failed`, `ReadingGapExceeded{gap: 65506, max_reading_gap: 100}` and is dropped from `State`, the decided instance's result and the later instance are unaffected, no verdict is emitted for the dropped instance, and a following reading `65510` is differenced from `65500` (delta 10) and passed to `on_event`. The `on_gap` entry is member 15 of QSpec FR-487's root (FR-487-AC-7); its returned state has `now` advanced by `gap`. History buffers and the `history_start` after the fault are FR-251-AC-15. | Test (TC-1081) |
| FR-251-AC-12 | An `on_event` or `on_watermark` step stopped by RT's typed `StepResourceStop` (the per-step budget supplied at build is exhausted) leaves the monitor `State` unchanged and the event unconsumed; the stop is a typed incomplete outcome, not a refusal, not a verdict, not `Incomplete(LimitReached)`, not `Failed` and not the terminal executor invariant failure (after which the monitor rejects input), and the monitor accepts the next input. Re-presenting the same event under a sufficient budget executes the transition body exactly once and gives the outcome the unstopped step would have given; "Valid inputs execute the transition body once" applies to steps that are not resource-stopped. | Test (TC-1081) |
| FR-251-AC-13 | Under the AC-2 premises, `once[5 ms, 10 ms] holds(p)` also gets capacity 22 (the upper bound 10 ms, not the width 5 ms, which would give 12); 23 events inside the window settle `Incomplete(LimitReached{limit: event_buffer, value: 22, setting: monitor.max_event_rate})`. | Test (TC-706) |
| FR-251-AC-14 | For `always (holds(req) implies eventually[0 ms, 3 ms] holds(ack))` the root `always` keeps no history buffer: `State` holds one constant-size accumulator per live instance, and with an activation capacity of 2, a third live instance is not started and settles `Incomplete(LimitReached{limit: activation, value: 2, setting: monitor.max_activations})`, with no instance dropped silently. The root reports one folded outcome per activation, pending until resolved; each `eventually` instance is reported once on settlement and each pending one at every watermark step. | Test (TC-1081) |
| FR-251-AC-15 | On the 16-bit target of AC-3, after `on_gap` the history buffers are empty and `history_start` and `now` are the faulting reading's unwrapped tick; `once[0 ms, 10 ms] holds(p)` (1 MHz tick, `max_event_rate` 2 events per ms) evaluated before its window has refilled past the marker is `indeterminate`, then decides once refilled. A counter poll between readings leaves the baseline unchanged. | Test (TC-1081) |
| FR-251-AC-16 | Two events in the same 1 MHz tick give the difference interval `(-1, 1)` at uncertainty 0: `a before b` is neither decided true nor decided false, not clamped to 0 and not decided by ingestion order. | Test (TC-706) |
| FR-251-AC-17 | A clause with an "on each" activation over a model-typed value: three admitted events (two in the same tick) create instances whose activation positions are 1, 2 and 3, read from `State.event_position` at creation and carried as captures, with no `Activation` field in the plan; an atom's captures are the typed scalar or aggregate projections it reads, and an atom over a model value with no admitted projection yields `Incomplete(LimitReached{limit: atom_projection, value: <that AtomId>, setting: monitor.atom_projection})` with replayable evidence, not a refusal (QSpec FR-487-AC-12). | Test (TC-1081) |

## Dependencies

- [ADR-026](../decisions/ADR-026-dense-time.md) §14 MN-1 to MN-7; ADR-014 B-2.
- [FR-234](FR-234-check-timed-intervals-and-classify-timed-property-forms.md).
- QSpec FR-090 (timestamped-event profile), QSpec FR-160 (clock
  uncertainty and three-valued obligations), QSpec FR-252 (clock binding).

## References

- The monitor contract handed to the runtime repository: ADR-026 OV-13.
- QSpec half (Linear STD-647, merged): the plan-root node, its seventeen bindings (`on_gap` is member 15, FR-487-AC-7; `activation_capacity` is member 16, FR-487-AC-8) and entry signatures, QSpec FR-487 (FR-487-AC-6 for plan identity) with TC-978 and FR-322-AC-308; the half-open tick cover, QSpec FR-160-AC-6 and FR-160-AC-7.
- Linear QSL-685 fixes this executable handoff for RT's IR-519. The plan
  producer (QSL-587), executable lowering/generation and RT implementation
  remain downstream work; these criteria specify their acceptance and do
  not claim an implemented monitor.
- QSpec half: QSpec FR-416 (Linear STD-139), the timed claim semantics the
  monitor plan evaluates.
- D. Basin, F. Klaedtke and E. Zălinescu, 2011; H.-M. Ho, J. Ouaknine and
  J. Worrell, 2014 (ADR-026 References).
