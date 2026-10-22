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
kernel.

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
    pub clocks: Vec<ClockField>,            // (object type, field), declaration order
    pub invariants: Vec<TimeInvariant>,     // when P { x <= c, ... }
    pub urgency: Vec<Urgency>,              // When(P) | Operation(OperationId)
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
package supplies is named by the package identity (QSpec FR-322 "Model-owned
members" step 4): the source writes it through the model alias, `time dense
unit M::ms` after `model M = "acme/rpc" …`, and a source `ms` and the
package's `ms` are two units. A dimension keeps its single-segment name, so
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
invariants and urgency" to it. A unit that writes a clock does so through
these package-admitted fields; the rules of this requirement also need the
facet to be selectable (Timed facet gate).

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

Not yet specified: the clock rules for a `Clock` parameter, result or
source-declared record field in a unit that selects the facet. This requirement
states the rules for a clock field of a model object type only.

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
| FR-230-AC-5 | In a graph whose own source declares base dimension `Time`, root `s` and `ms` (`1/1000` of `s`), `time dense unit ms` and `time dense unit s` check, and a quantity `3 ms` read under `unit s` converts to `3/1000`. A unit of a derived dimension `Duration = Time` refuses `ill_typed`/`type-mismatch`, because `Duration` is not the `Time` node. `time dense unit hour` refuses `missing_declaration`/`missing-name` naming `hour`; a graph with no base `Time` refuses `time dense unit ms` `missing_declaration`/`missing-name` naming `Time` (and not `ms`); two dimensions named `Time` in the unit's source refuse `ambiguous_declaration`/`ambiguous-name` at assembly; a unit `degc` of a time dimension with offset `273` refuses `ill_typed`/`type-mismatch`. Two graphs that declare `ms` as `1/1000` and as `1/100` give different package identities. With base dimension `Time`, root `s` and `ms` supplied by the admitted domain package `acme/rpc` (`model M = "acme/rpc" …`) and none in the source, `time dense unit M::ms` and `time dense unit M::s` check and `3 M::ms` under `unit M::s` converts to `3/1000`; a source that declares `unit ms : Time = 1/1000 * M::s;` beside the package's `ms` checks `time dense unit ms` over the source unit, with no ambiguity; a `Time` declared in both the source and the domain package refuses `conflicting_declaration`/`duplicate-declaration` at the source declaration's span, naming both, and so does a dimension `Rate` declared in both. | Test (TC-685) |
| FR-230-AC-7 | A unit whose header selects no `quire.temporal.complete/v1` and whose own source writes `time dense unit ms;`, a `time invariant`, an `urgent when P;` declaration, a function `f(c: Clock)`, a function `g(): Clock`, a record with a `Clock` field, `type Alias = Clock;` and `Option<Clock>` refuses each, `unsupported_construct`/`declaration-form` at its span, in source order and every one, retaining its family, its owning declaration, the unit's selected layers and the required facet, with no other refusal. | Test (TC-685) |
| FR-230-AC-8 | A header that selects `quire.temporal.complete/v1` is refused `unknown_profile`/`unsupported-selection` at E3 with no S3 result, because no catalog holds the identity. | Test (TC-685) |

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
