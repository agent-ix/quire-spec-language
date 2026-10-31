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
the two templates of one effect stay apart. Scope: FR-390-AC-1 to FR-390-AC-3
and FR-390-AC-23. The E-P procedure below verifies FR-390-AC-23 without
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

## Fixture E-P: reference-population provenance extension

E-P is a separate variant of fixture E plus protocol L. It leaves E and L's
existing definitions and rows unchanged. In E-P, `M` additionally declares
`Other{marker: Boolean}` with populations `otherNodes` and `otherSpare`, both
having effective member type `Other` and the same normalized model universe.
It also declares `Elsewhere{marker: Boolean}` and population
`elsewhereNodes` with effective member type `Elsewhere`. The package inputs
include domain package `N`, which declares its own `Other` and population
`externalOthers`; the protocol view and selected model remain `M`.
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
and selected model. `otherNodes` is complete and contains objects keyed
`ghost`, `nested-1` and `sequence-1`. The `otherSpare` state varies per row:
complete with `ghost` and `option-1`; explicitly listed empty and incomplete;
explicitly listed, complete and empty; or unlisted.
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

For each path/declaration row above, the harness separately removes that
occurrence entry and duplicates that occurrence entry. The
mutation matrix is:

| Mutation | Exercised paths | Expected |
| --- | --- | --- |
| Omit occurrence entry | `view: []`; `forward: [Field("peer")]`; `forward: [Field("bundle"), Field("nested")]`; `forward: [Field("bundle"), Field("optional"), OptionValue]`; `forward: [Field("bundle"), Field("sequence"), Index(0)]` | `invalid_runtime_input`/`missing-member`, naming the listed binder and preserving the missing typed path |
| Duplicate occurrence entry | the same five binder/path pairs, each duplicated in turn | `invalid_runtime_input`/`duplicate-member`, naming the listed binder and preserving the later typed path |
| Add entry for scalar `forward.total` | `[Field("total")]` | `invalid_runtime_input`/`unknown-member`, naming `forward` and the extra typed path |
| Replace a same-model population declaration | each of the four `Other` reference paths, swapping `otherNodes` and `otherSpare` | Accepted as provenance for that declared population; subsequent completeness and dangling outcomes follow the population named by the replacement |
| Replace with `M::elsewhereNodes` | `[Field("peer")]` | `invalid_runtime_input`/`wrong-value-kind` because `Elsewhere` does not conform to `Reference<Other>` |
| Replace with `N::externalOthers` | `[Field("peer")]` | `invalid_model_binding`/`wrong-model-selection` because the declaration is outside selected model `M` |

Each missing, duplicate, extra/non-reference or incompatible entry returns the
corresponding existing `invalid_runtime_input` cause (`missing-member`,
`duplicate-member`, `unknown-member` or `wrong-value-kind`) at the binder and
exact typed occurrence path, except a declaration outside the selected model,
which returns `invalid_model_binding`/`wrong-model-selection`.
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
