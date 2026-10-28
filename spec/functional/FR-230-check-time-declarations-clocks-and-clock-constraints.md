---
id: FR-230
title: "Check a state model's time declaration, clocks, clock constraints, time invariants and urgency"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-142
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-205
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-361
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-393
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-154
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/filament-core-data/FR-146
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-415
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-143
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
---
# FR-230: Check a state model's time declaration, clocks, clock constraints, time invariants and urgency

## Description

QSL's S3 checker SHALL admit the timed members of a state model (ADR-026
CK-1 to CK-9): one `time` member naming the model's time source, `Clock`
fields under a dense source, clock resets in postconditions, atomic clock
constraints in guards, time invariants and urgency declarations. It records
them on the checked state model so that the timed subject (FR-231), the zone
engine (FR-239) and replay (FR-237) read one checked form. S3 confines every
clock read to an atomic clock constraint, so no real value reaches the value
kernel. It admits a clock, and a quantity in a declared unit, in every type
position a unit writes: a function, predicate or operation parameter and
result, a record field, a tuple element, a union payload, a type alias target
and the argument of `Option`, and it admits a collection of either in every position a
collection of its element type is admitted, with one clock place for each
element (QSpec FR-415 "Clock collections").

## Use case

A verification operator models a client that times out after 3 ms and a
server that replies between 1 ms and 3 ms after a request. They declare
`time dense unit ms`, give the call object a clock, reset it on `send`,
guard `reply` with `x >= 1 ms` and bound waiting with time invariants. A
mistake such as copying the clock into an integer field, or comparing two
clocks, is refused at check time with a span.

## Stage boundary

S1 parses the timed productions. S2 builds the `TimeSourceForm`,
`TypeForm::Clock`, `TimeInvariantForm`, `UrgentForm` and time-quantity forms
defined by FR-393, retaining their source spans regardless of the selected
facets. S2 does not resolve the time unit, time dimension or tick operation,
check source uniqueness or clock placement, or validate a timed constraint.
S2 produces no timed-facet or timed-model refusal for these grammar-valid
forms. S3 reads the unit's FR-110 selected-layer union, including each
selected layer's `requires` closure. If that union omits
`quire.temporal.complete/v1`, FR-230-AC-7 applies. If it contains the layer,
S3 validates the forms and either records the checked `TimedModel` and its
derived constraints or returns the located refusal this requirement names;
it does not leave a timed form unchecked.

## Inputs

- A parsed state model with its members, in the spelling QSpec's shared
  grammar fixes for the `time` member, `Clock` fields, resets, clock
  constraints, `time invariant` and `urgent` (QSpec half, References).

## Outputs

On the checked state model:

```rust
pub enum TimeSource {
    Dense { unit: TimeUnit },                                   // QSpec FR-142
    Tick { operation: OperationId, period: ExactQuantity, unit: TimeUnit },
}

pub struct TimedModel {
    pub source: TimeSource,
    pub clocks: Vec<ClockField>,            // (object type, place path, guard), declaration order
    pub invariants: Vec<TimeInvariant>,     // when P { x <= c, ... }
    pub urgency: Vec<Urgency>,              // When(P) | Operation(OperationId)
}

pub struct ClockField {
    pub object_type: TypeId,
    pub path: Vec<PlaceStep>,               // field names, tuple indexes, union cases and collection slots down to the Clock
    pub guard: Option<PlaceGuard>,          // Some under an Option, `?` field, union case or collection slot: the presence test
}

pub enum PlaceStep {
    Field(Name), Index(usize), Case(Name),
    Slot(CollectionShape),                  // an element place of a clock collection: one entry for each clock of the element
}

pub struct CollectionShape { pub kind: CollectionKind, pub min: u64, pub max: Option<u64> }   // None: no declared maximum

pub enum Provenance { Carried { from: SlotExpr }, Fresh { resets: Vec<(ClockRef, ExactRational)> } }

pub struct ClockConstraint { pub clock: ClockRef, pub op: ClockOp, pub bound: ExactRational }
pub enum ClockOp { Lt, Le, Eq, Ge, Gt }
```

`CheckedStateModel::time: Option<TimedModel>`, each operation's checked
guard as data predicates and atomic clock constraints, and each operation's
resets as `(ClockRef, ExactRational)` pairs and, for each collection-valued postcondition
conjunct over a clock collection, the `Provenance` of every occurrence of the written value. A typed `CheckRefusal` with a
span on refusal.

## Behavior

### The `time` member

- The checker SHALL admit at most one `time` member per state model; a
  second SHALL refuse `invalid_timed_model`/`duplicate-time-source` at
  its span, naming the first (QSpec FR-415).
- The checker SHALL resolve the unit `u` as a unit of QSpec FR-142's time
  dimension (next section), and SHALL refuse a unit of another dimension,
  including a unit of a different dimension named `Time`, with
  `ill_typed`/`type-mismatch` at the unit's span.
- For each `time M ...` declaration, S3 SHALL resolve `M` through its selected
  model package to exactly one checked semantic node that denotes that model.
  The checked time source SHALL retain both the alias `M` and that node's
  `NodeKey`. S3 SHALL refuse a selection with no such node as
  `missing_declaration`/`missing-name`, and a selection with multiple such
  nodes as `ambiguous_declaration`/`ambiguous-name`, at the model alias span.
  It SHALL NOT choose a node by package declaration order, spelling, or a
  fallback identity. This is the model binding the timed-model root writes.
- Under `time tick O period p unit u`, the checker SHALL resolve `O` to an
  operation of the model (FR-103), refusing `missing_declaration`/
  `missing-name` otherwise, and SHALL require `p` to be a positive exact
  rational quantity of the time dimension (QSpec FR-205), refusing zero or a
  negative period with `invalid_model_binding`/`malformed-declaration`.
- A model with a `time` member is a timed model. The `TimedModel` SHALL be
  part of the checked package, so two models that differ only in it have
  different package identities.

### The time dimension and its units

QSL has no built-in dimension and no built-in unit: `ms` and `s` are not
predeclared. The checker reads the **unit graph of the checked package**
(FR-091), which holds the dimensions and units a QSL unit declares in its own
source (FR-094-AC-8) together with those the admitted domain package supplies
in its `dimensions` and `units` (QSpec FR-154, filament-core-data FR-146);
QSpec FR-142 gives the meaning of their identities and conversions. A model
may therefore use a time unit declared in either place. A unit the domain
package supplies is a checked-package node with no declaration name, owned by
its selection (QSpec FR-322 "Model-owned members" step 4): the source reaches
it only through the model alias, `time dense unit M::ms` after
`model M = "acme/rpc" …`, matched against the `name`s of that package's units,
and a source `ms` and the package's `ms` are two units. A dimension the package supplies is reached the same way,
`M::Time`, and a source `Time` and the package's `Time` are two dimensions
(QSpec FR-142 "Namespaces").

- The **time dimension** of a model SHALL be the exact nominal dimension node
  referenced by its checked timed-model root's `time_dimension` binding. It is
  the base dimension declared as `Time` that the resolved time unit's
  `dimension_node_id` identifies. The unit resolution selects the exact
  nominal node; the reader never guesses among same-named nodes or uses
  declaration order. Every quantity the model reads as time (the period
  `p`, a clock constant, a reset value and a time-invariant bound) SHALL be a
  quantity of that node, or of a dimension node that a dimension equivalence
  of the unit relates to it (QSpec FR-142 "Dimension equivalence"; FR-091),
  which the checker converts exactly to the model's unit `u` through the
  equivalence's root ratio (FR-230-AC-28), and SHALL
  otherwise be refused `ill_typed`/`type-mismatch` at its span (QSpec FR-142
  "Dimensions of different owners"). The **time units** SHALL be
  the units typed by that node. The explicit binding records this resolved
  authority in the checked graph; dimension names and dimensional vectors do
  not choose among candidate nodes. A unit on another nominal dimension is
  not a time unit. For example, a graph with the
  root `s` and a unit `ms` whose edge to `s` has scale `1/1000` makes `ms`
  and `s` time units; any zero-offset root and any edge chain that composes
  to the same scale do as well.
- The checker SHALL read `time dense unit u`, `time tick O period p unit u`
  and a quantity such as `3 ms` in this order: `u` is found by name, its
  dimension node is verified as the base `Time` dimension and recorded as the
  explicit root `time_dimension`, then `u` has a zero composed offset. It SHALL convert a quantity exactly by
  composing the unit's edges (QSpec FR-142).
- If a QSL unit's own source declares two dimensions named `Time`, the
  assembler refuses it first, `ambiguous_declaration`/`ambiguous-name`
  (FR-091), and S3 never sees it. If the unit's source and an admitted domain
  package both declare a dimension of one name, `Time` or any other, or two
  admitted domain packages do, the declarations are distinct dimensions and
  the checker SHALL refuse nothing (QSpec FR-142 "Namespaces"); the time
  dimension is the one the unit `u` is typed by.
- If the graph has no base dimension named `Time`, the checker SHALL refuse a
  `time` member or time quantity `missing_declaration`/`missing-name` at its
  span, naming `Time`.
- If the graph has no unit named `u`, the checker SHALL refuse
  `missing_declaration`/`missing-name` at the unit's span, naming `u`.
- If `u` is typed by a dimension other than the base `Time` dimension, the
  checker SHALL refuse `ill_typed`/`type-mismatch` at the unit's span.
- If a time unit's composed offset to its root is not zero, the checker SHALL
  refuse `ill_typed`/`type-mismatch` at the unit's span, naming the offset,
  because FR-142 allows no arithmetic on an affine unit.
- The unit lookup and the dimension comparison SHALL use the checked
  package's identities, so two models whose graphs declare `ms` differently
  have different package identities.

### Clock fields and the domain package

A `Clock` field belongs to a model object type, and object types come from
the admitted domain package. The package declares it as a field typed
`ix://quire/native/Clock` with the clock field shape (filament-core-data
FR-146, the contract of Linear STD-295; QSpec FR-154), and QSpec FR-322
"Model-owned members" gives it the clock type. The checker SHALL read each
such field as a clock of its owning object type, in declaration order, and
SHALL apply "Clocks and resets", "Clock constraints and guards" and "Time
invariants and urgency" to it. A unit that declares a clock in
a record, tuple, union, alias, parameter or result of its own source does so
under "Clock-typed positions", and the package fields feed the same places;
the rules of this requirement also need the facet to be selectable (Timed
facet gate).

A package field whose `typeRef` is `ix://quire/native/Clock` SHALL be admitted
as a clock field, not looked up as a native value type. Its package-declared
`unique:false` multiplicity is valid; the field still contributes the clock
place(s) required by its declared bounds.

### Clocks and resets

- The checker SHALL admit a field of type `Clock` only in a model whose
  source is `Dense`, and SHALL refuse it elsewhere with
  `invalid_timed_model`/`clock-without-dense-time` at the field's span.
- A postcondition conjunct `self.x = c` on a clock `x` of the operation's
  frame, with `c` an exact non-negative rational time quantity, SHALL check
  as a reset to `c` converted exactly to `u`. Any other write to a clock
  SHALL refuse `invalid_timed_model`/`clock-write`.
- A clock in the frame that the operation does not reset keeps its value;
  the checker records no write for it.

### Clock constraints and guards

- The checker SHALL admit `x ~ c`, with `~` one of `<`, `<=`, `=`, `>=`,
  `>` and `c` an exact non-negative rational time quantity, as an atomic
  clock constraint, converting `c` exactly to `u` (QSpec FR-142, FR-205).
  A negative constant SHALL refuse `invalid_timed_model`/
  `negative-clock-constant` at the constant's span.
- The checker SHALL admit a clock read only as the clock operand of an
  atomic clock constraint. A clock compared with another clock SHALL refuse
  `invalid_timed_model`/`clock-comparison`, and a clock read anywhere else,
  including a clock in arithmetic or a clock assigned to a data field,
  SHALL refuse `invalid_timed_model`/`clock-read-outside-constraint`, each
  at the read's span.
- An operation's precondition SHALL check as a Boolean combination of data
  predicates and atomic clock constraints. The checker SHALL record the
  guard in disjunctive form over convex clock parts, each part a conjunction
  of atomic clock constraints with its data predicate.

### Time invariants and urgency

- `time invariant when P { … }` SHALL check `P` as a state predicate
  through the clause checker invariants use (FR-104), and its body as a
  conjunction of atomic clock constraints whose operator is `<` or `<=`.
  A lower bound or an equality in the body SHALL refuse
  `invalid_timed_model`/`time-invariant-shape`.
- `urgent when P` SHALL check `P` as a state predicate (FR-104).
- `urgent O` SHALL resolve `O` to an operation of the model, and SHALL
  refuse an `O` whose guard reads a clock with `invalid_timed_model`/
  `urgent-guard-reads-clock` at the declaration's span, naming the clock
  read.
- A time invariant or urgency declaration in a model with no `time` member
  SHALL refuse `missing_declaration`/`missing-name`, naming the missing
  `time` member.

### Clock-typed positions

A **clock-bearing** type, a **clock place** and a **guarded** place are
QSpec FR-415's "Clock places". S3 follows an alias chain to the type the alias
names, so `type A = Clock;` makes `A` the clock type, `type B = Option<A>;` the
optional clock type, and the rules below apply to a position through its
resolved type. A clock-bearing type is admitted in every type position a unit
writes; the rules below fix what each position means and refuse only an
ill-formed use.

- **Places.** The checker SHALL list, for each object type of the model in
  field declaration order and each field in order, the clock places of the
  field's type depth first (a record field by name, a tuple element by index, a
  union case by name), one `ClockField` for each. The clocks of the model are
  the places of its objects: a model with `k` objects of a type holding `m`
  places has `k * m` clocks, and no state changes the number.
- **Guards.** A place under an `Option`, a `?` field or a union case SHALL
  carry the presence test of that case as its `PlaceGuard`. The checker SHALL
  accept `value(p)` of a guarded place `p` as a clock operand only where
  `present(p)` is established by the same guard conjunct or by the time
  invariant's predicate, and SHALL refuse it otherwise
  `undefined_expression`/`unproved-presence` at the read. A guard that
  contains `value(p) ~ c` SHALL record the data predicate `present(p)` with the
  convex part. A time invariant `when P { value(p) <= c }` SHALL bound `p`'s
  clock only at states where `P` holds, and `P` establishes `present(p)`.
- **Presence bit.** Every guarded place SHALL have a **presence**, which
  `present(p)` reads; this requirement states the contract that S3 records, and
  FR-120 and FR-231 own the evaluation. A guarded place that has a snapshot
  member on its path (an `Option<Timer>`, whose data FR-106 carries, a union
  case, whose value carries its `member`, QSpec FR-181, and a collection slot,
  whose liveness is the collection's own size) SHALL read its presence from
  that data or member and SHALL hold no bit. Only a guarded place with no
  snapshot member (an `Option<Clock>`, including an element of
  `Sequence<Option<Clock>>`) SHALL hold a **presence bit**, one for each such
  place of each object, in the state and in the state key, and its initial bit
  SHALL range over absent and present; this is an over-approximation, sound
  for every-behaviour families and optimistic for possibility families, and the
  number of initial states grows as 2 to the power of those places, times the
  count ranges of "Initial element count". `present(p)` of a guarded place SHALL check as a `Boolean`
  data predicate over the place's presence (its bit or its data), read through the data-predicate path and never
  as a clock read; the recorded guard carries it as a data predicate of its
  convex part.
  **Presence is derived.** `present(p)` is the derived Boolean predicate of
  QSpec FR-415 "Presence is derived" and FR-322 "Timed-model nodes" (Presence),
  FR-322-AC-155 to FR-322-AC-157, not a field. The checker SHALL resolve it
  through the place identity of "Place identity" (the owner object, the
  `clock_place` node and the slots) and SHALL NOT declare an `Option<Boolean>`
  or any other member for it, so no model's declaration identity changes for
  holding an `Option<Clock>`. `p` MAY name a place of another object
  (`present(other.v)`, evaluated at `other`) and the element bound by a
  quantifier (`present(e)` in `forall(e in self.q: ...)` or `exists`,
  evaluated at each live slot `e` binds), and SHALL be recorded as a data
  predicate of the convex part at that owner and slot.
  A write that makes the place present (`self.v = c`) sets the presence
  (the bit, or the data) and resets the clock to `c`. A **drop** (`self.v = absent`, another union
  case, or the removal of the slot) sets the presence false and leaves the dropped
  clock **unconstrained**: no zone constraint, time invariant bound or elapse
  applies to it, it has no valuation, and a later write that makes it present
  resets it afresh. "Every present clock is 0 in every initial state" (QSpec
  FR-415, FR-231, FR-239's zero valuation) therefore reads over the present
  clocks only. **State-key form.** A presence bit SHALL be held only for a **live** guarded
  place that has **no snapshot member**: an `Option<Clock>` (as a field, a
  record or tuple component, or a collection element). A place whose presence
  comes from data has no bit, and its state-key encoding SHALL be unchanged, so
  no state key of a data-backed place changes; an `Option<Clock>` field gains
  its option shell. The bit SHALL be encoded in the
  object's `record` (FR-120) by the place's own `Option` shell: an
  `Option<Clock>` value encodes as `{"type":"option","value":null}` when absent
  and `{"type":"option","value":{"type":"tuple","name":"Clock","components":[]}}`
  when present, so QSL's `semantic` SHALL stay the map of FR-120. The bits of a
  collection SHALL be those of its live elements only, in the collection's own
  order, so a variable-size or unbounded collection has exactly as many bits as
  it has live `Option<Clock>` elements, and a dropped slot has none.
- **Parameters.** A call argument for a `Clock` parameter, or a parameter of a
  clock-bearing type, SHALL be a **place expression**: a place path from
  `self` or from an object, record or tuple parameter; a clock parameter of the
  caller; a call of a function that returns a place; or an `if` or `match`
  whose branches are place expressions. Any other argument SHALL refuse
  `ill_typed`/`type-mismatch` at the argument. A path through a reference field
  or through an `if` or `match` SHALL check as one place for each object or
  branch it can reach, each with the data condition that selects it, and a
  constraint over it SHALL record the disjunction of those guarded constraints.
- **Results.** A function whose result type is clock-bearing SHALL have a
  body that is a place expression, and SHALL refuse `ill_typed`/
  `type-mismatch` at a body that is any other expression. A call of it is a
  place expression.
- **Clock-reading declarations.** A function or predicate whose parameter or
  result is clock-bearing is a clock-reading declaration. The checker SHALL
  check its body by "Clock constraints and guards": a clock parameter is read
  only as the clock operand of an atomic clock constraint or passed to another
  clock-reading declaration. It SHALL admit a call of a clock-reading
  declaration only as, or inside, an operation precondition, a time invariant
  body or the body of another clock-reading declaration, and SHALL expand the
  call there by substituting the argument places for its parameters, so that
  the recorded guard holds atomic clock constraints only. A call anywhere else
  SHALL refuse `invalid_timed_model`/`clock-read-outside-constraint` at the
  call. A clock-reading declaration SHALL write no clock; a reset in its body
  refuses `invalid_timed_model`/`clock-write`. The checked package SHALL hold
  no function or predicate node for a clock-reading declaration and no call
  node for a call of one.
- **Bounds.** The bound of an atomic clock constraint in a clock-reading
  declaration SHALL be a time quantity literal, or a quantity parameter of the
  declaration; the bound after expansion SHALL be an exact non-negative
  rational time quantity, and a call whose argument for such a parameter is not
  a constant of that kind SHALL refuse `invalid_timed_model`/
  `clock-bound-not-constant` at the argument.
- **Records, tuples and unions.** A record, tuple or union value of a
  clock-bearing type is a place, not a data value. The checker SHALL read a
  data field of such a record as an ordinary value, and SHALL refuse a
  clock-bearing value read whole (as an operand of `=`, an argument of a data
  parameter, the right side of an assignment to a data field, or a conversion)
  `invalid_timed_model`/`clock-read-outside-constraint` at the read. A record,
  tuple or union literal with a clock component SHALL check only as the right
  side of a postcondition conjunct `self.f = literal` whose clock components
  are exact non-negative rational time constants converted to `u`, as a reset
  of each of those clocks together with the writes of the data components; a
  guarded place written a constant becomes present with that clock, and one
  written `absent` or another union case is dropped. A literal with a clock
  component anywhere else SHALL refuse `invalid_timed_model`/`clock-write`.
- **Checked package.** The checked package SHALL hold no value/type node for
  `Clock`, an alias of it, `Option<Clock>`, a clock parameter or a clock result.
  It SHALL emit one `timed_model`/`timed_model` root node owned by the state
  model, with an explicit `time_dimension` reference equal to the resolved
  unit's nominal `dimension_node_id` and dependency edges to its `time_source`, `clock_place`,
  `clock_reset`, `clock_guard`, `time_invariant`, `urgency` and
  `clock_constraint` nodes. Each node's closed body records its canonical
  kind-specific data: source kind and resolved unit; a tick source additionally
  carries its model and a `state`/`operation_anchor` reference whose context
  equals both the source and root model, whose operation resolves as an
  operation declared by that model, and whose period is exact. A generic
  `function` node is not a tick operation. A place carries its owner, ordered
  field/tuple/union/collection-slot path, and `resolved_path` rows in one-to-one
  path order; each row records declaring type, member identity/type and, for a
  collection, its kind and bounds. These rows are checked-package data joined
  to the selected declaration; I2 does not reopen source text or infer members
  from a spelling.
  A `Slot` step is written as the template step of QSpec FR-322 "Timed-model
  nodes" (FR-322-AC-148 to FR-322-AC-150). There is one `clock_place` node for
  each path, and its slot selector is the symbolic element selector, never an
  index, so a collection holding more or fewer elements changes no node.
  A model whose guards the emission cannot yet write SHALL be refused
  `TimedModelNotEmitted`; it SHALL NOT be emitted with an empty `guards`
  aggregate. An empty aggregate is written only for a place or model that has
  no guard.
  The remaining node data records guards;
  reset target and exact converted value; guard/invariant constraint terms;
  urgency predicate or operation; and every bound's comparison, exact value
  and resolved unit. `Clock` remains a place, never a value type. Record and
  tuple nodes list data components only; a union case with a `Clock` payload
  has no data payload and its place is represented by the timed-model node.
  Timed nodes enter `semantic_graph.nodes` in producer graph order, the
  reader's identity projection and package identity. S4 SHALL not refuse a
  successfully checked timed model as `TimedModelNotEmitted`. The I2 read-back
  SHALL reconstruct the same timed model and derived constraints from these
  nodes and reject missing roots, dangling edges, wrong ownership, noncanonical
  paths, non-time units, malformed exact quantities and duplicate places with
  the corresponding typed cause.
- **Operation parameters.** An operation with a `Clock` parameter SHALL have
  one checked instance for each place of the parameter's range, and each
  instance's identity SHALL name the place. The range SHALL be the places of
  the model; when the postcondition resets the parameter it SHALL be the places
  of the clock fields the operation's frame (FR-103) names, and a reset of a
  clock the frame does not name SHALL refuse `invalid_timed_model`/
  `clock-write` at the reset.
- **Clock collections.** The checker SHALL admit a clock-bearing type as the element of a `Set`,
  `Bag`, `Sequence`, `OrderedSet` or as the value of a `Map`, nested in `Option`, in another
  collection, in a record, tuple or union, and as a member whose multiplicity is not `[1, 1]`,
  following QSpec FR-415 "Clock collections". It SHALL refuse a clock-bearing `Map` key
  `ill_typed`/`operator-ineligible` at the key type, because a key is an immutable lookup value
  and a clock advances with time. The places of a clock collection SHALL be listed in "Places"
  with a `Slot(CollectionShape)` step, one `ClockField` for each clock of the element type, in
  element-type declaration order; a slot `j` is live exactly while the collection holds it, and
  the guard of a slot place is that presence test, composed with the guards above it.
  The static clock dimension of the model SHALL be, for each object and each clock collection,
  the declared maximum (the collection type's bound `u`, the member's multiplicity upper bound)
  times the clocks of one element, summed with the places outside collections; a collection with
  no declared maximum SHALL record `max: None` and add no static bound, and its live places are
  bounded in analysis by the model checker's `max_clocks` limit (QSpec FR-415 "Cardinality
  bound").
- **Initial element count.** An element place of a clock collection is live
  exactly while the collection holds the element. A collection whose element
  has a non-clock component takes its initial elements, and so its count, from
  the snapshot (QSL FR-106). A **pure-clock collection** is a `Sequence`,
  `Set`, `Bag` or `OrderedSet` whose element type has no non-clock component
  (FR-120's rule): `Clock`, an alias of it, `Option<Clock>`, and a record or
  tuple of such components. It has no snapshot data, so its initial element
  count SHALL range over `[min, max]` (over-approximating) and SHALL NOT take
  `min`; each initial state is one choice of count. `min` is the collection's
  lower bound (QSpec FR-144's `K<T>[a, b]` `a`, a field's multiplicity lower
  bound, else 0) and `max` its declared maximum. Each present initial clock is
  0; an absent element (an `Option<Clock>` element at an initial bit of
  absent) is unconstrained ("Presence bit"). A `Set` or `OrderedSet` whose
  element has no data part (`Set<Clock>`, `OrderedSet<Clock>`,
  `Set<Option<Clock>>`, a set of clock-only records) holds at most one element
  by data-part uniqueness (FR-120's `p` = 1), so its range is
  `[min, min(max, 1)]`, and a `min` above 1 leaves no initial state
  (`Undecided(NoInitialState)`, FR-239). With no declared maximum the range is
  `[min, ∞)`: the initial states are produced in ascending count and the
  analysis stops `incomplete` with `limit-reached` on `max_clocks` as in
  "Cardinality bound" (QSpec FR-415-AC-13); it does not return
  `NotSimulated::RequiresBound`, which FR-120 applies only to `domains()` of an
  unbounded frame-granted value. A `Map` is not a pure-clock collection: its
  keys are data the snapshot carries, so its entries come from the snapshot
  (the Map ruling stays with QSL-1169).
- **Element identity.** A `Set`, `Bag` or `OrderedSet` of a clock-bearing element type SHALL
  decide uniqueness and multiplicity by equality of the data parts (QSpec FR-144), so the
  checker SHALL record `Set<Clock>` as holding at most one element, and SHALL refuse `=` and
  `!=` over a clock collection, `contains`, `convert` and any other query that compares
  elements as values, and `count`, `fold`, `reduce`, `sum`, `map` and `filter` whose body reads
  a clock, `invalid_timed_model`/`clock-read-outside-constraint` at the read. The data fields of
  an element, `size` of the collection and the presence of a slot or a key are data reads and
  check as usual; they are recorded as data predicates of the timed model and read no clock.
- **Selection.** The checker SHALL admit the selectors `p[i]` for a `Sequence` or `OrderedSet`
  and `p[k]` for a `Map`, `i` and `k` data terms of the pre-state, as a place expression, and
  SHALL check the selected place as a guarded place whose presence test is `i < size(p)` or
  `k` being in the domain; it SHALL accept the selection where the same guard conjunct, the
  precondition or the time invariant's predicate establishes that test, and SHALL refuse it
  otherwise `undefined_expression`/`unproved-presence` at the selector. A `Set` and a `Bag`
  are read through `forall` and `exists` only.
- **Quantified reads.** The checker SHALL admit `forall(e in p: B)` and `exists(e in p: B)` in
  a precondition, a time invariant body and a clock-reading declaration, `p` a clock collection
  place expression and `B` a Boolean combination of data predicates over the data part of `e`
  and atomic clock constraints over the places of `e`. It SHALL record the guard of a
  `forall` as the conjunction of `B` over the live slots, true on an empty collection, and
  the guard of an `exists` as the union of convex parts, one for each slot, each carrying the
  data predicate "the collection holds the slot" and the element's data predicates, false on
  an empty collection; for a bounded collection the record holds one entry for each size from
  the minimum to the maximum, and for an unbounded one the record is the family of those
  entries indexed by size. A time invariant body `forall(e in p: e.x <= c)` SHALL check as the
  upper bound `x <= c` on each live slot of `p`, and a body `exists` SHALL refuse
  `invalid_timed_model`/`time-invariant-shape`, because a disjunction of bounds is not a
  conjunction of upper bounds.
- **Resets by selection.** The checker SHALL check `p[i].x = c`, `p[k].x = c`, `p[i] = literal`,
  `p[k] = literal` and `forall(e in p: P(e) implies e.x = c)` (`P` a data predicate over `e`'s
  data part, or absent) as resets of the selected places to exact non-negative rational time
  constants, require the collection field to be named by the operation's frame (FR-103), and
  record the selected slot expression or the data predicate with the reset. A place not selected
  keeps its value.
- **Collection-valued writes.** A postcondition conjunct `self.f = E`, `f` a clock collection,
  SHALL check `E` as a **place-preserving collection expression**: a collection literal
  (`sequence[...]`, `set[...]`, `bag[...]`, `orderedSet[...]`), the collection `self.f` or
  another clock collection place, an element bound by a traversal of one, and `filter`,
  `flatten`, `convert`, `map` whose body returns the bound element or a literal, applied to those.
  The checker SHALL record, for every occurrence of the written value, its `Provenance`:
  `Carried` from the slot of the pre-state element it comes from (the clock and its value follow
  the element to its new slot), or `Fresh` with the exact non-negative rational time constants of
  the literal's clock components. When two occurrences of a `Set`, `OrderedSet` or `Map` result
  are equal by data part (or key), the checker SHALL record the first occurrence in FR-145's
  visiting order. An occurrence that is neither carried nor a literal with constant clock
  components SHALL refuse `invalid_timed_model`/`clock-write` at the occurrence, and a clock
  collection written anywhere but as the right side of such a conjunct SHALL refuse
  `clock-write`. A frame that does not name `f` SHALL leave its places and provenance unchanged.
- **Clock references.** `Reference<Clock>` (a package `reference` whose `target` is
  `ix://quire/native/Clock`, or an alias of it) SHALL denote one clock place of the model, named
  by object, field path and, in a clock collection, slot, in the order of "Places". A value of
  `Reference<Clock>` is data: it holds no clock value, so it SHALL be stored in any field,
  parameter, result, record component or collection element a data value may occupy, and
  `Option<Reference<Clock>>` is absent or one place. The checker SHALL admit `ref(p)`, for `p` a
  place expression, as a data term whose value is the place `p` denotes, and `deref(r)` for `r` a
  `Reference<Clock>` as a place expression. A `deref(r)` SHALL check as one place for each place
  `r` can denote (every place of the model whose type is the clock type the reference targets, except a place under a `Map` value, which has no place id),
  each with the data condition `r = ref(place)`, and a constraint over it SHALL record the
  disjunction of those guarded constraints, as a path through a reference field does. A
  `deref(r)` of an absent `Option<Reference<Clock>>`, or of a reference to a slot place, SHALL be
  accepted only where the same guard conjunct, the precondition or the time invariant's
  predicate establishes `present(r)` and the slot's presence, and SHALL be refused otherwise
  `undefined_expression`/`unproved-presence` at the `deref`. A reset `deref(r) = c` SHALL check as
  a reset of each place `r` can denote, under the frame rule of "Operation parameters". `=` and
  `!=` over two `Reference<Clock>` compare the places they denote, by object, field path and
  slot, and never the clocks' values; they are data reads. A reference to a slot place names the
  slot, so it keeps naming that slot when the collection moves its element.
  **Place identity.** The place id of a `Reference<Clock>` SHALL be a
  canonical, one-to-one encoding of the triple (owner object identity, the
  `clock_place` node key, slot indices), so that it is distinct for every
  runtime place: `Call1.x` and `Call2.x` have distinct ids, as do slots 0 and 1
  of one collection, and one place always has one id. It is the UTF-8 text
  `place/v1/<h(owner)>/<h(node key)>/<slots>`, where `h` is the lowercase hex
  of the UTF-8 bytes of the owner object's FR-120 reference identity and of the
  `clock_place` node's key, and `<slots>` is the decimal slot index at each
  `Slot` step of the path in path order joined by `.`, or `-` when the path has
  none. Two place ids are equal exactly when their texts are byte equal. The id is
  stable within one checked package, compared for equality only (`<` over two
  ids refuses `ill_typed`/`operator-ineligible`), and an id of one package
  names no place of another. QSpec FR-415 states the same id as the state key, and
  nothing is vendored here. The `clock_place` node key names the template of the
  path and is the same for every instance; `<slots>` is the concrete position of
  the instance at each `Slot` step and is never part of any node. A place under a `Map` value (`self.m[k].x`) is selected by key, which
  a decimal index cannot name, so it has no place id and `ref` of one is
  refused `ill_typed`/`type-mismatch` until QSL-1169 rules the Map encoding.
  The same value is carried in a field, a parameter, a result, a record
  component and a collection element, and `Option<Reference<Clock>>` is absent
  or one place id. The domain-package target form of a `reference` whose
  `target` is `ix://quire/native/Clock` (or an alias of it) is that place id
  and needs no kernel value form: it is QSpec FR-181's reference value
  `{"type":"reference","universe","object_type","identity"}`, with the
  population's `universe`, `object_type` the effective id of the Clock
  type after alias resolution (so `Reference<A>` and `Reference<Clock>` to one
  place give one triple, and it never has the shape of an object reference to
  the owner) and `identity` the lowercase hex of the id
  text's UTF-8 bytes (quire-exact's `ObjectReference` takes any nonempty
  authored identity). A snapshot-supplied `Reference<Clock>` whose identity
  does not decode to a place id of this form, or names a place the universe
  and the checked package do not contain, SHALL be refused
  `invalid_runtime_input`/`invalid-value` (FR-120 key encoding). The state key
  holds the place id as that value and no clock value.
- **Clock constraints on a type.** A package constraint whose `appliesTo` is the clock type
  (`ix://quire/native/Clock` itself or an alias of it) is a **clock constraint**. The keywords
  admitted are `max`, with `operands.value` an exact non-negative rational number of the model's
  time unit, which the checker SHALL record as the time invariant `x <= c` on every clock place of
  that type (the places typed by the alias, or every clock place when `appliesTo` is `Clock`),
  guarded by the place's presence, and `min`, with the same operand, which the checker SHALL
  refuse `invalid_timed_model`/`time-invariant-shape` because a lower bound on every clock is not
  an upper-bound invariant and no clock is below it at a reset. Any other keyword on a clock
  refuses `invalid_model_binding`/`malformed-declaration` at the constraint, because no other
  keyword has a meaning for a clock. A time source is required as for any clock place.
- **Dense time.** A clock place held by a model object field SHALL check by
  "Clocks and resets", under `Dense` only. A function, predicate or record
  type that mentions `Clock` is admitted in any unit; the requirement binds
  where an object field holds a clock place.

### Unit-typed members

`Quantity<u>` (QSpec FR-142 "Quantity type reference") is a type in every
position that takes a type, and the checker SHALL treat it alike in each.

- The checker SHALL admit `Quantity<u>` as a function, predicate or operation
  parameter or result, a record or object field, a tuple element, a union
  payload, a type alias target, the argument of `Option`, and the element of
  `Set`, `Bag`, `Sequence`, `OrderedSet` and map types, and SHALL resolve each
  `u` in the unit namespace of the checked package (FR-091).
- The type of each such position SHALL be the unit's own `scalar_type`/`unit`
  node, and the position SHALL introduce no node of its own: a field node is
  typed by it, an alias of `Quantity<u>` is that same type under the alias's
  name, and `Option<Quantity<u>>` and the collections are the composite types
  over it.
- A value of `Quantity<u>` SHALL stand where `Quantity<u>` is required in any
  position, and a quantity of another unit, including another unit of the same
  dimension, SHALL refuse `ill_typed`/`type-mismatch` at the expression.
- A package field, operation parameter, operation result, alias `target`,
  sequence `items` and map `values` whose `typeRef` is
  `ix://quire/unit/<name>` SHALL read as `Quantity<u>` at that position, the
  alias, sequence or map as the alias, `Sequence<Quantity<u>>` or
  `Map<Text, Quantity<u>>` of it (FR-056). A `name` that matches no unit SHALL
  refuse `missing_declaration`/`missing-name` naming the member and its typed
  member path. A variant case `payloadType` of a unit or of `Clock` is read by QSpec FR-154's
  variant payload row as every payload type is (FR-056).
- A unit-typed member SHALL carry no default. A package constraint whose
  `appliesTo` is the unit (`ix://quire/unit/<name>` or an alias of it) SHALL admit the keywords
  `min` and `max`, `operands.value` an exact rational number of the unit written as a canonical
  rational string, and the constrained position's domain SHALL be the quantities of the unit
  from `min` to `max`. A literal of the position outside that interval SHALL refuse
  `ill_typed`/`type-mismatch` at the literal, and an operation whose postcondition does not
  establish the interval for a written position is reported by the invariant check as for an
  `Int[lower, upper]` position. Any other keyword on a unit refuses
  `invalid_model_binding`/`malformed-declaration` at the constraint. A unit-typed member
  without a constraint has every quantity of the unit as its domain. A `reference` `target` of a
  unit refuses `invalid_model_binding`/`malformed-declaration`, because a quantity has no
  identity to refer to.

### Timed facet gate

QSpec's shared grammar admits the timed forms (`time-decl`, `time-invariant`,
`urgent-decl` and the `Clock` type) when the package selects
`quire.temporal.complete/v1`, and gives a recognised form whose facet is absent
a located `unsupported_construct` (QSpec `temporal-complete.md`). The facet is
a header-selectable layer that requires `quire.model.complete/v1` (FR-110,
FR-361 item 18). A unit **selects the timed facet** when at least one of its
header `profile` declarations names `quire.temporal.complete/v1`; it selects
nothing else (FR-361 item 1). S1 and S2 build the forms under every selection
(FR-393), so the refusal is S3's. The rules of the other sections of this
requirement apply to a unit that selects the facet.

The family of a refused construct is `time-declaration` for a `time` member,
`time-invariant` for a `time invariant`, `urgent-declaration` for an `urgent`
declaration and `clock-type` for a `Clock` in a type position. An unqualified
`Clock` in any type position is the clock type (QSpec shared grammar).

- If a unit's header does not select `quire.temporal.complete/v1`, then S3
  SHALL refuse each `time` member, each `time invariant`, each `urgent`
  declaration and each `Clock` that the unit's own source writes in a type
  position (a parameter, result, record field, type alias target, tuple element,
  union payload, or the element of `Option` or a collection type), in source order and every one, with
  `unsupported_construct`/`declaration-form` at the construct's span.
- The refusal SHALL retain the construct's family, the owning declaration (the
  model for a `time` member, `time invariant` or `urgent` declaration; the
  function, operation, record, alias, tuple or union for a `Clock` type position), the
  header layers the unit selected in source order and the required facet
  `quire.temporal.complete/v1`, as FR-361 items 10 and 11 do for the model
  forms.
- A construct S3 refuses under this gate SHALL receive no other
  refusal.
- The gate SHALL NOT reach a `Clock` field of a package-admitted object type,
  which the unit does not write; once the facet is selected "Clock fields and
  the domain package" governs it.

### Limits

- The checker SHALL accept any number of clocks, any constant and any
  period that the exact number types represent. The work of checking timed
  members is charged to the existing S3 work meter.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-230-AC-1 | S2 builds ADR-026 §11's `Rpc` time member as `TimeSourceForm::Dense` with unit `ms`, the two time invariants in source order with bounds `3 ms` and `T`, and `send`'s reset and `reply`'s `x >= 1` guard as timed forms with their source spans, and produces no timed semantic refusal. The package field `Call.x`, typed `ix://quire/native/Clock`, is admitted at intake and reaches S3 as a clock declaration; it is not an S2 source form. With `quire.temporal.complete/v1` selected, S3 checks to `TimeSource::Dense{ms}`, one clock field `Call.x`, two time invariants (`inflight` with `x <= 3`, `Waiting` with `x <= T`), `send`'s reset of `x` to 0, and `reply`'s guard with the one convex part `x >= 1`. For the variant with `time dense unit s`, S2 builds a dense source naming `s`; S3 checks with `reply`'s bound converted to `1/1000`, and its package identity differs from the `ms` unit's. | Test |
| FR-230-AC-2 | Each vector below is grammar-valid: S2 builds its source-level time, reset, constraint, invariant or urgency form with the offending construct's span and returns no timed semantic refusal; clock fields supplied by domain packages are admitted at intake. S3 then refuses each at that span: a second `time` member (`invalid_timed_model`/`duplicate-time-source`); `time dense unit m` (`ill_typed`/`type-mismatch`, with `m` declared on the fixture's non-Time `Length` dimension); `self.n = self.x` with `n: Int` (`invalid_timed_model`/`clock-read-outside-constraint`); `x <= self.y` with both clocks (`invalid_timed_model`/`clock-comparison`); `x >= -1 ms` (`invalid_timed_model`/`negative-clock-constant`); a `Clock` field under `time tick` (`invalid_timed_model`/`clock-without-dense-time`); `self.x = self.x + 1` (`invalid_timed_model`/`clock-write`); a time invariant body `x >= 1` (`invalid_timed_model`/`time-invariant-shape`); `urgent reply` where `reply` guards on `x` (`invalid_timed_model`/`urgent-guard-reads-clock`); `urgent when P` in an untimed model (`missing_declaration`/`missing-name`). Each clock of these vectors is a package-admitted field typed `ix://quire/native/Clock`. | Test |
| FR-230-AC-3 | S2 builds `time tick tick period 1/2 ms unit ms` as a `TimeSourceForm::Tick` naming operation `tick`, exact period `1/2 ms` and unit `ms`; it builds `period 0 ms` as an exact zero quantity and retains an unresolved tick operation name in the form, with no timed semantic refusal. S3 resolves `tick` and checks the valid form to `TimeSource::Tick` with period `1/2`; `period 0 ms` refuses `invalid_model_binding`/`malformed-declaration` at the period, and a tick operation name that resolves to nothing refuses `missing_declaration`/`missing-name` at the operation name. | Test |
| FR-230-AC-4 | S2 builds `(x < 1 or x > 2) and self.ready` as an expression form preserving the ordered operators and exact bounds, and builds `10^40 ms` as an exact time quantity form without a timed semantic refusal. S3 converts the guard to two convex parts, `x < 1` and `x > 2`, each with the data predicate `self.ready`. A model with 64 package-admitted clock fields over a universe of 100 objects has 6,400 clock places and checks the `10^40 ms` constant with no refusal. | Test |
| FR-230-AC-5 | S2 builds each grammar-valid `time dense unit u` form and each quantity literal, including `time dense unit hour`, `time dense unit M::ms` and `3 M::ms`, as timed forms retaining their unit names and spans; it neither resolves a unit nor assigns a dimension. S3 reads the checked package's source and admitted-package unit graph. In a graph whose own source declares base dimension `Time`, root `s` and `ms` (`1/1000` of `s`), `time dense unit ms` and `time dense unit s` check, and `3 ms` read under `unit s` converts to `3/1000`. A unit of a derived dimension `Duration = Time` refuses `ill_typed`/`type-mismatch`, because `Duration` is not the `Time` node. `time dense unit hour` refuses `missing_declaration`/`missing-name` naming `hour`; a graph with no base `Time` refuses `time dense unit ms` `missing_declaration`/`missing-name` naming `Time` (not `ms`); two dimensions named `Time` in the unit's source refuse `ambiguous_declaration`/`ambiguous-name` at assembly, before S3; a unit `degc` of a time dimension with offset `273` refuses `ill_typed`/`type-mismatch`. Two graphs that declare `ms` as `1/1000` and as `1/100` give different package identities. With base dimension `Time`, root `s` and `ms` supplied by the admitted domain package `acme/rpc` (`model M = "acme/rpc" …`) and none in the source, `time dense unit M::ms` and `time dense unit M::s` check and `3 M::ms` under `unit M::s` converts to `3/1000`; a source that declares `unit ms : Time = 1/1000 * M::s;` beside the package's `ms` checks `time dense unit ms` over the source unit, with no ambiguity; a `Time` declared in both the source and the domain package, a dimension `Rate` declared in both the source and package `acme/rpc`, and two admitted domain packages `acme/rpc` and `acme/flow` that each declare a base `Time` remain distinct declarations and check with no refusal. | Test |
| FR-230-AC-7 | A unit whose header selects no `quire.temporal.complete/v1` (a header that selects `quire.model.complete/v1` alone, or no `profile`) and whose own source writes `time dense unit ms;`, a `time invariant`, an `urgent when P;` declaration, a function `f(c: Clock)`, a function `g(): Clock`, a record with a `Clock` field, `type Alias = Clock;` and `Option<Clock>` refuses each, `unsupported_construct`/`declaration-form` at its span, in source order and every one, retaining its family, its owning declaration, the unit's selected layers and the required facet, with no other refusal. | Test |
| FR-230-AC-8 | A header `profile t = "quire.temporal.complete/v1";` resolves at E3 to the temporal layer (FR-110-AC-13), and the unit's S3 check admits its `time` member, `time invariant`, `urgent` declaration and `Clock` in a type position with no `unsupported_construct`/`declaration-form` refusal; the checked state models of AC-1 to AC-5 are produced from units whose header selects the layer. The same unit with the identity `quire.temporal.complete/v1` in no header gets AC-7's refusals. | Test |
| FR-230-AC-9 | With `predicate within(c: Clock, b: Quantity<ms>): Boolean { c <= b }` over a dense model, the precondition `within(self.x, 3 ms)` expands to the atomic constraint `x <= 3` on the clock `Call.x`, a time invariant body `within(self.x, 3 ms)` checks as the bound `x <= 3`, `within(self.x, 3 ms) and within(self.y, 1 ms)` expands to two constraints, and the checked package holds no node for `within`. `within(self.x, self.limit)` refuses `invalid_timed_model`/`clock-bound-not-constant` at the argument, `within(5, 3 ms)` refuses `ill_typed`/`type-mismatch` at `5`, a body `self.n = c` for a clock parameter `c` with `n: Int` refuses `clock-read-outside-constraint`, a body that resets `c` refuses `clock-write`, and `within(self.x, 3 ms)` in an `invariant` clause refuses `clock-read-outside-constraint` at the call. | Test |
| FR-230-AC-10 | With `function pick(a: Clock, b: Clock, left: Boolean): Clock { if left then a else b }`, the guard `pick(self.x, self.y, self.l) >= 1 ms` checks to two convex parts, `x >= 1` with the data predicate `self.l` and `y >= 1` with `not self.l`; a function `peer_clock(o: Call): Clock { o.peer.x }` with `peer: Reference<Call>` over a universe `{c1, c2}` checks `peer_clock(self) <= 2 ms` to the parts `c1.x <= 2` under `self.peer = c1` and `c2.x <= 2` under `self.peer = c2`; a function `g(): Clock { 0 }` refuses `ill_typed`/`type-mismatch` at the body. | Test |
| FR-230-AC-11 | With `record Timer { x: Clock; n: Int }` held by object field `t` of `Call` over a universe of three objects, the model has three clocks, each the place `Call.t.x`, declared in object order; `self.t.x >= 1 ms` in a precondition and `self.t = Timer { x: 0 ms, n: 3 }` in a postcondition check as a guard and a reset of the clock and a write of `n`; `self.u = self.t` refuses `clock-read-outside-constraint` at `self.t`, `self.t.n = 4` checks as a data write, a `Timer` literal in a `let` refuses `clock-write`, and a tuple `(Clock, Int)` field holds the place `Call.p.0`. The checked `Timer` record node lists `n` only. | Test |
| FR-230-AC-12 | With `type Alias = Clock;` and a field `w: Alias`, `w` is a clock under `time dense`. With a field `v: Option<Clock>` the guard `present(self.v) and value(self.v) >= 1 ms` checks to one convex part `v >= 1` carrying the data predicate `present(self.v)`, `value(self.v) >= 1 ms` alone refuses `undefined_expression`/`unproved-presence` at the read, `self.v = 0 ms` records a reset of `v` that makes it present, `self.v = absent` records no reset and drops it, and a time invariant `when present(self.v) { value(self.v) <= 3 ms }` records the bound with the guard. A union case `Armed(Clock)` is the guarded place `Call.state.Armed` and a union case with a payload in the checked package has no payload. | Test |
| FR-230-AC-13 | An operation `reset(c: Clock)` with postcondition `c = 0` and a frame naming `Call.x` has one checked instance per object's place of `Call.x`, each instance naming its place; the same operation with an empty frame refuses `invalid_timed_model`/`clock-write` at the reset; an operation `check(c: Clock)` with the precondition `c >= 1 ms` and no reset has one instance for each clock place of the model. | Test |
| FR-230-AC-14 | A field `t: Timer` of an object type in a `time tick` model and in an untimed model refuses `invalid_timed_model`/`clock-without-dense-time` at its span, and so does a field `ts: Sequence<Timer>`. A function `f(c: Clock)`, a function `g(): Clock { self.x }`, a record with a `Clock` field, `type Alias = Clock;`, `Option<Clock>` and `Sequence<Clock>` declared in a dense model's unit raise no refusal of their own. | Test |
| FR-230-AC-15 | With dimension `Time`, root `s` and `ms` of scale `1/1000`: `record Window { d: Quantity<ms>; n: Int }`, `type Span = Quantity<ms>;`, a field `o: Option<Quantity<ms>>`, a tuple `(Quantity<ms>, Int)`, a union case `Within(Quantity<ms>)`, `Set<Quantity<ms>>`, `Map<Text, Quantity<ms>>`, `function f(d: Quantity<ms>): Quantity<ms>`, `predicate p(d: Span)` and an operation parameter and result of `Quantity<ms>` each check; `Span` and `Quantity<ms>` are one type, every position is typed by the one `scalar_type`/`unit` node of `ms`, and no node is introduced for the alias, the tuple element, the union payload or `Option`. `f(3 s)` refuses `ill_typed`/`type-mismatch`; `record R { d: Quantity<Time> }` refuses `missing_declaration`/`missing-name`. | Test |
| FR-230-AC-16 | A domain package `acme/rpc` with a clock field `Call.x`, an operation `reset` whose parameter is `ix://quire/native/Clock`, an operation `clk` whose result is `ix://quire/native/Clock`, an alias `Stopwatch` whose `target` is `ix://quire/native/Clock` with a field `Call.s` typed `Stopwatch`, an optional clock field `Call.w` and a record `Timer` with a clock field `x` held by `Call.t` checks to the clocks `Call.x`, `Call.s`, `Call.w` (guarded) and `Call.t.x`; `reset`'s postcondition `c = 0` with a frame naming `Call.x` checks as in FR-230-AC-13. A package sequence type whose `items` is `ix://quire/native/Clock`, a map type whose `values` is `ix://quire/native/Clock` and a field `Call.q` typed `ix://quire/native/Clock` with multiplicity `{0, 3, true, false}` check to the clock collections of FR-230-AC-19. | Test |
| FR-230-AC-17 | A domain package `acme/rpc` with a unit `ms` and an alias `D` whose `target` is `ix://quire/unit/acme/rpc/ms`, a sequence type with that `items`, a map type with that `values`, a field, an operation parameter and an operation result typed by it checks to `Quantity<M::ms>` at each position (with `model M = "acme/rpc"`); the same alias, `items` or `values` naming `ix://quire/unit/ghost` refuses `missing_declaration`/`missing-name` naming the member and its path (`TypeDefinition.target`, `TypeDefinition.items`, `TypeDefinition.values`). | Test |
| FR-230-AC-18 | With the domain packages `acme/rpc` (alias `R`, base dimension `Time`, root `s`, unit `ms` of `1/1000` of `s`) and `acme/flow` (alias `F`, base dimension `Time`, root `sec`) both selected and a source that also declares `dimension Time; unit s : Time = 1;` and no dimension equivalence: `time dense unit R::ms` checks with the `acme/rpc` `Time` as the model's time dimension and `x <= 3 R::ms` and `period 1/2 R::s` check; `time dense unit F::sec` checks with the `acme/flow` `Time` as the time dimension, and the source `time dense unit s` checks with the source `Time` as the time dimension; the three models have three different package identities. Under `time dense unit R::ms`, `x <= 3 F::sec` and `x <= 3 s` refuse `ill_typed`/`type-mismatch` at the constant; under `time tick O period 1 F::sec unit R::ms` the period refuses `ill_typed`/`type-mismatch`; a unit `len` of a base dimension `Length` and a unit of a derived `Duration = R::Time` each still refuse `time dense unit` with `ill_typed`/`type-mismatch`. | Test |
| FR-230-AC-19 | Over a dense model whose object type `Call` has `ts: Sequence<Timer>[0, 3]` (`Timer { x: Clock; n: Int }`), `b: Bag<Clock>[0, 2]`, `s: Set<Clock>`, `m: Map<Text, Timer>[0, 2]`, `o: Option<Sequence<Clock>[0, 2]>` and `q: Sequence<Option<Clock>>[0, 2]`, the checker lists the places `Call.ts.[slot].x`, `Call.b.[slot]`, `Call.s.[slot]`, `Call.m.[slot].x`, `Call.o.[slot]` (guarded by `present(o)`) and `Call.q.[slot]` (guarded by `present` of the element), each with its `CollectionShape`; the static clock dimension of one object is `3 + 2 + 0 + 2 + 2 + 2` (the unbounded `s` adds no static bound, `max: None`); `s` holds at most one element and `b` one for each occurrence; `ts` is typed by the `collection_bounds` node over the sequence of the `Timer` data node listing `n` only; `m: Map<Timer, Int>` refuses `ill_typed`/`operator-ineligible` at the key type. | Test |
| FR-230-AC-20 | `self.ts = filter(e in self.ts: e.n > 1)` checks with the provenance `Carried` from each surviving slot; `self.ts = flatten(sequence[sequence[Timer { x: 0 ms, n: 9 }], self.ts])` checks with `Fresh { x = 0 }` at slot 0 and `Carried` from slot `j` at slot `j + 1`; `self.st = set[Timer { x: 0 ms, n: 1 }, Timer { x: 5 ms, n: 1 }]` records one `Fresh { x = 0 }` occurrence, the first; `self.ts = sequence[Timer { x: self.u, n: 1 }]` (a clock read as a component) and `self.ts = sequence[Timer { x: self.ts[0].x, n: 1 }]` refuse `invalid_timed_model`/`clock-write` at the occurrence; `self.ts` written in an `invariant` clause or a `let` refuses `clock-write`; a frame that does not name `ts` leaves it unchanged. | Test |
| FR-230-AC-21 | `self.ts[1].x = 0 ms` checks as a reset of slot 1 and refuses `undefined_expression`/`unproved-presence` at `1` unless the precondition establishes `1 < size(self.ts)`; `self.m[k].x = 0 ms` resets by key and `self.m[k] = Timer { x: 0 ms, n: 1 }` replaces the entry with a fresh one; `forall(e in self.ts: e.n > 1 implies e.x = 0 ms)` records a reset with the data predicate `e.n > 1`; the guard `forall(e in self.ts: e.x <= 3 ms)` records one entry for each size 0 to 3 with the conjunction over the live slots (true for size 0); `exists(e in self.ts: e.n = 2 and e.x >= 1 ms)` records one convex part for each slot with the data predicate "holds the slot" and `n = 2`; a time invariant body `forall(e in self.ts: e.x <= 3 ms)` records `x <= 3` on each live slot and a body `exists(...)` refuses `time-invariant-shape`; `self.a = self.b`, `contains(self.ts, t)` and `count<Int>(e in self.ts: e.x >= 1 ms)` over clock collections refuse `clock-read-outside-constraint`, while `size(self.ts) > 1` and `e.n` check as data reads. | Test |
| FR-230-AC-22 | `Option<Clock>` is covered in every position: a field, an operation parameter `arm(c: Option<Clock>)` with `c = 0 ms` (one instance for each guarded place, each making its place present), a function result `maybe(o: Timer): Option<Clock>` with the body `o.x` (a place expression) and the body `absent` (an inactive place), an alias target, a record component, a union payload, `Option<Timer>` with `value(self.o).x >= 1 ms` refusing `undefined_expression`/`unproved-presence` unless `present(self.o)` holds, `Sequence<Option<Clock>>` with a slot whose `value` needs both the slot's and the element's presence, and `Option<Sequence<Clock>>` whose slots are live only while the option is present. | Test |
| FR-230-AC-23 | A domain package `acme/rpc` with a sequence type over `ix://quire/native/Clock`, a map type over it, a field `Call.q` typed `ix://quire/native/Clock` with multiplicity `{0, 3, true, false}` (`unique:false`), a field `Call.r` typed `Timer` with `{0, 2, true, false}` and an operation parameter and result of multiplicity `{0, 2, true, false}` checks to the clock collections `Call.q`, `Call.r.[slot].x` and the parameter and result of those shapes, with static dimension `3 + 2` for one `Call`; a model whose collection maximum is `4` instead of `3` has a different package identity. | Test |
| FR-230-AC-24 | With object `Call` holding clocks `x` and `y` and a field `r: Reference<Clock>`, a universe of two calls, and `time dense unit ms`, the precondition `deref(self.r) >= 1 ms` checks to four convex parts, `Call1.x >= 1`, `Call1.y >= 1`, `Call2.x >= 1` and `Call2.y >= 1`, each with the data condition `self.r = ref(place)`; the postcondition `self.r = ref(self.x)` writes the data value of the place `Call.x` of `self` and resets no clock; `deref(self.r) = 0 ms` resets each of the four places and refuses `invalid_timed_model`/`clock-write` at the reset when the frame names only `Call.x`. `ref(self.x) = ref(other.x)` checks as a data comparison and is false for two calls; `deref(self.r)` over `r: Option<Reference<Clock>>` refuses `undefined_expression`/`unproved-presence` at the `deref` without `present(self.r)` in the guard and checks with it. Two states that differ only in clock values have the same state key, and two that differ in `r` have different state keys. A `ref(self.q[0])` for a slot place names slot `0` and still names slot `0` after the collection reorders its elements. | Test |
| FR-230-AC-25 | A constraint `max` `"3"` whose `appliesTo` is the alias `Timeout` of `Clock` checks as the time invariant `x <= 3` on every place typed `Timeout` and on no other clock, guarded by the presence of an `Option<Timeout>` place; the same constraint with `appliesTo` `ix://quire/native/Clock` bounds every clock place of the model by `3`; a `min` `"1"` constraint refuses `invalid_timed_model`/`time-invariant-shape`; `minLength` and `format` constraints and a negative or non-rational `max` operand each refuse `invalid_model_binding`/`malformed-declaration` at the constraint; the same constraint under a `time tick` or untimed unit refuses `clock-without-dense-time`. | Test |
| FR-230-AC-26 | A constraint `min` `"0"` and `max` `"10"` whose `appliesTo` is the alias `Level` of `ix://quire/unit/ms` gives `Level` the domain of the quantities of `ms` from `0` to `10`: `5 ms` checks and `11 ms` refuses `ill_typed`/`type-mismatch` at the literal; a keyword `minLength` on it refuses `invalid_model_binding`/`malformed-declaration`; a package `reference` whose `target` is `ix://quire/unit/ms` refuses `invalid_model_binding`/`malformed-declaration` at `TypeDefinition.target`. | Test |
| FR-230-AC-27 | Over FR-230-AC-18's packages and source with `dimension R::Time ~ F::Time by 1 R::s = 1 F::sec;` declared (root ratio 1), `time dense unit R::ms` checks with `x <= 3 F::sec` and the constant converts to `3000` in `R::ms` (`F::sec` is `1000` of `R::ms` through the equivalence class and its root ratio), a reset of `x` to `2 F::sec` converts to `2000`, and `time tick O period 1 F::sec unit R::ms` checks with period `1000`; `x <= 3 s` over the unrelated source `Time` still refuses `ill_typed`/`type-mismatch` at the constant, and the `len` and `Duration = R::Time` units of AC-18 still refuse `time dense unit` with `ill_typed`/`type-mismatch`, `Duration` having no equivalence. The three models keep three different package identities, and a model's identity differs from that of the same model without the equivalence. | Test |
| FR-230-AC-28 | With FR-230-AC-18's `acme/rpc` and a package `acme/clock` (alias `G`, base dimension `Time`, root `msec`, and `sec` of `1000` of `msec`) selected, and `dimension R::Time ~ G::Time by 1 R::s = 1000 G::msec;` declared (root ratio 1000), `time dense unit R::ms` checks with `x <= 3 G::msec` and the constant converts to `3` in `R::ms` (`1` of `G::msec` is `1/1000` of `R::s`, that is `1` of `R::ms`, so `3 G::msec` is `3` of `R::ms`), a reset of `x` to `2000 G::msec` converts to `2000`, and `time tick O period 1 G::sec unit R::ms` checks with period `1000`; with the declaration written `by 1 R::s = 1 G::msec` (root ratio 1) the same constant `3 G::msec` converts to `3000`, so the model's identity differs between the two declarations; `x <= 3 s` over the unrelated source `Time` still refuses `ill_typed`/`type-mismatch`. | Test |
| FR-230-AC-29 | For each S3-admitted timed state model, the checked package contains exactly one `timed_model`/`timed_model` root and typed dependent nodes for its time source, every clock place, reset, guard, invariant, urgency and clock constraint; `Clock` and clock-bearing aliases/options/parameters/results have no value or type nodes, and data type nodes omit clock components. The root and all dependent timed nodes occur in `semantic_graph.nodes` and therefore the identity projection. | Test |
| FR-230-AC-30 | Changing only the resolved time-source kind, unit, tick operation or exact period changes the corresponding timed-node key and package identity; changing only a clock place's owning model or ordered field/tuple/union/collection-slot path, presence guard or collection bound changes that place key and package identity; changing only a reset target/value, guard term, invariant term, urgency condition/operation, or clock-constraint comparator/bound/unit changes its timed-node key and package identity. Reordering source occurrences without changing canonical semantic order leaves node keys and package identity unchanged. | Test |
| FR-230-AC-31 | I2 admits a complete timed-node subgraph whose root owns each referenced node and whose places and exact time quantities agree with the checked model; it refuses a missing root as `invalid_package`/`missing-timed-model`, a dangling or wrong-kind edge as `invalid_package`/`timed-node-reference`, a duplicate place as `invalid_package`/`duplicate-clock-place`, a noncanonical path as `invalid_package`/`malformed-clock-place`, and a non-time unit or malformed quantity as `ill_typed`/`invalid-time-quantity`, reporting the first failing timed node in ascending node-id digest order. | Test |
| FR-230-AC-32 | A checked timed model reaches S4 and I2 read-back without `TimedModelNotEmitted`; the timed-model root's `model` reference equals the unique checked selected-package semantic node resolved for its source model alias, its source unit reference equals the resolved unit node, and the emitted timed-model nodes reconstruct the same time source, clock places, guards, resets, invariants, urgency and normalized constraints. The reconstructed checked package identity equals the producer's identity. | Test |
| FR-230-AC-33 | The checked timed root's explicit `time_dimension` equals the resolved unit's nominal `dimension_node_id`, and every time-valued quantity uses that dimension or an admitted FR-142 equivalence; names and declaration order do not select it. | Test |
| FR-230-AC-34 | Each clock place carries one ordered `resolved_path` row per path step; rows identify declaring type, member identity/type and collection kind/bounds, with the terminal row identifying `Clock`. I2 validates cardinality, order and joins to the selected declaration without source access; a missing, extra or mismatched row refuses `invalid_package`/`malformed-clock-place`. | Test |
| FR-230-AC-35 | A tick source references a `state`/`operation_anchor`, carries its model explicitly, and the anchor context equals the source and root model; the operation resolves among that model's declarations. A function node or an anchor for another model refuses `invalid_package`/`timed-node-reference`. A source operation name that resolves to no model declaration is refused earlier by FR-342 as `missing_declaration`/`missing-name` at the operation member, before timed-node emission. | Test |
| FR-230-AC-36 | I2 refuses a `tick` timed source with any clock place as `invalid_package`/`clock-without-dense-time` at the offending `clock_place` node's `model` binding pointer `/semantic_graph/nodes/{i}/body/members/0/value/target`, even when the tick operation anchor and every reference are otherwise valid. A valid tick source with no clock places remains admitted. | Test |
| FR-230-AC-37 | I2 requires the dense or tick source unit's `dimension_node_id` to equal the timed root's exact base `Time` `time_dimension` node. A source unit in base `Length`, a derived non-Time dimension or a nominal derived-Time dimension refuses `ill_typed`/`invalid-time-quantity` at `/semantic_graph/nodes/{i}/body/members/1/value/target`; TM-24..TM-29 cover each dimension class for both source kinds. | Test |
| FR-230-AC-38 | For `time M dense unit u` and `time M tick O period p unit u`, S3 resolves model alias `M` against the selected package and retains its alias and unique checked semantic-node `NodeKey` on the checked time source. A package with no semantic node denoted by `M` refuses `missing_declaration`/`missing-name` at `M`; a package with multiple candidate semantic model nodes refuses `ambiguous_declaration`/`ambiguous-name` at `M`, regardless of declaration or graph order. With exactly one candidate, the emitted timed-model root's `model` reference is that node. | Test |
| FR-230-AC-39 | Over `v: Option<Clock>` and `q: Sequence<Option<Clock>>[0, 2]`, `present(self.v)` checks as a `Boolean` operand of `and` and `not`, is recorded as a data predicate of the convex part and never as a clock constraint, and `value(self.v) >= 1 ms` without it refuses `undefined_expression`/`unproved-presence`. | Test |
| FR-230-AC-40 | `self.v = 0 ms` records a make-present of `v` with reset 0; `self.v = absent` records a drop that leaves `v` with no zone constraint, so a time invariant `when present(self.v) { value(self.v) <= 3 ms }` places no bound at a state where `v` was dropped. | Test |
| FR-230-AC-41 | `present(...)` of `Option<Timer>`, of a union case `Armed(Clock)` and of a slot of `q` is read from the snapshot's data, `member` or the collection's size, and those places hold no bit; `v: Option<Clock>` and each `Option<Clock>` element of `q` hold a bit whose initial value takes both values, giving `2` initial states for `v` alone. | Test |
| FR-230-AC-42 | In a model with `v: Option<Clock>`, `q: Sequence<Option<Clock>>[0, 2]`, `o: Option<Timer>` and a union case `Armed(Clock)`, only `v` and the live `Option<Clock>` elements of `q` carry a presence bit, encoded by their `option` shell as `{"type":"option","value":null}` (absent) or `{"type":"option","value":{"type":"tuple","name":"Clock","components":[]}}` (present); `semantic` stays a map; a one-element `q` has one bit and a dropped slot none; the keys of `o`, `Armed`, the slots of `q` and a model with no `Option<Clock>` are unchanged; two states that differ only in one bit have different state keys. | Test |
| FR-230-AC-43 | In FR-230-AC-24's model, `ref(self.x)` of `Call1` and of `Call2` have different place ids, `ref(self.q[0])` and `ref(self.q[1])` have different place ids, `ref(self.m[k])` over a `Map` value refuses `ill_typed`/`type-mismatch`, `ref(self.x) = ref(other.x)` is false for two calls and true for one call, and the id of `Call1.x` is the text `place/v1/<h(owner)>/<h(node key)>/-` while that of `Call1.q[1]` ends `/1`. | Test |
| FR-230-AC-44 | A place id is stored in a field, a parameter, a result, a record component and a `Sequence<Reference<Clock>>` element and reads back equal; `=` over two ids is a data read, `<` refuses `ill_typed`/`operator-ineligible`; the id of `ref(self.q[0])` is unchanged after a reorder of `q`; and a package `reference` instance is QSpec FR-181's `reference` value whose `object_type` is the Clock type's effective id and whose `identity` is the hex of the id text. | Test |
| FR-230-AC-45 | A snapshot `Reference<Clock>` whose identity is not of the `place/v1/...` form, or names an owner object or `clock_place` the package and universe do not contain, refuses `invalid_runtime_input`/`invalid-value`. | Test |
| FR-230-AC-46 | Over `s: Sequence<Clock>[1, 3]`, `b: Bag<Clock>[0, 2]`, `r: Sequence<Timer>[1, 3]` with a snapshot of two `Timer` elements and `st: Sequence<Stopwatch>[0, 2]` (`Stopwatch { x: Clock }`): the initial count of `s` ranges over `[1, 3]` and gives three initial states, `b` over `[0, 2]`, `st` over `[0, 2]`, and `r` takes the count 2 from the snapshot; over `qo: Sequence<Option<Clock>>[0, 2]` an element at initial bit absent is unconstrained and every present initial clock is 0. | Test |
| FR-230-AC-47 | `t: Set<Clock>[0, 5]`, `o: OrderedSet<Clock>[0, 5]`, `so: Set<Option<Clock>>[0, 5]` and a `Set<Stopwatch>[0, 5]` each range over `[0, 1]`; `Set<Clock>[2, 5]` leaves no initial state (`Undecided(NoInitialState)`); a `Map<Text, Clock>[0, 2]` is not pure-clock and takes its entries from the snapshot. | Test |
| FR-230-AC-48 | Over `u: Sequence<Clock>` with no declared maximum, initial states are produced in ascending count from 0 and the analysis stops `incomplete` with `limit-reached` whose `limit` is `max_clocks` once the live clocks exceed it, and it does not return `NotSimulated::RequiresBound`. | Test |
| FR-230-AC-49 | The emitted `clock_place` of `ts.x` below `ts: Sequence<Timer>[0, 3]` is one node for the three places `ts.0.x`, `ts.1.x` and `ts.2.x`, its slot step carries the symbolic element selector of QSpec FR-322-AC-148 and no index, and the place ids of slots 0 and 1 differ only in the final component of `<slots>`; the same holds for `Set`, `Bag`, `OrderedSet` and a nested `Sequence` (two `<slots>` components). | Test |
| FR-230-AC-50 | A checked timed model with a guard is refused `TimedModelNotEmitted` until the guard is written, and is never emitted with an empty `guards` aggregate; a model with no guard emits empty aggregates and reads back (QSpec FR-322-AC-150). | Test |
| FR-230-AC-51 | Over `v: Option<Clock>` and `q: Sequence<Option<Clock>>[0, 2]` on two objects, `present(other.v)`, `present(self.q[i])`, and `present(e)` in the bodies of `forall(e in self.q: ...)` and `exists(e in self.q: ...)` each check as a `Boolean` data predicate resolved through the place identity, are recorded as data predicates of the convex part at the owner and slot, declare no `Option<Boolean>` or other member, and leave every model declaration key as it is without `v`; a `present` with a wrong slot count or owner type is refused as QSpec FR-322-AC-156 states. | Test |

## Dependencies

- ADR-026 §2 CK-1 to CK-9.
- [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md)
  (operations and frames), [FR-104](FR-104-check-state-clauses.md) (state
  predicates), [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md)
  (the sibling `terminal` declaration).
- QSpec FR-142 (dimensions, units and exact unit conversion, read from the
  admitted package) and QSpec FR-205 (exact quantities).
- [FR-361](FR-361-gate-model-constructs-on-the-model-profile-selection.md)
  (items 8, 10 and 11: the gate's stage and retained payload),
  [FR-393](FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md)
  (the S2 forms) and [FR-110](FR-110-resolve-header-profile-selections-at-e3.md) (the header
  resolution that decides selection).
- [FR-088](FR-088-clause-name-and-type-identity.md) (unit identity),
  [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md)
  (the unit graph a unit declares).

## References

- QSpec half: QSpec FR-415 (Linear STD-139) owns the shared grammar of the
  `time` member, `Clock` fields, resets, clock constraints, time invariants
  and `urgent` (ADR-026 OV-7).
- Selection of the timed facet: FR-110-AC-13 and FR-361-AC-14 (the header `profile` that names `quire.temporal.complete/v1`).
- filament-core-data FR-146 (Linear STD-295, agent-ix/filament-core-data#301):
  the `Clock` field type and the `dimensions` and `units` of the admitted
  domain package.
