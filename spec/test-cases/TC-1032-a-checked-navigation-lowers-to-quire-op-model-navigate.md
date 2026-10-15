---
id: TC-1032
title: "A checked navigation lowers to one quire.op.model.navigate application"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-363
    type: verifies
---
# TC-1032: A checked navigation lowers to one quire.op.model.navigate application

## Description

Verify that a checked `Navigate` node lowers to the `quire.op.model.navigate`
application FR-093 gives it. Scope: FR-363-AC-6.

Catches a lowering that routes `r.name` over an end through `deref` and
`record.project`, which names no field and is refused by the reader.

## Test Procedure

Over TC-1031's fixture **N**, check and lower the functions
`f1(r: Reference<M::Fulfilment>): Option<Reference<M::Order>> { r.order }`,
`f2(o: Reference<M::Order>): Set<Reference<M::Line>>[0,3] { o.lines }` and
`f3(l: Reference<M::Line>): Reference<M::Order> { l.owner }`. Read the emitted
package with IR's v2 reader.

## Expected Results

Each body is one `expression` node with `operator` `query`, `operation.identity`
`quire.op.model.navigate`, no laws, mode or leaves, `member` `relationship_end`
and arguments `[ref(parameter)]`. The `member` of `f1` names the model node of
`Fulfilment/order` and `order`; the `member` of `f2` names `Order/lines` and
`lines`; the `member` of `f3` names `Order/lines` and `owner`. The `result_type`
of each is the type node of its function's declared result type. No
`quire.op.model.deref` and no `quire.op.record.project` node is emitted, and
the reader admits each package.
