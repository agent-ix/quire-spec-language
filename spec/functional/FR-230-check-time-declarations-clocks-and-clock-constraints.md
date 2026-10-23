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
and the argument of `Option`.

## Use case

A verification operator models a client that times out after 3 ms and a
server that replies between 1 ms and 3 ms after a request. They declare
`time dense unit ms`, give the call object a clock, reset it on `send`,
guard `reply` with `x >= 1 ms` and bound waiting with time invariants. A
mistake such as copying the clock into an integer field, or comparing two
clocks, is refused at check time with a span.

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
    pub path: Vec<PlaceStep>,               // field names, tuple indexes and union cases down to the Clock
    pub guard: Option<PlaceGuard>,          // Some under an Option, `?` field or union case: the presence test
}

pub struct ClockConstraint { pub clock: ClockRef, pub op: ClockOp, pub bound: ExactRational }
pub enum ClockOp { Lt, Le, Eq, Ge, Gt }
```

`CheckedStateModel::time: Option<TimedModel>`, each operation's checked
guard as data predicates and atomic clock constraints, and each operation's
resets as `(ClockRef, ExactRational)` pairs. A typed `CheckRefusal` with a
span on refusal.

## Behavior

### The `time` member

- The checker SHALL admit at most one `time` member per state model; a
  second SHALL refuse `invalid_timed_model`/`duplicate-time-source` at
  its span, naming the first (QSpec FR-415).
- The checker SHALL resolve the unit `u` as a unit of QSpec FR-142's time
  dimension (next section), and SHALL refuse a unit of another dimension with
  `ill_typed`/`type-mismatch` at the unit's span.
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
and a source `ms` and the package's `ms` are two units. A dimension keeps its single-segment name, so
the package's `Time` is found by the name `Time`.

- The **time dimension** SHALL be the base dimension node declared with the
  name `Time` in that graph, by the unit's source or by the admitted domain
  package, and nothing else. The **time units** SHALL be
  the units typed by that node. A derived dimension whose FR-142 vector is
  `Time` with exponent 1 (for example `Duration = Time`) is a different
  dimension node, and FR-142 converts no unit of one node to a unit of
  another, so its units are not time units. For example, a graph with the
  root `s` and a unit `ms` whose edge to `s` has scale `1/1000` makes `ms`
  and `s` time units; any zero-offset root and any edge chain that composes
  to the same scale do as well.
- The checker SHALL read `time dense unit u`, `time tick O period p unit u`
  and a quantity such as `3 ms` in this order: the graph has a time base,
  then `u` is found by name, then `u` is typed by the time dimension, then
  `u` has a zero composed offset. It SHALL convert a quantity exactly by
  composing the unit's edges (QSpec FR-142).
- If the graph has no base dimension named `Time`, the checker SHALL refuse
  a `time` member or a time quantity `missing_declaration`/`missing-name` at
  its span, naming `Time`.
- If a QSL unit's own source declares two dimensions named `Time`, the
  assembler refuses it first, `ambiguous_declaration`/`ambiguous-name`
  (FR-091), and S3 never sees it. If the unit's source and the admitted domain
  package both declare a dimension of one name, `Time` or any other, the
  checker SHALL refuse `conflicting_declaration`/`duplicate-declaration` at
  the source declaration's span, naming both declarations (QSpec FR-142
  "Namespaces"); the same holds for two admitted domain packages, at the
  later package's selection row.
- If the graph has no unit named `u`, the checker SHALL refuse
  `missing_declaration`/`missing-name` at the unit's span, naming `u`.
- If `u` is a unit of another dimension, the checker SHALL refuse
  `ill_typed`/`type-mismatch` at the unit's span (FR-230-AC-2).
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
- **Checked package.** The checked package SHALL hold no node for the clock
  type, an alias of it, `Option<Clock>`, a clock parameter or a clock result. A
  record or tuple node SHALL list its data components only, and a union case
  whose payload is a `Clock` SHALL be a case without payload whose clock is the
  guarded place of that case. A model field of clock-bearing type SHALL be
  read by (declaring node, field path) as FR-322 "Model-owned members" states.
- **Operation parameters.** An operation with a `Clock` parameter SHALL have
  one checked instance for each place of the parameter's range, and each
  instance's identity SHALL name the place. The range SHALL be the places of
  the model; when the postcondition resets the parameter it SHALL be the places
  of the clock fields the operation's frame (FR-103) names, and a reset of a
  clock the frame does not name SHALL refuse `invalid_timed_model`/
  `clock-write` at the reset.
- **Collections.** The checker SHALL refuse a clock-bearing type as the element
  of a `Set`, `Bag`, `Sequence`, `OrderedSet` or map, and a field of
  clock-bearing type whose multiplicity is not `[1, 1]`,
  `invalid_timed_model`/`clock-in-collection` at the type form or field,
  because the number of clocks would vary with the state.
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
  sequence `items`, map `values` and variant `payloadType` whose `typeRef` is
  `ix://quire/unit/<name>` SHALL read as `Quantity<u>` at that position, the
  alias, sequence or map as the alias, `Sequence<Quantity<u>>` or
  `Map<Text, Quantity<u>>` of it (FR-056). A `name` that matches no unit SHALL
  refuse `missing_declaration`/`missing-name` naming the member and its typed
  member path.
- A unit-typed member SHALL carry no default and no constraint: its domain is
  every quantity of the unit.

### Timed facet gate

QSpec's shared grammar admits the timed forms (`time-decl`, `time-invariant`,
`urgent-decl` and the `Clock` type) when the package selects the complete
facets, and gives a recognised form whose facet is absent a located
`unsupported_construct`. QSL reads the facet as `quire.temporal.complete/v1`;
the grammar names no single facet for these forms, so the identity is QSL's
reading. S1 and S2 build the forms under every selection (FR-393), so the
refusal is S3's.

No QSL catalog holds `quire.temporal.complete/v1` today (FR-361 item 8), so E3
refuses a header that selects it, `unknown_profile`/`unsupported-selection`
(FR-110), before S3 runs. A unit that reaches S3 therefore lacks the facet, and
the gate below refuses every timed construct it writes. The rules of the other
sections of this requirement apply to a unit that selects the facet, and take
effect when the identity is a selectable profile.

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
| FR-230-AC-1 | ADR-026 §11's `Rpc` unit checks to `TimeSource::Dense{ms}` with one clock field `Call.x`, two time invariants (`inflight` with `x <= 3`, `Waiting` with `x <= T`), `send`'s reset of `x` to 0, and `reply`'s guard with the one convex part `x >= 1`. The same unit with `time dense unit s` checks with `reply`'s bound converted to `1/1000`, and its package identity differs from the `ms` unit's. The clock field `Call.x` is a field of the admitted domain package typed `ix://quire/native/Clock`. | Test (TC-685) |
| FR-230-AC-2 | Refusals, each at its span: a second `time` member (`invalid_timed_model`/`duplicate-time-source`); `time dense unit m` (`ill_typed`/`type-mismatch`); `self.n = self.x` with `n: Int` (`invalid_timed_model`/`clock-read-outside-constraint`); `x <= self.y` with both clocks (`invalid_timed_model`/`clock-comparison`); `x >= -1 ms` (`invalid_timed_model`/`negative-clock-constant`); a `Clock` field under `time tick` (`invalid_timed_model`/`clock-without-dense-time`); `self.x = self.x + 1` (`invalid_timed_model`/`clock-write`); a time invariant body `x >= 1` (`invalid_timed_model`/`time-invariant-shape`); `urgent reply` where `reply` guards on `x` (`invalid_timed_model`/`urgent-guard-reads-clock`); `urgent when P` in an untimed model (`missing_declaration`/`missing-name`). Each clock of these vectors is a package-admitted field typed `ix://quire/native/Clock`. | Test (TC-685) |
| FR-230-AC-3 | `time tick tick period 1/2 ms unit ms` checks to `TimeSource::Tick` with period `1/2`; `period 0 ms` refuses `invalid_model_binding`/`malformed-declaration`; a tick operation name that resolves to nothing refuses `missing_declaration`/`missing-name`. | Test (TC-685) |
| FR-230-AC-4 | A guard `(x < 1 or x > 2) and self.ready` checks to two convex parts, `x < 1` and `x > 2`, each with the data predicate `self.ready`. A model with 64 package-admitted clock fields over a universe of 100 objects and a constant of `10^40 ms` checks with no refusal. | Test (TC-685) |
| FR-230-AC-5 | In a graph whose own source declares base dimension `Time`, root `s` and `ms` (`1/1000` of `s`), `time dense unit ms` and `time dense unit s` check, and a quantity `3 ms` read under `unit s` converts to `3/1000`. A unit of a derived dimension `Duration = Time` refuses `ill_typed`/`type-mismatch`, because `Duration` is not the `Time` node. `time dense unit hour` refuses `missing_declaration`/`missing-name` naming `hour`; a graph with no base `Time` refuses `time dense unit ms` `missing_declaration`/`missing-name` naming `Time` (and not `ms`); two dimensions named `Time` in the unit's source refuse `ambiguous_declaration`/`ambiguous-name` at assembly; a unit `degc` of a time dimension with offset `273` refuses `ill_typed`/`type-mismatch`. Two graphs that declare `ms` as `1/1000` and as `1/100` give different package identities. With base dimension `Time`, root `s` and `ms` supplied by the admitted domain package `acme/rpc` (`model M = "acme/rpc" …`) and none in the source, `time dense unit M::ms` and `time dense unit M::s` check and `3 M::ms` under `unit M::s` converts to `3/1000`; a source that declares `unit ms : Time = 1/1000 * M::s;` beside the package's `ms` checks `time dense unit ms` over the source unit, with no ambiguity; a `Time` declared in both the source and the domain package refuses `conflicting_declaration`/`duplicate-declaration` at the source declaration's span, naming both, and so does a dimension `Rate` declared in both the source and a package `acme/rpc` that declares `Rate`; two admitted domain packages `acme/rpc` and `acme/flow` that both declare a base dimension `Time`, with no source `Time`, refuse `conflicting_declaration`/`duplicate-declaration` at `acme/flow`'s selection row `/lock/model_selections/1`, naming both declarations. | Test (TC-685) |
| FR-230-AC-7 | A unit whose header selects no `quire.temporal.complete/v1` and whose own source writes `time dense unit ms;`, a `time invariant`, an `urgent when P;` declaration, a function `f(c: Clock)`, a function `g(): Clock`, a record with a `Clock` field, `type Alias = Clock;` and `Option<Clock>` refuses each, `unsupported_construct`/`declaration-form` at its span, in source order and every one, retaining its family, its owning declaration, the unit's selected layers and the required facet, with no other refusal. | Test (TC-685) |
| FR-230-AC-8 | A header that selects `quire.temporal.complete/v1` is refused `unknown_profile`/`unsupported-selection` at E3 with no S3 result, because no catalog holds the identity. | Test (TC-685) |
| FR-230-AC-9 | With `predicate within(c: Clock, b: Quantity<ms>): Boolean { c <= b }` over a dense model, the precondition `within(self.x, 3 ms)` expands to the atomic constraint `x <= 3` on the clock `Call.x`, a time invariant body `within(self.x, 3 ms)` checks as the bound `x <= 3`, `within(self.x, 3 ms) and within(self.y, 1 ms)` expands to two constraints, and the checked package holds no node for `within`. `within(self.x, self.limit)` refuses `invalid_timed_model`/`clock-bound-not-constant` at the argument, `within(5, 3 ms)` refuses `ill_typed`/`type-mismatch` at `5`, a body `self.n = c` for a clock parameter `c` with `n: Int` refuses `clock-read-outside-constraint`, a body that resets `c` refuses `clock-write`, and `within(self.x, 3 ms)` in an `invariant` clause refuses `clock-read-outside-constraint` at the call. | Test (TC-685) |
| FR-230-AC-10 | With `function pick(a: Clock, b: Clock, left: Boolean): Clock { if left then a else b }`, the guard `pick(self.x, self.y, self.l) >= 1 ms` checks to two convex parts, `x >= 1` with the data predicate `self.l` and `y >= 1` with `not self.l`; a function `peer_clock(o: Call): Clock { o.peer.x }` with `peer: Reference<Call>` over a universe `{c1, c2}` checks `peer_clock(self) <= 2 ms` to the parts `c1.x <= 2` under `self.peer = c1` and `c2.x <= 2` under `self.peer = c2`; a function `g(): Clock { 0 }` refuses `ill_typed`/`type-mismatch` at the body. | Test (TC-685) |
| FR-230-AC-11 | With `record Timer { x: Clock; n: Int }` held by object field `t` of `Call` over a universe of three objects, the model has three clocks, each the place `Call.t.x`, declared in object order; `self.t.x >= 1 ms` in a precondition and `self.t = Timer { x: 0 ms, n: 3 }` in a postcondition check as a guard and a reset of the clock and a write of `n`; `self.u = self.t` refuses `clock-read-outside-constraint` at `self.t`, `self.t.n = 4` checks as a data write, a `Timer` literal in a `let` refuses `clock-write`, and a tuple `(Clock, Int)` field holds the place `Call.p.0`. The checked `Timer` record node lists `n` only. | Test (TC-685) |
| FR-230-AC-12 | With `type Alias = Clock;` and a field `w: Alias`, `w` is a clock under `time dense`. With a field `v: Option<Clock>` the guard `present(self.v) and value(self.v) >= 1 ms` checks to one convex part `v >= 1` carrying the data predicate `present(self.v)`, `value(self.v) >= 1 ms` alone refuses `undefined_expression`/`unproved-presence` at the read, `self.v = 0 ms` records a reset of `v` that makes it present, `self.v = absent` records no reset and drops it, and a time invariant `when present(self.v) { value(self.v) <= 3 ms }` records the bound with the guard. A union case `Armed(Clock)` is the guarded place `Call.state.Armed` and a union case with a payload in the checked package has no payload. | Test (TC-685) |
| FR-230-AC-13 | An operation `reset(c: Clock)` with postcondition `c = 0` and a frame naming `Call.x` has one checked instance per object's place of `Call.x`, each instance naming its place; the same operation with an empty frame refuses `invalid_timed_model`/`clock-write` at the reset; an operation `check(c: Clock)` with the precondition `c >= 1 ms` and no reset has one instance for each clock place of the model. | Test (TC-685) |
| FR-230-AC-14 | Each refuses at its span with its cause: a field `s: Set<Clock>` and a field `q: Sequence<Timer>` (`invalid_timed_model`/`clock-in-collection`); a field `t: Timer` of an object type in a `time tick` model and in an untimed model (`clock-without-dense-time`); a field of clock-bearing type with multiplicity `[0, 3]` (`clock-in-collection`). A function `f(c: Clock)`, a function `g(): Clock { self.x }`, a record with a `Clock` field, `type Alias = Clock;` and `Option<Clock>` declared in a dense model's unit raise no refusal of their own. | Test (TC-685) |
| FR-230-AC-15 | With dimension `Time`, root `s` and `ms` of scale `1/1000`: `record Window { d: Quantity<ms>; n: Int }`, `type Span = Quantity<ms>;`, a field `o: Option<Quantity<ms>>`, a tuple `(Quantity<ms>, Int)`, a union case `Within(Quantity<ms>)`, `Set<Quantity<ms>>`, `Map<Text, Quantity<ms>>`, `function f(d: Quantity<ms>): Quantity<ms>`, `predicate p(d: Span)` and an operation parameter and result of `Quantity<ms>` each check; `Span` and `Quantity<ms>` are one type, every position is typed by the one `scalar_type`/`unit` node of `ms`, and no node is introduced for the alias, the tuple element, the union payload or `Option`. `f(3 s)` refuses `ill_typed`/`type-mismatch`; `record R { d: Quantity<Time> }` refuses `missing_declaration`/`missing-name`. | Test (TC-685) |
| FR-230-AC-16 | A domain package `acme/rpc` with a clock field `Call.x`, an operation `reset` whose parameter is `ix://quire/native/Clock`, an operation `clk` whose result is `ix://quire/native/Clock`, an alias `Stopwatch` whose `target` is `ix://quire/native/Clock` with a field `Call.s` typed `Stopwatch`, an optional clock field `Call.w` and a record `Timer` with a clock field `x` held by `Call.t` checks to the clocks `Call.x`, `Call.s`, `Call.w` (guarded) and `Call.t.x`; `reset`'s postcondition `c = 0` with a frame naming `Call.x` checks as in FR-230-AC-13. A package clock as a sequence `items` refuses at intake, FR-154's `malformed-declaration`. | Test (TC-685) |
| FR-230-AC-17 | A domain package `acme/rpc` with a unit `ms` and an alias `D` whose `target` is `ix://quire/unit/acme/rpc/ms`, a sequence type with that `items`, a map type with that `values`, a field, an operation parameter and an operation result typed by it checks to `Quantity<M::ms>` at each position (with `model M = "acme/rpc"`); the same alias, `items` or `values` naming `ix://quire/unit/ghost` refuses `missing_declaration`/`missing-name` naming the member and its path (`TypeDefinition.target`, `TypeDefinition.items`, `TypeDefinition.values`). | Test (TC-685) |

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
- Remaining work: the rules of this requirement wait on E3 admitting
  `quire.temporal.complete/v1` as a selectable profile (FR-110, FR-361 item 8).
- filament-core-data FR-146 (Linear STD-295, agent-ix/filament-core-data#301):
  the `Clock` field type and the `dimensions` and `units` of the admitted
  domain package.
