---
id: TC-1070
title: "Evaluate the expressions of protocol nodes from a checked package"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: verifies
---
# TC-1070: Evaluate the expressions of protocol nodes from a checked package

## Description

Verify that S6a evaluates each expression slot of a protocol node of a checked
package over the sources the caller supplies, that a capture slot returns a
frozen value which later slots read only when the caller supplies it, and that
the two templates of one effect stay apart. Scope: FR-390-AC-1 to FR-390-AC-3, FR-390-AC-23 and FR-390-AC-28 to
FR-390-AC-67. The E-P procedure below verifies FR-390-AC-23 without
changing fixture E or rows 1 to 33.

Catches a retry relation or recovery predicate that recomputes a capture from
the current state, a caller that must supply a source the expression does not
read, a template that reads the other template's binders, and a slot that
returns the value of its neighbour.

Fixture **E** is a domain package `M` and the protocol `Refunds` below. Like
TC-1057's fixture R, every binder a trigger guard, retry relation or recovery
predicate reads is a record value, because those slots bar model state and a
compensation's attempt record type is a record value type (QSpec FR-464,
FR-466). Fixture E requires of `M`:

- `Total = Int[0, 1000]` and `Tally = Int[0, 3]`;
- the object type `Node{total: Total}`, with population `nodes`, and the
  operation `Node::step`;
- the record value types `Receipt{total: Total}`, `Attempt{tally: Tally}`,
  `Flag{ready: Boolean}` and `Tick{ready: Boolean, count: Int[0, 9]}`;
- model alias `M` and profile aliases `P` and `T`.

```
protocol Refunds using P over (view: M::Node)
    on each (opened: M::Tick) when (opened.ready) {
  capture order: M::Total = view.total;
  role Service on M::Node;
  compensate Refund for Main::Applied as (forward: M::Receipt)
      by Service on M::Node::step using T clock "refund-clock" {
    capture target: M::Total = forward.total;
    activate first (trigger: M::Flag)
        when { not trigger.ready and target >= 0 } {
      capture activated: Boolean = trigger.ready;
    }
    within [0,30]; attempts 3 of M::Attempt;
    retry (earlier: M::Attempt, later: M::Attempt) {
      earlier.tally < 3 and later.tally = earlier.tally + 1
      and target >= 0 and not activated
    };
    commit Main::Committed;
    recover (recovered: M::Receipt) { recovered.total = target and not activated };
  }
  compensate Partial for Main::Applied as (partialForward: M::Receipt)
      by Service on M::Node::step using T clock "partial-clock" {
    capture partialTarget: M::Total = partialForward.total;
    activate first (partialTrigger: M::Flag)
        when { not partialTrigger.ready and partialTarget >= 0 } {
      capture partialActivated: Boolean = partialTrigger.ready;
    }
    within [0,30]; attempts 3 of M::Attempt;
    retry (partialEarlier: M::Attempt, partialLater: M::Attempt) {
      partialEarlier.tally < 3 and partialLater.tally = partialEarlier.tally + 1
      and partialTarget >= 0 and not partialActivated
    };
    commit never;
    recover (partialRecovered: M::Receipt) {
      partialRecovered.total = partialTarget and not partialActivated
    };
  }
  run sequence Main {
    attempt Tried by Service on M::Node::step contracts []
        as (attempted: M::Flag) { attempted.ready };
    effect Applied of Main::Tried as (applied: M::Receipt) { order >= 0 };
    commit Committed by Service as (committed: M::Flag) { committed.ready };
  }
  finish Closed as (closed: M::Receipt) { true };
}
```

Fixture **L** is a second protocol over the same `M`, which adds the
relationship `owns` between `Node` and `Node`. Its channel carries a record, as
FR-218-AC-24 requires, and every binder its `visible` entry and case guards read
is a record value:

```
protocol Pipeline using P over (view: M::Node) on origin {
  role Seller on M::Node;
  role Buyer on M::Node;
  relationship Owns = M::owns;
  channel Pay from Seller to Buyer carries M::Flag
      ordering unordered delivery at-least-once capacity 2 overflow block;
  run sequence Main {
    send Sent via Pay as (sent: M::Flag) { true };
    receive Got via Pay of Main::Sent as (got: M::Flag)
        related by Owns (view, view) { true };
    choice Decide by Buyer visible (got.ready) {
      case yes when { got.ready } check A using P { true };
      case no when { not got.ready } check B using P { true };
    };
  }
  finish Closed as (closed: M::Flag) { true };
}
```

The package is the checked package QSL compiles from the unit, which carries
the protocol as the v2 protocol nodes FR-383 emits. The observation is one
current snapshot with every population complete: `nodes` holds `n5`
(`total` 5). Record values are written `Type(field value)`: `Receipt(total 5)`,
`Attempt(tally 0)`, `Flag(ready true)`. A slot is written `node.member[index]`,
the node named by its source name (`Refund` is the node of `compensate Refund`,
`Applied` the `effect`, `Committed` the `commit`, `Closed` the `finish`,
`Refunds` the clause). A supply is the list of `binder = value` pairs; a
captured value is supplied under its capture's binder name. The meter is the
default meter.

## Test Procedure

Evaluate one slot per row with the supply the row gives and assert the
`ProtocolSlotEvaluation`'s value. Rows 2, 3 and 6 are capture slots; their
values are what rows 4 to 21 supply as captured values.

| # | Slot | Supply | Expected |
| --- | --- | --- | --- |
| 1 | `Refunds.activation.guard` | `opened = Tick(ready true, count 2)` | true; `result_units` zero by S6a (forwarded scalar) |
| 2 | `Refunds.captures[0]` | `view = n5` | 5, type `M::Total`; `result_units` zero by S6a (forwarded scalar) |
| 3 | `Refund.registration_captures[0]` | `forward = Receipt(total 5)` | 5; `result_units` zero by S6a (forwarded scalar) |
| 4 | `Refund.trigger.guard` | `trigger = Flag(ready false)`, `target = 5` | true |
| 5 | `Refund.trigger.guard` | `trigger = Flag(ready true)`, `target = 5` | false |
| 6 | `Refund.activation_captures[0]` | `trigger = Flag(ready true)` | true; `result_units` zero by S6a (forwarded scalar) |
| 7 | `Refund.activation_captures[0]` | `trigger = Flag(ready false)` | false; `result_units` zero by S6a (forwarded scalar) |
| 8 | `Refund.retry.relation` | `earlier = Attempt(tally 0)`, `later = Attempt(tally 1)`, `target = 5`, `activated = false` | true |
| 9 | `Refund.retry.relation` | row 8 with `activated = true` | false |
| 10 | `Refund.retry.relation` | row 8 with `earlier = Attempt(tally 3)` | false |
| 11 | `Refund.retry.relation` | row 8 with `later = Attempt(tally 0)` | false |
| 12 | `Refund.recovery.predicate` | `recovered = Receipt(total 5)`, `target = 5`, `activated = false` | true |
| 13 | `Refund.recovery.predicate` | row 12 with `recovered = Receipt(total 9)` | false |
| 14 | `Refund.recovery.predicate` | row 12 with `activated = true` | false |
| 15 | `Partial.retry.relation` | `partialEarlier = Attempt(tally 0)`, `partialLater = Attempt(tally 1)`, `partialTarget = 5`, `partialActivated = false` | true |
| 16 | `Partial.retry.relation` | row 15 with `partialActivated = true` | false |
| 17 | `Partial.recovery.predicate` | `partialRecovered = Receipt(total 5)`, `partialTarget = 5`, `partialActivated = false` | true |
| 18 | `Partial.recovery.predicate` | row 17 with `partialRecovered = Receipt(total 9)` | false |
| 19 | `Partial.recovery.predicate` | row 17 with `partialActivated = true` | false |
| 20 | `Applied.constraint` | `order = 5` | true (`order >= 0`) |
| 21 | `Committed.constraint` | `committed = Flag(ready true)` | true; `result_units` zero by S6a (forwarded scalar) |
| 22 | `Closed.constraint` | none | true; `result_units` zero by S6a (literal constant) |
| 23 | the registration capture of row 3 again with `forward = Receipt(total 9)`, then the recovery predicate of row 12 with `recovered = Receipt(total 9)` and `target = 5` | the capture slot gives 9 with `result_units` zero by S6a (forwarded scalar); the recovery predicate gives false at its existing S6a retain points: the total registered earlier (5) is the one it reads, whatever the record now holds |
| 24 | `Refund.recovery.predicate` | `recovered = Receipt(total 9)`, `target = 9`, `activated = false` | true: the evaluator reads the captured value it is given and recomputes nothing |
| 25 | `Refund.retry.relation` | none | refuses `invalid_runtime_input`/`missing-member` naming `target`, the lowest source |
| 26 | each of `Refund.trigger.guard`, `Refund.recovery.predicate`, `Partial.retry.relation`, `Partial.recovery.predicate` | none | refuses `missing-member` naming `target` (`partialTarget`) |
| 27 | `Refund.registration_captures[0]` and `Refund.activation_captures[0]` | none | refuses `missing-member` naming `forward` and `trigger` |
| 28 | `Refund.retry.relation` | row 8's supply and `forward = Receipt(total 5)` | refuses `unknown-member` naming `forward`: the retry relation does not read it |
| 29 | `Got.related.from[0]` (L) | `view = n5` | the reference `n5`; `result_units` zero by S6a (forwarded reference, no retain point) |
| 30 | `Got.related.to[0]` (L) | `view = n5` | the reference `n5`; `result_units` zero by S6a (forwarded reference, no retain point) |
| 31 | `Decide.visible[0]` (L) | `got = Flag(ready true)` | true (`got.ready`); `result_units` zero by S6a (forwarded scalar) |
| 32 | `Decide.cases.guard[0]` and `[1]` (L) | `got = Flag(ready true)` | false (`no`, with `boolean.result-retain`), true (`yes`, `result_units` zero by S6a as a forwarded scalar), in UTF-8 label order |
| 33 | `Decide.cases.guard[0]` (L) | `got = Flag(ready false)` | true (`no`, with `boolean.result-retain`) |

Row 26's four refusals name `target`, the lowest source of each slot (its level
is below the trigger's, the activation capture's and the recovery binder's).
Case indexes name the emitted UTF-8 label order, so `no` is index 0 and `yes`
is index 1 even though the source declares `yes` first.

## Composed-root reader vectors

These rows run over the emitted-and-admitted package of fixtures E, L and E-C
through the reader view and compare with the same call over the checked
package. They leave E and L's rows 1 to 33 unchanged. Rows are labelled `C`
so they do not collide with E-P's rows 34 to 46.

Fixture **E-C** is its own package, separate from fixture E: a domain package
`M` with the record `Mix{ready: Boolean, count: Int[0, 9], note: Option<Int[0,
9]>, items: Sequence<Int[0, 9]>[0, 3]}`, the union `Choice` with the members
`Left(Int[0, 9])` and `Right(Boolean)`, the record `Pair{first: Int[0, 9], second: Boolean}`, `Total = Int[0, 1000]`, the object type
`Node{total: Total}` with population `nodes`, and one protocol `Composed` over
`view: M::Node`. It shares nothing with fixture E's protocol `Refunds`. `Composed` declares the capture
`order: M::Total = view.total` and a run `Main` whose attempt controls, in
declaration order, are:

- `Tried`, with the result binder `(t: M::Mix)`, whose constraint is `order >= 0`;
- `Picked`, with the result binder `(p: M::Mix)`, whose constraint is `order >= 0`;
- one further attempt control per row of the table below, in table order, whose
  only constraint is that row's expression. Its result binder is `(m: M::Mix)`,
  except that of `Composed.case`, which is `(choice: M::Choice)`.

Each attempt control has one constraint slot, as FR-390's slot table gives it.
Shapes are written in checked-term form, not QSL surface syntax; whether each is
expressible in QSL's surface syntax is a check for the code lane. The
constraints of `Tried` and `Picked` both read the one protocol-level capture
`order`, so the application `order >= 0` is a single graph node held by two
slots of two origins (`protocol Composed run Main Picked body` and `protocol Composed run Main Tried body`) (the checker builds equal applications over one binder as
one node), with two `expression` occurrences, 0 at `Tried` and 1 at `Picked` (measured by FR-383-AC-61 / TC-1063 L8h over the `Tried` and `Picked` controls). Each table slot has its own site name and shares no application with another
slot, and its binder `m` or `choice` is shared with no other slot.

| Slot | Shape | Walk positions of interest |
| --- | --- | --- |
| `Composed.let` | `let c = m.count in c + c >= 1` | `m.count` once; the `let` is not a term and the two `c` are `Local` leaves, not visits |
| `Composed.if` | `if m.ready then m.count >= 1 else m.count + 1 >= 2` | `m.count` twice (the then-branch's `>=` and the else-branch's `+`); the conditional is not a term; the else-branch holds applications (`>=`, `+`, `m.count`) when `m.ready` is true |
| `Composed.case` | `case choice of Left(x) => x >= 1; Right(b) => not b` | both arms walked, in emitted arm order; each arm is one application (`x >= 1`, `not b`) over a `Local`; the case is not a term |
| `Composed.deref` | `deref(view).total >= 0` | the dereference, the field read and the comparison |
| `Composed.arith` | `-(m.count) + (m.count / 2) <= 0` | negate, divide, add and the comparison; no coercion position (integer arithmetic over `Int[0, 9]` operands has the unbounded `Int` result type), and with `m.count` 4 the sum is -2 and the comparison holds |
| `Composed.coerce` | `coerce(m.count, Total) <= 9` | `m.count` once; an explicit widening coercion of the `Int[0, 9]` read to `Total`, then the comparison (whether the coercion is an application is an open question) |
| `Composed.option` | `present(m.note) and value(m.note) >= 1` | `m.note` twice |
| `Composed.coll` | `size(m.items) >= 1 and contains(m.items, m.count)` | `m.items` twice |
| `Composed.record` | `Pair(m.count, m.ready).first >= 0` | `m.count` once; the `Pair` constructor of the record declared above, a field read of `first` and the comparison (whether the constructor is an application is an open question for the spec lane) |

| # | Slot | Supply | Expected |
| --- | --- | --- | --- |
| C1 | each slot of AC-28 (fixtures E and L), rows 4 to 22 and 29 to 33 supplies | as in those rows | reader value, outcome category and ordered charge log equal the checked package's (AC-28); `read_node` returns a `NodeRead` for every walk position (AC-29) |
| C2 | `Refund.retry.relation`, handles of `earlier.tally` | row 8's supply | two distinct handles; their `Location` values equal the in-process `Location` of the term in `earlier.tally < 3` and in `earlier.tally + 1`, and differ (AC-31) |
| C3 | `Refund.retry.relation` | `earlier` unavailable, `later = Attempt(tally 1)`, `target = 5`, `activated = false` | halts at the first `earlier.tally` visit; the reader result's `Location` equals the in-process one, that of the term in `earlier.tally < 3` (AC-32) |
| C4 | C2's handles on the package with the two retry-origin `earlier.tally` rows' `Location` values swapped | row 8's supply | the handles read the swapped locations; row 8's value is unchanged (AC-33) |
| C4a | C3 on the package of C4 | C3's supply | the halt reports the location of the term in `earlier.tally + 1` (AC-34) |
| C5 | package with the retry-origin `earlier.tally` row keyed to occurrence 1 removed | any | `ViewFailure::Unsupported`/`MissingLocationProjection` at the `Node` locus of the `earlier.tally` application, retry origin of `Refund`, visit number 1, two visits, one row (AC-35) |
| C5a | the package of C5 | any slot, any meter | no slot has a view; the meter is unchanged (AC-36) |
| C6 | package with a retry-origin `earlier.tally` row inserted directly after the occurrence-1 `earlier.tally` row, keyed to occurrence 1 at `term_ordinal` 1 | any | `MissingLocationProjection` at the `Node` locus of the `earlier.tally` application; no meter charge (AC-37) |
| C6a | package with a retry-origin row added at `term_ordinal` 1 for `order >= 0` of `Applied.constraint` | any | `MissingLocationProjection` at the `Node` locus of that application; no meter charge (AC-38) |
| C7 | package with the occurrence-1 retry-origin `earlier.tally` row's `term_ordinal` changed from 0 to 1 | any | refused at I2 by the QSpec reader, `invalid_package`/`invalid-value` at `/checked_term_data/protocol_locations`; view creation not attempted (AC-39) |
| C7a | package with a retry-origin `earlier.tally` row duplicated | any | refused at I2 with `invalid_package`/`conflicting-definition` at `/checked_term_data/protocol_locations/<index of the duplicate>`; view creation not attempted (AC-40) |
| C8 | package whose `Refund.recovery.predicate` slot descriptor holds the real `CheckedBody` of `Refund.retry.relation`, taken through the checked package's existing API | row 12's supply | the reader value, `NodeRead` values and charge log equal the unaltered view's (AC-30) |
| C9 | Inspection: list FR-322's admitted expression operation identities and FR-400's `NodeKind` variants | n/a | every identity maps to a variant; no identity is left to `MissingOperationProjection` (AC-41) |
| C10 | `Composed.let`, `Composed.if`, `Composed.case` (E-C) | `m = Mix(ready true, count 4, note present 2, items [4])`, `choice = Left(3)`; then the `ready false` and `Right(true)` variants | value and ordered charge log equal the checked package's; each position's `Location` equals the checked package's (AC-42) |
| C10a | `Composed.deref`, `Composed.arith`, `Composed.coerce` (E-C) | `view = n5` and C10's `m` | as C10 (AC-43) |
| C10b | `Composed.option`, `Composed.coll`, `Composed.record` (E-C) | C10's supplies | as C10 (AC-44) |
| C11 | `Composed.let` | as C10 | `m.count` has one visit and one row; the two `c` reads are leaves (AC-45) |
| C11a | `Composed.if`, `Composed.case` | `ready true`, `Left(3)` | the unselected else-branch of `Composed.if` (`>=`, `+`, `m.count`) and the unselected `Right` arm of `Composed.case` (`not b`) hold visits, rows and handles with the checked package's `Location` values (AC-46, AC-47) |
| C11b | `Composed.if` | as C10 | `m.count` has two visits, two rows and two handles with their own `Location` values (AC-48) |
| C11c | each E-C slot on a package with the node ids relabelled (ascending `dependencies` and identities recomputed) so that a slot's ascending-id order differs from its walk order | as C10 | every handle's `Location` is unchanged (AC-49) |
| C11d | `Composed.case` with `Left(3)`, then `Right(true)`; `Composed.let` | as C10 | the handle of each `Local` read (`x`, `b`, `c`) reads the `Location` of its own admitted node, and no row names it (AC-59) |
| C11e | `Composed.let`, `Composed.if`, `Composed.case` | as C10 | the handle of each literal reads the `Location` of its own admitted node, and no row names it (AC-60) |
| C11f | `Composed.let` | as C10 | the handle of the `let` node reads the `Location` of its own admitted node, and no row names it (AC-61) |
| C11g | `Composed.if` | as C10 | the handle of the conditional node reads the `Location` of its own admitted node, and no row names it (AC-62) |
| C11h | `Composed.case` | `Left(3)`, then `Right(true)` | the handle of the case node reads the `Location` of its own admitted node, and no row names it (AC-63) |
| C12 | Inspection of the reader's `read_node`, pairing and descriptor | n/a | root and origin come only from the descriptor; no `NodeRead` is built from a `CheckedBody` (AC-50) |
| C13 | Inspection of the reader's `NodeKind` match | n/a | exhaustive, no wildcard arm (AC-51) |
| C14 | `Refund.retry.relation` (E), handle of `target >= 0` | row 8's supply | the handle reads the retry-origin row (occurrence 1, `term_ordinal` 0), whose `Location` equals the in-process `Location` of that term (AC-52) |
| C14a | `Refund.trigger.guard` (E), handle of `target >= 0` | row 4's supply | the handle reads the trigger-guard-origin row (occurrence 0, `term_ordinal` 0), whose `Location` equals the in-process `Location` of that term (AC-53) |
| C14b | C14 and C14a on the package with those two rows' `Location` values swapped | as C14, C14a | each handle reads the other's location (AC-54) |
| C14c | `Picked` constraint (E-C), handle of `order >= 0` | `order = 5` | the handle reads the `Picked`-origin row (occurrence 1, `term_ordinal` 0) (AC-55) |
| C14d | `Tried` constraint (E-C), handle of `order >= 0` | `order = 5` | the handle reads the `Tried`-origin row (occurrence 0, `term_ordinal` 0) (AC-56) |
| C14e | E-C package with the `Tried`-origin row of `order >= 0` (occurrence 0) removed | any | `MissingLocationProjection` at the `Node` locus of `order >= 0`, `Tried` constraint's origin name, visit number 0, one visit, zero rows, pointer `/checked_term_data/protocol_locations` (AC-57) |
| C14f | the package of C14e | any | the `Picked` constraint's visit is paired and not reported unpaired (AC-58) |
| C15 | every slot of fixtures E, E-C and L, selected on the source route and on the bytes route | the slot's supply | the same triple (node, member, index), root, binders and origin; the bytes descriptor carries no span; `Decide.visible[0]` and `Decide.cases.guard[1]` select distinct origins (AC-65) |
| C16 | a bare package with no supply, with identity rows of another package, with rows naming no identity; a missing node; a missing member | any | the three witness refusals report `CallAdmissionLocus::Package` and the in-process view the `Package` locus of `UnsupportedView`; the node and member refusals report `Node` and `Member`; no witness refusal reports `Node` or `Binder` (AC-66) |
| C17 | a package with unsupported projections of several reasons at once, including the three equality-witness reasons | any | the first reason in FR-390's order is reported; equality-witness reasons at the `Package` locus, no charge (AC-67) |

## Fixture E-P: reference-population provenance extension

E-P is a separate variant of fixture E plus protocol L. It leaves E and L's
existing definitions and rows unchanged. In E-P, `M` additionally declares
`Other{marker: Boolean, next: Option<Reference<Other>>}` with populations
`otherNodes` and `otherSpare`, both
having effective member type `Other` and the same normalized model universe.
It also declares `Elsewhere{marker: Boolean}` and population
`elsewhereNodes` with effective member type `Elsewhere`. The package inputs
include domain package `N`, which declares its own `Other` and population
`externalOthers`; the protocol view and selected model remain `M`.
`M` also declares the navigation relationship `OtherLinks` from `Other` to
`Other`, with source and target ends.
`Receipt` is the E-P variant `Receipt{total: Total, peer: Reference<Other>,
bundle: PeerBundle}`; `PeerBundle` is the record
`PeerBundle{nested: Reference<Other>, optional: Option<Reference<Other>>,
sequence: Sequence<Reference<Other>>[1,1]}`. `Node`, `nodes`, `Refunds` and
`Pipeline` otherwise retain the definitions in E and L. The extension does not
make `Other` a direct protocol context. `forward: Receipt` remains the source
of `Refund.registration_captures[0]`, whose initializer reads only
`forward.total`.

The test harness constructs occurrence metadata with the checked population
declaration node keys obtained from the E-P checked package. Paths are
`quire_semantic_value::admission::PathStep` values, relative to the supplied
value root. The concrete path coverage is:

| Supplied value occurrence | Path | Population declaration |
| --- | --- | --- |
| `view = n5` for `Got.related.from[0]` | `[]` | `nodes` |
| `forward.peer` | `[Field("peer")]` | `otherSpare` |
| `forward.bundle.nested` | `[Field("bundle"), Field("nested")]` | `otherNodes` |
| `forward.bundle.optional = present(reference)` | `[Field("bundle"), Field("optional"), OptionValue]` | `otherSpare` |
| `forward.bundle.sequence[0]` | `[Field("bundle"), Field("sequence"), Index(0)]` | `otherNodes` |

Every `Receipt` reference has an `ObjectReference` value for the same model
universe and effective type `Other`; `otherNodes` and `otherSpare` therefore
cannot be distinguished by that value. The `view = n5` root reference is a
`Node` reference attributed to `nodes`. The occurrence metadata's `population`
is the exact checked declaration node, and is the only supplied-population
provenance. The observation is admitted through FR-106 for the same E-P view
and selected model. For rows 34 to 41, `otherNodes` is complete and contains
objects keyed `ghost`, `nested-1` and `sequence-1`. Rows 42 to 46 specify their
own `otherNodes` contents and completeness. The `otherSpare` state varies per row:
complete with `ghost` and `option-1`; explicitly listed empty and incomplete;
explicitly listed, complete and empty; or unlisted.
All `Other` objects carry `next` absent unless a row gives it a reference.
Each listed `OtherLinks` extent is complete; absent instances mean an empty
extent. An instance end's population label maps to its exact checked
population declaration in the admitted model.
All rows evaluate one slot per call with a fresh default meter unless stated
otherwise.

| # | Slot / case | Supplied value and occurrence metadata | Observation state | Expected |
| --- | --- | --- | --- | --- |
| 34 | `Got.related.from[0]` (L), root reference | `view = n5`; path `[]` maps to checked declaration `nodes` | `nodes` complete with `n5` | Reference `n5`; forwarded-reference `result_units` zero by S6a |
| 35 | `Refund.registration_captures[0]`, complete nested references | `forward = Receipt(total 5, peer ghost, bundle PeerBundle(nested nested-1, optional present option-1, sequence [sequence-1]))`; metadata has the four `forward` path/declaration rows above | `nodes`, `otherNodes` and `otherSpare` complete; all named objects present in their attributed populations | Evaluated value 5; admission completes despite the initializer reading only `forward.total` |
| 36 | same slot and supplied value as row 35, incomplete seeded population with a matching key elsewhere | `forward.peer` path maps to `otherSpare`; remaining `forward` paths retain row 35 metadata | `otherSpare` is explicitly listed empty and incomplete; `otherNodes` is complete and contains `ghost` | `Incomplete`, `incomplete_population`/`incomplete-scope` naming the exact `otherSpare` declaration node; no charge |
| 37 | same slot and supplied value as row 35, unlisted seeded population | `forward.peer` path maps to `otherSpare`; remaining `forward` paths retain row 35 metadata | `otherSpare` is unlisted; `otherNodes` is complete and contains `ghost` | `Incomplete`, `incomplete_population`/`incomplete-scope` naming the exact `otherSpare` declaration node from the closed manifest; no charge |
| 38 | same slot and supplied value as row 35, complete empty target population | all four `forward` paths retain row 35 metadata | `otherSpare` is explicitly listed, complete and empty; `otherNodes` is complete and contains `ghost` | Refused, `dangling_reference`/`absent-target-in-complete-population` at `forward.peer`, naming `ghost` and `otherSpare`; no charge |
| 39 | same slot and supplied value as row 35, all targets present in attributed populations | all four `forward` paths retain row 35 metadata | `otherSpare` is complete and contains `ghost` and `option-1`; other targets are in `otherNodes`; `elsewhereNodes` is incomplete and unreferenced | Evaluated value 5 |
| 40 | same slot and value as row 39, foreign model declaration | replace `forward.peer`'s population key with `N::externalOthers` | admitted authority remains bound to `M` | Refused `invalid_model_binding`/`wrong-model-selection`, retaining the existing admission record; no charge |
| 41 | same slot and value as row 39, wrong effective member type | replace `forward.peer`'s population key with `M::elsewhereNodes` | `elsewhereNodes` is a declaration of selected model `M` | Refused `invalid_runtime_input`/`wrong-value-kind` at `forward.peer`; no charge |
| 42 | same slot as row 35, recursive completeness from a target object's reference field | all four supplied `forward` references explicitly map to `otherSpare`; `otherSpare::ghost.next` names `field-child` in `otherNodes` | `otherSpare` is complete with all four supplied keys; `otherNodes` is explicitly listed incomplete and contains `field-child`; `OtherLinks` is complete and empty | `Incomplete`, `incomplete_population`/`incomplete-scope` naming `otherNodes`' exact declaration node; no charge |
| 43 | same slot as row 35, recursive completeness from a relationship end | all four supplied `forward` references explicitly map to `otherSpare`; `otherSpare::ghost.next` is absent; `OtherLinks` instance 0 has source `otherSpare::ghost` and target `otherNodes::edge-child` | `otherSpare` is complete with all four supplied keys; `otherNodes` is explicitly listed incomplete and contains `edge-child`; `OtherLinks` is complete | `Incomplete`, `incomplete_population`/`incomplete-scope` naming `otherNodes`' exact declaration node; no charge |
| 44 | same slot as row 35, check-8 ordering across a reference field and relationship end | all four supplied `forward` references map to `otherSpare`; `otherSpare::ghost.next` names absent `field-ghost` in `otherNodes`; `OtherLinks` instance 0 has source `otherSpare::ghost` and target absent `edge-ghost` in `otherNodes` | `otherSpare` and `otherNodes` are complete; both referenced target keys are absent | Refused by check 8 as `dangling_reference`/`absent-target-in-complete-population` for `field-ghost` at `otherSpare::ghost.next`, before the relationship-end defect |
| 45 | same slot as row 35, relationship-end closure defect after field closure succeeds | all four supplied `forward` references map to `otherSpare`; `otherSpare::ghost.next` names present `field-child` in `otherNodes`; `OtherLinks` instance 0 has source `otherSpare::ghost` and target absent `edge-ghost` in `otherNodes` | `otherSpare` and `otherNodes` are complete; `field-child` is present and `edge-ghost` is absent | Refused by check 8 as `dangling_reference`/`absent-target-in-complete-population`, naming `OtherLinks`, instance 0, end `target`, `edge-ghost` and `otherNodes` |
| 46 | same slot as row 35, recursive closure complete | all four supplied `forward` references map to `otherSpare`; `otherSpare::ghost.next` names `field-child` in `otherNodes`; `OtherLinks` instance 0 has source `otherSpare::ghost` and target `otherNodes::edge-child` | `otherSpare` and `otherNodes` are complete; both `field-child` and `edge-child` are present | Evaluated value 5 |

For each path/declaration row above, the harness separately removes that
occurrence entry and duplicates that occurrence entry. The
mutation matrix is:

| Mutation | Exercised paths | Expected |
| --- | --- | --- |
| Omit occurrence entry | `view: []`; `forward: [Field("peer")]`; `forward: [Field("bundle"), Field("nested")]`; `forward: [Field("bundle"), Field("optional"), OptionValue]`; `forward: [Field("bundle"), Field("sequence"), Index(0)]` | `invalid_runtime_input`/`missing-member`, naming the listed binder and preserving the missing typed path |
| Duplicate occurrence entry | the same five binder/path pairs, each duplicated in turn | `invalid_runtime_input`/`duplicate-member`, naming the listed binder and preserving the later typed path |
| Add entry for scalar `forward.total` | `[Field("total")]` | `invalid_runtime_input`/`unknown-member`, naming `forward` and the extra typed path |
| Add entry for a nonexistent nested field | `[Field("bundle"), Field("absent")]` | `invalid_runtime_input`/`unknown-member`, naming `forward` and the absent typed path |
| Replace a same-model population declaration | each of the four `Other` reference paths, swapping `otherNodes` and `otherSpare` | Accepted as provenance for that declared population; subsequent completeness and dangling outcomes follow the population named by the replacement |
| Replace with `M::elsewhereNodes` | `[Field("peer")]` | `invalid_runtime_input`/`wrong-value-kind` because `Elsewhere` does not conform to `Reference<Other>` |
| Replace with `N::externalOthers` | `[Field("peer")]` | `invalid_model_binding`/`wrong-model-selection` because the declaration is outside selected model `M` |

Each missing, duplicate, extra/non-reference or incompatible entry returns the
corresponding existing `invalid_runtime_input` cause (`missing-member`,
`duplicate-member`, `unknown-member` or `wrong-value-kind`) at the binder and
exact typed occurrence path, except a declaration outside the selected model,
which returns `invalid_model_binding`/`wrong-model-selection`.
Rows 42 and 43 prove that a reference field in a target object and a parsed
relationship end extend check 7's required-population closure independently.
Rows 44 and 45 prove check 8's existing walk order: object reference fields
are checked before relationship ends, and an end refusal names the exact
relationship, instance, end and population declaration. Row 46 completes the
recursive closure and evaluates successfully.

These cases run with a present refused observation result as well; source
selection, value shape/type and occurrence errors retain their earlier
precedence, and every refusal leaves the meter unchanged. A second incomplete
population that is not named by any occurrence or recursive closure leaves
row 39 admissible, as `elsewhereNodes` demonstrates. A reference in a target object's field or in a parsed
relationship end extends the FR-106 check-7 closure; after all required
populations are complete, a missing target in a complete population returns
the existing check-8 dangling-reference record in FR-106 walk order.

## Expected Results

Every slot returns the value its row gives from the supplied values alone. A
capture slot returns its initializer's value and nothing keeps it: rows 23 and
24 show that a later slot reads the captured value the caller supplies, 5 or 9,
and never the record the capture once read. A supply holds exactly the
binders and captured values the expression reads, so rows 25 to 28 name a
source or refuse a supplied value the expression does not read. The two
templates read their own binders only.
