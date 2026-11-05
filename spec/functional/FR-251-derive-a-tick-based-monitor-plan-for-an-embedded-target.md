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
| State | One typed `State` record whose fields are the persistent state slots, named by subformula: the unwrapped time (an integer tick, kept in state), each buffered subformula's sequence of entries (unwrapped tick and captured bindings), and the active obligation instances with their captured bindings and origin. An obligation's **origin** is the unwrapped tick of the event that activated it (monotone, so time-based windows read it directly); an "on each" activation's position is `State.event_position` read when `on_event` creates the instance and kept as an ordinary capture, which travels as the integer field `position` of the instance's `status` record in `activations` (QSpec FR-487 imposes no type on `status`); there is no `Activation` field and the plan root has no binding for it. The `init` entry's result is the initial value of every slot, so the state initializer is the `init` body. Buffer capacities are premises of the plan root (below), not part of any sequence type; every fold over a buffer is bounded by its capacity, and the `buffer_<n>` of an unbounded past operator also holds one summary cell as its first element (an accumulator and its own `history_start`, not an entry and not counted in the capacity, Behavior "Unbounded past operators"). An operator with no finite window (a root `always` or `eventually`, or a future operator nested without a bounding outer window) keeps no history buffer: each live obligation instance carries a constant-size accumulator (its status and origin tick) as a field of `State`, and the number of live instances is bounded by a plan-declared **activation capacity**, the plan root's `activation_capacity` binding (QSpec FR-487, member 16), bounding the `activations` field of `State`. A bounded future operator nested under a bounded outer window is bounded by its own buffer capacity (the retention rule of Buffers); an unbounded future operator nested there is bounded by the activation capacity. A bounded future interval operator keeps its buffer of the Buffers rule even under an unbounded root; the activation capacity bounds live instances of unbounded operators only (a pending instance of an unbounded operator nested under a bounded one counts against the same shared activation capacity), and an overflow reports the limit of whichever capacity was exceeded. `State` has exactly the fields `now`, `history_start` (the unwrapped tick from which buffered history is trusted, initially the baseline), one `buffer_<n>` per buffer, `activations` and `event_position` (QSpec FR-487), the unwrapped monotone count of admitted events (the event position of QSpec FR-091 and FR-092; `init` sets it to 0 and `on_event` adds 1 per admitted event). |
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
dependency of the root, and the root's key also covers the body roots of the
declared helper functions its entries, atoms and guard call (the call closure of
QSpec FR-487; a helper stays a declared node keyed by its name for call
identity), so its content key (QSpec FR-322) selects the
complete immutable plan and no second identity scheme is introduced. The
logical Rust record above is the producer's in-memory view of that node; the
wire node, not a QSL-owned Rust type, is what IR reads and the driver admits.

The `Emission` that carries the plan-root bytes SHALL carry, from the same
acquisition the package was emitted from, every witness row the emitter
retains for the package: the selected model declaration and field rows, the
source-unit rows, the model enum and source enum declaration and member rows,
the source union rows, the source record rows, the tuple declaration rows, the operation-member rows, the equality producer and fact
rows, the temporal binding rows and the omitted nodes. The plan-root writer
SHALL re-key every node id each of those rows holds through the renames it
performs and SHALL NOT write a row anew, so a consumer authenticates the
selected generated closure against the rows of the acquisition and never
invents them or acquires the unit a second time.

The plan's subject names the clause it monitors by the clause's checked node
identity and its claim occurrence. The selection route is the `Emission`'s
temporal binding rows: `declaration(subject.clause_node(),
subject.occurrence().origin().ordinal())` returns the emitted declaration. The
lookup is re-keyed with the retained rows through every rename the writer
performs or may perform. The `Emission` that carries the plan-root bytes SHALL
select, through that route, the clause node and claim occurrence the emitted
package holds: the declaration identity its temporal binding rows
return for the subject's clause node and claim ordinal equals the plan root's
`clause` binding, and equals the `declaration` of the package's
`temporal_clause_names` and `temporal_bindings` rows for that ordinal, after
every rename the writer performs. A subject whose clause node and claim
ordinal the emitted package does not hold selects nothing; it never selects
another occurrence of the clause or another clause. A unit with several
temporal clauses, or a clause with several claim occurrences, selects each
subject's own occurrence.

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
buffer table. It covers the checked body of every atom and of the activation
guard, every entry body, and the body of every declared helper function that an
entry, an atom or the guard calls, directly or through another helper (the call
closure of QSpec FR-487-AC-13), so replacing any one of those bodies, including
through a public plan constructor, gives a different `PlanId`; a name, a
declaration path or a node index alone is not a body. No new persistent identity
scheme is introduced.

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
- **Buffers.** For each past-time or bounded-future interval operator `n`
  with upper bound `b`, the plan SHALL set the buffer capacity to
  `ceil(rate x R(n)) + 1`, where `rate` is the maximum event rate in events per
  tick and `R(n)`, the **retention** in ticks, is

  `R(n) = L(n) + b + (d(n) + 1) x (1 + 2u)`

  - `b` is the operator's own upper bound (the lower bound `a` does not
    shorten it, because entries are kept until the end of the window at `b`);
  - `L(n)` is the **operand-lookahead term**: the sum of the upper bounds of the
    interval operators that enclose `n`, outermost to innermost, each of which
    consults `n`'s entries over a window of its own upper bound, so an entry
    of `n` must survive until every enclosing operator can still consult it
    (`L` is 0 for an operator with no enclosing interval operator);
  - `d(n)` is the number of enclosing interval operators, and `1 + 2u` is one
    **tick cover** (one tick plus `2u`, the widening of QSpec FR-160 at each
    end of the cover by the declared uncertainty `u`), counted once for `n` and
    once for each enclosing operator.

  A single operator is the case `L = 0`, `d = 0`, giving
  `ceil(rate x (b + 1 + 2u)) + 1`. The rate SHALL be an ADR-014 B-2 run limit of
  the monitor.
- **Unbounded past operators.** `once`, `historically`, `since` and `triggered`
  written without an interval (`[0, *]`), or with a lower bound and no upper bound
  (`[a, *]`, `a` > 0), are decided with **sound but incomplete** semantics:
  never a wrong definite verdict, a typed unknown outcome where finite memory
  cannot decide, and never a refusal. Plan derivation SHALL NOT refuse these
  clauses, nor an unbounded operator over an unbounded one, nor one over a
  future operator; no operator is typed "unsupported" for want of an interval.
  - **Lower boundary.** The monitor's lower-boundary kind is FR-501's
    `HistoryCutoff`, which is what `history_start`, initially the baseline,
    already means for a bounded past operator: the monitor starts at a counter
    baseline and does not observe the system's origin, so `AuthoritativeOrigin`
    is not selectable for a monitor, and a reference-evaluator trace given
    `AuthoritativeOrigin` is outside the AC-7 differential for an operator that
    reaches before the baseline. An unbounded past operator therefore always
    reaches before `history_start`, and its result is the Kleene evaluation over
    the history present with an unknown prefix: a witness decides, and
    otherwise the result is `indeterminate` with reason `MissingHistory`. The
    prefix before the baseline, and the prefix lost by a counter gap or by an
    overflow below, enter as the one value `indeterminate`.
  - **Oracle of the differential.** For these operators the oracle of the AC-7
    differential is this requirement's reference evaluator (the `qsl-eval`
    module of "Declarative semantics") evaluating FR-092's declarative meaning
    over the tick trace extended before the baseline by an unknown prefix, every
    atom value `indeterminate` (the `HistoryCutoff` meaning, equal to the
    accumulator that starts `indeterminate`). It is not QSpec FR-092's
    bounded-profile evaluator, which refuses an unbounded past interval, and not
    the infinite-trace evaluator (QSpec FR-367), whose position 0 is the
    authoritative origin and reads atoms before it as false (FR-092-AC-8); QSpec
    FR-092 states that the monitor is a separate evaluator. The two are related by
    refinement, which the differential also checks: wherever the monitor's verdict
    is definite it equals the infinite-trace verdict, and an `indeterminate`
    verdict is compared with nothing (AC-41).
  - **Buffer.** The operator keeps a **cover-width buffer**. Its capacity is
    `ceil(rate x R(n)) + 1` with the retention
    `R(n) = L(n) + T(n) + (d(n) + 1) x (1 + 2u)`, where `T(n)` is `a` for
    `[a, *]` (0 for no interval) plus `S(n)`, the **settling delay** of the
    operand: the largest, over the paths through the operand, of the sum of the
    upper bounds of the future operators on the path (0 for an atom or a past
    operand). `L(n)` is the lookahead term of Buffers with one difference: an
    enclosing unbounded past operator adds 0 to `L` for its operand, because it
    consults an operand value only once that value has settled (below), so a
    bounded operator under an unbounded past one has `L` from its bounded
    enclosing operators only, and `d` still counts the unbounded one. An
    unnested `once` over an atom has capacity `ceil(rate x (1 + 2u)) + 1`.
  - **Summary cell.** The `buffer_<n>` sequence of such an operator has one
    **summary cell**, its first element, holding the folded accumulator and the
    cell's own `history_start` (the tick from which the accumulator is
    trusted). The fields of `State` do not change. The cell is not counted in the
    capacity: the sequence holds at most the capacity of entries and the cell, and
    the formula's `+ 1` is separate slack, so the cell never overflows the buffer.
  - **Folding.** An entry **ages out** and is folded into the cell, in tick
    order and as a prefix, when (i) its age (the newest admitted reading's tick
    minus the entry's tick) is at least `T(n) + L(n) + (d(n) + 1) x (1 + 2u)`,
    so its widened cover lies wholly before the newest reading's and no enclosing
    operator can still consult it, and (ii) its operand value has **settled**
    (below). The fold is the Kleene recursion of FR-092's table at `a` = 0:
    `once`: `A := A or v`; `historically`: `A := A and v`;
    `p since q`: `A := q or (p and A)`; `p triggered q`: `A := q and (p or A)`,
    with `v`, `p` and `q` the operand values of the folded entry. The initial and
    the reset `A` is `indeterminate`. One folded value is sufficient state: for
    `a` = 0 and no upper bound, FR-092 gives `p since q` at `i` as some `d` >= 0
    with `q` at `i - d` and `p` at every `i - e`, `e` in `[0, d)`; splitting
    `d = 0` from `d >= 1` gives `S_i = q_i or (p_i and S_(i-1))` with `S_(i-1)`
    the same operator at the previous position, which holds in three-valued
    logic because `and` distributes over `or` there and quantifiers are lattice
    meets and joins; `triggered` is `not (not p since not q)`, which gives
    `T_i = q_i and (p_i or T_(i-1))`; `once` and `historically` are the cases
    `p` true and `q` false. For `[a, *]`, `a` > 0, the operator at `i` is the
    `[0, *]` operator at `i - a` (the witness at `i - d`, `d >= a`, and `p` over
    `[a, d)`, shift by `a`), so an entry feeds the fold only once its age is at
    least `a` plus the above, which is why `a` is in `T(n)`. An entry that has
    aged out is ordered definitely before every instant the operator is later
    evaluated at, so no ordering is lost by folding it.
  - **Settled operand.** A folded entry's operand value must have settled. A
    past or atomic operand settles at its own position. A bounded future operand
    (`historically (req implies eventually[0 ms, 5 ms] ack)`) settles when its
    window is closed by progress, `S(n)` ticks later: the entry stays in the
    buffer, is not folded early, and the result is `Pending` where the unsettled
    entry could change it and definite where it could not (a witness already
    decides `once`). An unbounded future operand holds one `activations` element
    per live instance, counted against the shared activation capacity, and the
    entry is folded when its instance settles; while it does not, the entry stays
    in the buffer, so on an open trace the buffer can overflow, which is the
    overflow rule below, never a silent loss, and the operator is `Pending`.
  - **Overflow is sticky.** An event that would exceed the buffer of an
    unbounded past operator settles the affected obligations
    `Incomplete(LimitReached{limit: event_buffer, value: <capacity>,
    setting: monitor.max_event_rate})` as in "Overflow", and the loss does not
    end there: the event's contribution to the operator is unknown from then on.
    Exactly as `on_gap` does for every buffer, the operator's buffer is emptied,
    its summary cell is reset to the accumulator `indeterminate` and the cell's own
    `history_start` is the overflowing event's tick. Every later result of the
    operator is then the Kleene evaluation over events after that tick with the
    `indeterminate` prefix, so `once` can be `true` from a later witness and
    never `false`, `historically` can be `false` from a later counterexample and
    never `true`, and `since` and `triggered` follow the recursion: no wrong
    definite verdict follows from the lost event. The buffer content discarded
    with it is reported by the `Incomplete` result and by the cell's
    `history_start`; no event is lost silently.
  - **After a gap.** `on_gap` empties the buffer and resets the summary cell as
    above with `history_start` the faulting reading's tick, for `since` and
    `triggered` as for `once` and `historically`: the accumulator is
    `indeterminate`, so `p since q` is `true` only after a later `q` followed by
    `p` at every position since, `false` as soon as a position has neither `q`
    nor `p` after the gap, and otherwise `indeterminate`; `triggered` is the dual.
  - **Replayable evidence.** Each result retains the subformula, origin, the
    cell's `history_start` and, for overflow, `value` and `setting`, as every
    result retains its subject and premises, so a replay of the same admitted
    inputs gives the same outcome.
  - **Unbounded over unbounded.** `always eventually p`, `eventually always p`
    and any unbounded future operator whose operand holds an unbounded future
    operator keep no history buffer: each live instance of the inner operator
    holds its constant-size accumulator in `activations` and counts against the
    one shared activation capacity (the same rule as an unbounded operator nested
    under a bounded one). The outer operator folds the settled results of its
    inner instances into one outcome per activation; an inner instance beyond the
    capacity is not started and settles `Incomplete(LimitReached{limit:
    activation, value: <capacity>, setting: monitor.max_activations})`, never
    silently dropped. On an open trace such an operator stays `Pending` until a
    witness decides it and is never decided `true` by the absence of a
    counterexample alone. An unbounded past operator over an unbounded future
    one is the "Settled operand" case above.
- **Overflow.** An event that would exceed a buffer SHALL never be dropped:
  the obligations it affects SHALL settle `Incomplete(LimitReached{limit, value, setting})`
  with `limit` the monitor limit kind `EventBuffer`, written `event_buffer`,
  `value` the buffer capacity (an unsigned count) the rate gave, and `setting`
  the monitor setting `monitor.max_event_rate`, the monitor target member that
  raises it (FR-255's setting table does not list it, as it is an exact
  fraction). `EventBuffer` and `monitor.max_event_rate` extend FR-127's
  `limit` and `setting` fields for a monitor run, which has no model-check
  request. **Overflow raises `history_start`.** An overflow of any buffer, bounded or
  unbounded, also raises `State.history_start` to `T + 1 + 2u`, where `T` is the tick of
  the overflowing event and `1 + 2u` is one tick cover, with the existing `State`
  fields only: the lost event covers `[T - u, T + 1 + u)`, so a window whose
  widened start `now - b - u` is below `T + 1 + u` still holds it. The lost event
  is not in any buffer, so every past-time operator whose window reaches before
  `history_start` (`now - b` below `T + 1 + 2u`) is `indeterminate`, never a
  definite verdict, and it is decided again only when `now - b` is at least
  `T + 1 + 2u`, the first reading at which the widened window no longer holds the
  lost event. A bounded operator's older entries stay in its buffer, and a
  bounded `once` can never settle `false`, nor a `historically` `true`, over a
  window that holds the lost event. The overflow of an unbounded past operator is
  also sticky for that operator ("Unbounded past operators").
- **Activation overflow.** A new obligation instance that would exceed the
  activation capacity SHALL never be dropped silently: the obligations it
  affects settle `Incomplete(LimitReached{limit, value, setting})` with
  `limit` `activation`, `value` the activation capacity and `setting`
  `monitor.max_activations` (QSpec FR-487); the instance is not started.
- **Simultaneous overflow.** When several operators (interval operators with a
  buffer, or unbounded operators at the activation capacity) overflow at one
  position, the entry SHALL report the overflowed operator with the lowest
  `subformula` id first and the remaining overflowed operators in ascending
  `subformula` id order, each as its own `Incomplete` result at that position
  with its own capacity as `value` (several instances of one operator that
  overflow the activation capacity together are ordered by ascending `origin`); none is dropped. The order is fixed by the
  ids alone, never by evaluation order.
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
| FR-251-AC-2 | On a 1 MHz counter with uncertainty 0, `once[0 ms, 10 ms] holds(p)` with `max_event_rate` 2 events per ms (0.002 per tick) gets buffer capacity ceil(0.002 x (10000 + 1)) + 1 = 21 + 1 = 22, so 22 entries fit and the 23rd event inside one 10 ms window is the first that does not; a trace with 23 events inside one 10 ms window settles the affected obligation `Incomplete(LimitReached{limit, value, setting})` with `limit` `event_buffer`, `value` 22 and `setting` `monitor.max_event_rate`, and drops no event. | Test (TC-706) |
| FR-251-AC-3 | With a 16-bit counter and `max_reading_gap` 100: readings `65530` then `4` give a difference of 10 ticks; readings `65530` then `65500` give a modular difference of 65,506, so RT detects the gap and calls `on_gap` (AC-11); every obligation instance still open across it settles `Failed`, `ReadingGapExceeded{gap: 65506, max_reading_gap: 100}`, with no verdict. A 16-bit target with `max_reading_gap` 65,536 refuses `GapExceedsWidth`. | Test (TC-706) |
| FR-251-AC-4 | AC-1's admitted plan alone builds a monitor after the producer/preparation objects are dropped. Its closed program contains both atom bodies, the ordered implication/eventually/always topology, activation and capture bindings, state initializers and executable event/watermark transitions. No source parsing, clause argument, atom callback or RT temporal translation occurs. Removing a transition body refuses admission with no initialization or monitor. | Test (TC-1081) |
| FR-251-AC-5 | Replace an atom, the activation guard body, a helper called by an entry, operand reference, entry body, `init` body, interval association, buffer owner, activation binding, uncertainty, counter, tick unit, event rate or maximum gap independently with one from a different selected clause/target while retaining the original selection: each candidate refuses admission without initialization. Matching numeric subformula IDs from two subjects do not alias. A correctly derived plan for the changed target is admitted as a distinct plan. | Test (TC-1081) |
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
| FR-251-AC-17 | A clause with an "on each" activation over a model-typed value: three admitted events (two in the same tick) create instances whose activation positions are 1, 2 and 3, read from `State.event_position` at creation and carried as `status.position` of each `activations` record, with no `Activation` field in the plan; an atom's captures are the typed scalar or aggregate projections it reads, and an atom over a model value with no admitted projection yields `Incomplete(LimitReached{limit: atom_projection, value: <that AtomId>, setting: monitor.atom_projection})` with replayable evidence, not a refusal (QSpec FR-487-AC-12). | Test (TC-1081) |
| FR-251-AC-18 | On a 1 MHz counter with uncertainty 0 and `max_event_rate` 2 events per ms (0.002 per tick), `once[0 ms, 10 ms] once[0 ms, 5 ms] holds(p)` gets capacity 22 for the outer operator (ceil(0.002 x (10000 + 1)) + 1) and 32 for the inner one: retention 10000 + 5000 ticks plus two covers = 15002 ticks, ceil(0.002 x 15002) + 1 = 31 + 1 = 32, not the 12 that its own bound alone would give. A 33rd entry for the inner operator settles `Incomplete(LimitReached{limit: event_buffer, value: 32, setting: monitor.max_event_rate})`. | Test (TC-706) |
| FR-251-AC-19 | For `once[0 ms, 10 ms] eventually holds(ack)` (the `eventually` unbounded, nested under a bounded `once`) with `activation_capacity` 2, when `ack` has not occurred and events at ticks 0, 1000 and 2000 (a 1 MHz counter) each start an `eventually` instance under the `once`, the third start finds two live instances, is not started and settles `Incomplete(LimitReached{limit: activation, value: 2, setting: monitor.max_activations})`, counted against the same capacity as an unbounded root's instances, with no new plan-root binding. | Test (TC-1081) |
| FR-251-AC-20 | Two sibling operators `once[0 ms, 10 ms] holds(p)` (lower `subformula` id) and `once[0 ms, 10 ms] holds(q)`, each capacity 22 under the AC-2 premises, both overflow at the 23rd event: that position yields two `Incomplete(LimitReached{limit: event_buffer, value: 22, setting: monitor.max_event_rate})` results, the lower-id operator's first, each with its own `instance.subformula`, and neither is dropped; the order is the same under any evaluation order of the entry. | Test (TC-706) |
| FR-251-AC-21 | The retention rule holds for mixed-direction nesting (sum of upper bounds plus one cover per level; no direction-specific tightening). On a 1 MHz counter, uncertainty 0, rate 2 events per ms (0.002 per tick): `once[0 ms, 10 ms] eventually[0 ms, 5 ms] holds(p)` and `eventually[0 ms, 5 ms] once[0 ms, 10 ms] holds(p)` each give the inner operator retention 10000 + 5000 + 2 covers = 15002 ticks and capacity ceil(0.002 x 15002) + 1 = 32; the outer operator of the first is the `once` with capacity 22, of the second the `eventually` with capacity ceil(0.002 x 5001) + 1 = 12. With `p` at tick 0 and 30 events at the rate across ticks 0 to 14999, the inner buffer still holds the tick-0 entry at tick 14999 and every operator reads it for the decision that needs it, no `Incomplete` is produced, and the tick-0 entry expires only after tick 15002. | Test (TC-1081) |
| FR-251-AC-22 | On a 1 MHz counter with uncertainty 0 and rate 2 events per ms (0.002 per tick), `once holds(p)` (no interval) is planned, not refused, with capacity ceil(0.002 x (0 + 0 + 1 x (1 + 0))) + 1 = 1 + 1 = 2, and with uncertainty 2 ticks capacity ceil(0.002 x 5) + 1 = 2; its `buffer_<n>` holds at most 2 entries and the summary cell (3 elements), the cell not counted in the capacity. | Test (TC-706) |
| FR-251-AC-23 | Under AC-22's premises with `p` true at tick 0 and an event at tick 5000, the tick-0 entry has aged out and is folded into the summary cell, the buffer holds the tick-5000 entry and the cell only, and `once holds(p)` at tick 5000 is `true`. | Test (TC-1081) |
| FR-251-AC-24 | Under AC-22's premises with `p` true at tick 0 and false at tick 100, `historically holds(p)` at tick 5000 is `false` through the folded accumulator. | Test (TC-1081) |
| FR-251-AC-25 | The monitor's lower boundary is `HistoryCutoff` (FR-501): at the start, `once holds(p)` over trace with `p` never true is `indeterminate` with reason `MissingHistory`, not `false`; `historically holds(p)` over a trace with `p` always true is `indeterminate`, not `true`; one witness (a true `p`, a false `p`) decides `once` true and `historically` false. | Test (TC-1081) |
| FR-251-AC-26 | Under AC-22's premises, three events inside one tick cover settle the affected `once holds(p)` obligations `Incomplete(LimitReached{limit: event_buffer, value: 2, setting: monitor.max_event_rate})`, the sequence never holds more than 2 entries plus the cell, and the operator's buffer is emptied and its cell reset to `indeterminate` with `history_start` at the overflowing event's tick. | Test (TC-706) |
| FR-251-AC-27 | After the overflow of AC-26, with `p` true at the lost third event and no `p` after it, `once holds(p)` at tick 9000 is `indeterminate`, not `false`; with a later true `p`, `true`. For `historically holds(p)` with `p` false at the lost event and true afterwards, the result at tick 9000 is `indeterminate`, not `true`; with a later false `p`, `false`. | Test (TC-1081) |
| FR-251-AC-28 | On the 16-bit target of AC-3 after `on_gap`, with `p` true before the gap and never after it, `once holds(p)` is `indeterminate`, with `p` true after it `true`; `historically holds(p)` with `p` true before and after it is `indeterminate`, with `p` false after it `false`. Each result retains the subformula, the cell's `history_start` and the origin, and a replay gives the same results. | Test (TC-1081) |
| FR-251-AC-29 | `p since q` with `q` at tick 0, `p` true at ticks 100 and 200, and an event at tick 5000 is `true` at tick 5000 through the folded accumulator; with `p` false and `q` false at tick 150 and, at tick 5000, `p` true and `q` false it is `false` (`A := q or (p and A)` with `A` false). `p triggered q` follows `T := q and (p or T)`: with `q` false at tick 100 it is `false` at tick 5000, and with `q` true at every position and `p` false throughout it is `indeterminate` (the cutoff prefix), not `true`. | Test (TC-1081) |
| FR-251-AC-30 | On the 16-bit target of AC-3 after `on_gap`, `p since q` with `q` before the gap and `p` true after it is `indeterminate`; with `q` after the gap and `p` true since, `true`; with a position after the gap that has `p` false and `q` false, `false`. `p triggered q` is the dual. | Test (TC-1081) |
| FR-251-AC-31 | Under AC-22's premises, `once[0 ms, 10 ms] once holds(p)`: the inner unbounded operator has retention 10000 + 0 + 2 covers = 10002 ticks and capacity ceil(0.002 x 10002) + 1 = 22; `once` over `once[0 ms, 10 ms] holds(p)` (a bounded operator under an unbounded past one, an enclosing unbounded operator adding 0 to `L`) gives the inner `once[0 ms, 10 ms]` retention 0 + 10000 + 2 covers = 10002 ticks and capacity 22, and the outer unbounded `once` capacity ceil(0.002 x 1) + 1 = 2. | Test (TC-706) |
| FR-251-AC-32 | Under AC-22's premises, `historically (holds(req) implies eventually[0 ms, 5 ms] holds(ack))` is planned, not refused, with settling delay 5000 ticks, retention 5000 + 0 + 1 cover = 5001 ticks and capacity ceil(0.002 x 5001) + 1 = ceil(10.002) + 1 = 12. With `req` at tick 0 and no `ack`, the tick-0 entry is not folded before its `eventually` window is closed by progress: at tick 4000 the result is `Pending`, and once progress shows the window closed without `ack` the entry folds and `historically` is `false`; with `ack` at tick 2998 it folds with value true and the result stays `indeterminate` (cutoff), not `true`. | Test (TC-1081) |
| FR-251-AC-33 | Under AC-22's premises with `activation_capacity` 1, `historically (holds(req) implies eventually holds(ack))` is planned, not refused; two `req` events before any `ack` start one `eventually` instance, and the second is not started and settles `Incomplete(LimitReached{limit: activation, value: 1, setting: monitor.max_activations})`; the first entry stays in the buffer unfolded while its instance is pending, and the operator is `Pending`, never `true`. | Test (TC-1081) |
| FR-251-AC-34 | Under AC-22's premises, `once[5 ms, *] holds(p)` is planned, not refused, with retention 5000 + 0 + 1 = 5001 ticks and capacity ceil(0.002 x 5001) + 1 = 12; with `p` true at tick 0, the result at tick 4000 is `indeterminate` (the window reaches before the baseline) and at tick 6000 `true`, as `once[0 ms, *]` at tick 1000. | Test (TC-1081) |
| FR-251-AC-35 | `always eventually holds(p)` is planned, not refused, keeps no history buffer and holds one constant-size accumulator per live inner instance in `activations`; with `activation_capacity` 2 and three positions before any `p`, the third inner instance is not started and settles `Incomplete(LimitReached{limit: activation, value: 2, setting: monitor.max_activations})`, none dropped silently, and the first `p` settles the two live `eventually` instances `true`; on an open trace the operator is `Pending`, never decided `true` by the absence of a counterexample. | Test (TC-1081) |
| FR-251-AC-36 | `eventually always holds(p)` is planned, not refused; with `activation_capacity` 2 and `p` true at positions 1, 2 and 3, positions 1 and 2 start two pending `always p` instances and position 3 settles `Incomplete(LimitReached{limit: activation, value: 2, setting: monitor.max_activations})`; the first `p` settles no `always p` instance true; a `p` false at position 4 settles the two live instances `false` and leaves the outer `eventually` `Pending`. | Test (TC-1081) |
| FR-251-AC-37 | Two plans of AC-1's clause that differ only in the checked body of one atom, or only in the checked body of the activation guard, have different `PlanId`s. | Test (TC-1081) |
| FR-251-AC-38 | Two plans of AC-1's clause that differ only in the checked body of a declared helper function that an entry calls, directly or through another helper, have different `PlanId`s and different plan-root keys while the helper's own `node_id` (its declaration path, its call identity) is unchanged and it stays a declared node; the key covers the helper through its body root's node `node_id`, which is content, and not through a graph position or a name alone (QSpec FR-487). | Test (TC-1081) |
| FR-251-AC-39 | `TimedMonitorPlan` built through its public constructor with one atom body swapped has an unequal `PlanId`, and a plan with equal bodies throughout has an equal `PlanId`. | Test (TC-1081) |
| FR-251-AC-40 | Under AC-2's premises (`once[0 ms, 10 ms] holds(p)`, capacity 22, `b` 10000 ticks, `u` 0), 23 events at ticks 0 to 22 with `p` false at ticks 0 to 21 and true at the lost 23rd event settle the affected obligations `Incomplete(LimitReached{limit: event_buffer, value: 22, setting: monitor.max_event_rate})` and raise `history_start` to `22 + 1 + 0` = 23; `once[0 ms, 10 ms] holds(p)` read at tick 5000 is `indeterminate`, not `false`, at tick 10022 (`now - b` is 22) is still `indeterminate`, and at tick 10023 (`now - b` is 23) decides from the entries then in the window. For `historically[0 ms, 10 ms] holds(p)` with `p` true at ticks 0 to 21 and false at the lost event, the read at tick 5000 and at tick 10022 is `indeterminate`, not `true`. With `u` 2 ticks the lost event at tick 22 raises `history_start` to 22 + 1 + 4 = 27 and the first deciding tick is 10027. | Test (TC-706) |
| FR-251-AC-41 | For `once`, `historically`, `since` and `triggered` without an upper bound, the differential of AC-7 compares the monitor with the `HistoryCutoff` evaluator of "Unbounded past operators" (every atom before the baseline `indeterminate`) and requires equal verdicts; it also evaluates the same trace under QSpec's infinite-trace evaluator and requires that every definite monitor verdict equals that evaluator's verdict, and an `indeterminate` monitor verdict is not compared. | Test (TC-1081) |
| FR-251-AC-42 | The `Emission` returned with a plan-root package holds, non-empty wherever the acquisition had rows, the selected model declaration rows, selected model field rows, source-unit rows, model enum and source enum declaration and member rows, source union rows, source record rows, tuple declaration rows, operation-member rows, equality producer rows, equality fact rows, temporal binding rows and omitted nodes of that acquisition. Every node id in each of them (anchor, structural node, record node, declaration node, union payload node, record member type node, checked node, source-map entry node, fact nodes, omitted node and the node an omission cause names, temporal binding declaration) equals the plan-root package's id for the node, so a node the writer renames is named by its renamed id in every row and a row naming no renamed node is unchanged; no row is written anew. The selected model declaration and field rows equal the unit's acquisition rows, and IR's selected exact identity join accepts them against the package read back. | Test (TC-1081) |
| FR-251-AC-43 | For a plan root written over a unit with two temporal clauses, each subject's clause node and claim ordinal select, through the returned `Emission`'s temporal binding rows, the declaration the plan root's `clause` binding names and the declaration of the package's `temporal_clause_names` row of that ordinal, read back through the strict reader; when the writer renames the clause node's id, the selection returns the renamed id and not the base package's id; a subject whose clause node or claim ordinal the package does not hold selects nothing, and a subject's selection never returns the other clause's declaration. | Test (TC-1081) |

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
