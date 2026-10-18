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
package over the sources the caller supplies, computes every capture itself from
its initializer, reads a capture only when the expression reaches it, and keeps
the two templates of one effect apart. Scope: FR-390-AC-1 to FR-390-AC-3.

Catches a caller that must supply a captured total, a capture evaluated though a
short-circuit skipped it, a capture evaluated twice in one call, a retry
relation read through the wrong template's binders, and a slot that returns the
value of its neighbour.

Fixture **E** is TC-1057's fixture **R** with two additions. The domain package
`M` declares the populations `nodes` (of `Node`), `plains` (of `Plain`) and
`ticks` (of `Tick`). The protocol `Refunds` of TC-1057 gains a second template
after `Refund`:

```
compensate Partial for Main::Applied as (partialForward: M::Node)
    by Service on M::Node::step using T clock "partial-clock" {
  capture partialTarget: M::Total = partialForward.total;
  activate first (partialTrigger: M::Plain)
      when { not partialTrigger.ready and partialTarget >= 0 } {
    capture partialActivated: Boolean = partialTrigger.ready;
  }
  within [0,30]; attempts 3 of M::Node;
  retry (partialEarlier: M::Node, partialLater: M::Node) {
    partialEarlier.tally < 3 and partialLater.tally = partialEarlier.tally + 1
    and partialTarget >= 0 and not partialActivated
  };
  commit never;
  recover (partialRecovered: M::Node) {
    partialRecovered.n = 1 and partialTarget >= 0 and not partialActivated
  };
}
```

The package is the checked package QSL compiles from the unit (the S4 protocol
nodes). The observation is one current snapshot with every population complete:

| Population | Key | Fields |
| --- | --- | --- |
| `nodes` | `a` | `total` 5, `tally` 0, `n` 1 |
| `nodes` | `b` | `total` 5, `tally` 1, `n` 0 |
| `nodes` | `c` | `total` 5, `tally` 3, `n` 0 |
| `plains` | `off` | `ready` false |
| `plains` | `on` | `ready` true |
| `ticks` | `go` | `ready` true, `count` 2 |

A slot is written `node.member[index]`, the node named by its source name
(`Refund` is the node of `compensate Refund`, `Applied` the `effect`,
`Committed` the `commit`, `Closed` the `finish`, `Refunds` the clause). A supply is the list
of `binder = value` pairs, every value a reference into the observation. The
meter is the default meter.

## Test Procedure

Evaluate one slot per row with the supply the row gives and assert the
`ProtocolSlotEvaluation`: its value and its `captures()`.

| # | Slot | Supply | Expected value | `captures()` |
| --- | --- | --- | --- | --- |
| 1 | `Refunds.activation.guard` | `opened = go` | true | none |
| 2 | `Refunds.captures[0]` | `view = a` | 5, type `M::Total` | none |
| 3 | `Refund.registration_captures[0]` | `forward = a` | 5 | none |
| 4 | `Refund.trigger.guard` | `forward = a`, `trigger = off` | true | `target` |
| 5 | `Refund.trigger.guard` | `forward = a`, `trigger = on` | false | none |
| 6 | `Refund.activation_captures[0]` | `trigger = on` | true | none |
| 7 | `Refund.activation_captures[0]` | `trigger = off` | false | none |
| 8 | `Refund.retry.relation` | `earlier = a`, `later = b`, `forward = a`, `trigger = off` | true | `target`, `activated` |
| 9 | `Refund.retry.relation` | as row 8 with `trigger = on` | false | `target`, `activated` |
| 10 | `Refund.retry.relation` | as row 8 with `earlier = c` | false | none |
| 11 | `Refund.retry.relation` | as row 8 with `later = a` | false | none |
| 12 | `Refund.recovery.predicate` | `recovered = a`, `forward = a`, `trigger = off` | true | `target`, `activated` |
| 13 | `Refund.recovery.predicate` | as row 12 with `recovered = b` | false | none |
| 14 | `Refund.recovery.predicate` | as row 12 with `trigger = on` | false | `target`, `activated` |
| 15 | `Partial.retry.relation` | `partialEarlier = a`, `partialLater = b`, `partialForward = a`, `partialTrigger = off` | true | `partialTarget`, `partialActivated` |
| 16 | `Partial.recovery.predicate` | `partialRecovered = a`, `partialForward = a`, `partialTrigger = off` | true | `partialTarget`, `partialActivated` |
| 17 | `Partial.recovery.predicate` | as row 16 with `partialRecovered = b` | false | none |
| 18 | `Applied.constraint` | `view = a` | true (`order >= 0`) | `order` |
| 19 | `Committed.constraint` | `committed = on` | true | none |
| 20 | `Closed.constraint` | none | true (`true`) | none |
| 21 | `Refund.retry.relation` | none | refuses `invalid_runtime_input`/`missing-member` naming `forward`, never `target` or `activated` | |
| 22 | each of `Refund.trigger.guard`, `Refund.recovery.predicate`, `Partial.retry.relation`, `Partial.recovery.predicate` | none | refuses `missing-member` naming `forward` (`partialForward`), never a capture | |
| 23 | `Refund.activation_captures[0]` | none | refuses `missing-member` naming `trigger` | |
| 24 | `Refund.retry.relation` | row 8's supply and `target = a` | refuses `invalid_runtime_input`/`unknown-member` naming `target` | |
| 25 | `Refund.retry.relation` | row 8's supply and `activated = on` | refuses `unknown-member` naming `activated` | |

Row 22's four refusals name the template's own forward binder because the
sources are checked in ascending level and the forward binder is the lowest.

## Expected Results

Every slot returns the Boolean or the capture value its row gives, from
sources alone. A capture is computed from its initializer and is never
supplied: rows 21 to 25 ask for a source, never a capture, and refuse a supplied
capture. `captures()` lists a capture only when the expression reaches it: rows
10, 11 and 13 decide on the first operand and read none, and rows 8, 9 and 12
read `target` before `activated`, once each. The two templates read their own
binders only.
