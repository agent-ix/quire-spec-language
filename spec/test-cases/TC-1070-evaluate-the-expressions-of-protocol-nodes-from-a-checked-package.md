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
the two templates of one effect stay apart. Scope: FR-390-AC-1 to FR-390-AC-3.

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
| 1 | `Refunds.activation.guard` | `opened = Tick(ready true, count 2)` | true |
| 2 | `Refunds.captures[0]` | `view = n5` | 5, type `M::Total` |
| 3 | `Refund.registration_captures[0]` | `forward = Receipt(total 5)` | 5 |
| 4 | `Refund.trigger.guard` | `trigger = Flag(ready false)`, `target = 5` | true |
| 5 | `Refund.trigger.guard` | `trigger = Flag(ready true)`, `target = 5` | false |
| 6 | `Refund.activation_captures[0]` | `trigger = Flag(ready true)` | true |
| 7 | `Refund.activation_captures[0]` | `trigger = Flag(ready false)` | false |
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
| 21 | `Committed.constraint` | `committed = Flag(ready true)` | true |
| 22 | `Closed.constraint` | none | true |
| 23 | the registration capture of row 3 again with `forward = Receipt(total 9)`, then the recovery predicate of row 12 with `recovered = Receipt(total 9)` and `target = 5` | the capture slot gives 9; the recovery predicate gives false: the total registered earlier (5) is the one it reads, whatever the record now holds |
| 24 | `Refund.recovery.predicate` | `recovered = Receipt(total 9)`, `target = 9`, `activated = false` | true: the evaluator reads the captured value it is given and recomputes nothing |
| 25 | `Refund.retry.relation` | none | refuses `invalid_runtime_input`/`missing-member` naming `target`, the lowest source |
| 26 | each of `Refund.trigger.guard`, `Refund.recovery.predicate`, `Partial.retry.relation`, `Partial.recovery.predicate` | none | refuses `missing-member` naming `target` (`partialTarget`) |
| 27 | `Refund.registration_captures[0]` and `Refund.activation_captures[0]` | none | refuses `missing-member` naming `forward` and `trigger` |
| 28 | `Refund.retry.relation` | row 8's supply and `forward = Receipt(total 5)` | refuses `unknown-member` naming `forward`: the retry relation does not read it |
| 29 | `Got.related.from[0]` (L) | `view = n5` | the reference `n5` |
| 30 | `Got.related.to[0]` (L) | `view = n5` | the reference `n5` |
| 31 | `Decide.visible[0]` (L) | `got = Flag(ready true)` | true (`got.ready`) |
| 32 | `Decide.cases.guard[0]` and `[1]` (L) | `got = Flag(ready true)` | true (`yes`), false (`no`) |
| 33 | `Decide.cases.guard[1]` (L) | `got = Flag(ready false)` | true (`no`) |

Row 26's four refusals name `target`, the lowest source of each slot (its level
is below the trigger's, the activation capture's and the recovery binder's).

## Expected Results

Every slot returns the value its row gives from the supplied values alone. A
capture slot returns its initializer's value and nothing keeps it: rows 23 and
24 show that a later slot reads the captured value the caller supplies, 5 or 9,
and never the record the capture once read. A supply holds exactly the
binders and captured values the expression reads, so rows 25 to 28 name a
source or refuse a supplied value the expression does not read. The two
templates read their own binders only.
