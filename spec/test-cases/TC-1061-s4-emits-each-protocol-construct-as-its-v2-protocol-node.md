---
id: TC-1061
title: "S4 emits each protocol construct as its v2 protocol node"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: verifies
---
# TC-1061: S4 emits each protocol construct as its v2 protocol node

## Description

Verify that S4 emits a checked protocol as the clause, role, channel, queue,
obligation, control and compensation nodes of QSpec FR-464, FR-465 and
FR-466, with each construct's arguments written from the checked fact, the
absent optionals absent and every node admitted by the IR v2 reader. Scope:
FR-383-AC-1 to FR-383-AC-8.

Catches a protocol that still emits only its clause stub, a construct with no
node, an argument written from the wrong checked fact, a default written for
an optional the source omits, an `each` activation whose guard is invented, a
relationship alias written into the clause, a queue missing for a channel or
written twice, a loss selector written for another overflow, a `commit never`
written as a commit, and a `Full` and a `Partial` template merged.

Fixture **W**: QSpec TC-946's base protocol `OrderFlow`, authored as one QSL
unit over a domain package `M` with the order, charge, shipment and refund
types that protocol names, model alias `M`, protocol profile alias `P`,
temporal profile alias `T` and the unit's temporal clause
`OrderClosureBounded`. It has activation `on each (opened: M::OrderStarted)`,
the capture `order`, the static roles `Orders`, `Payments` and `Fulfillment`,
the relationship `ShipmentOwner`, the FIFO channel `Charges` from `Orders` to
`Payments` (delivery `at-least-once`, capacity `literal 4`, overflow
`reject`), `requires temporal OrderClosureBounded`, `terminal any`,
`scheduling adversarial`, the `run` tree `Main` (`Requested`, `Received`,
`Charged`, `Applied`, `Shipments` with branches `east` and `west` joined
`all`, `Outcome` with cases `shipped` and `failed`, `Poll` with `max 3` and an
`exhausted` check `Limit`, `Finalized`: 15 controls), `finish Closed` and the
template `Refund` for `Applied` with `commit Finalized`. Variants of **W**
change only what each row states. Every row compiles the unit, reads the
package with the IR v2 reader and asserts typed fields, never message text.

## Test Procedure

| # | Unit | Expected |
| --- | --- | --- |
| 1 | **W** | the package holds one `protocol_clause`; three `role`, one `channel`, one `queue`, one `finish` and one `terminal` node; 15 `control` nodes, one `compensation` node; every body is a `quire.op.protocol.control` application with one `protocol_profile` law equal to the lock's row, a `profile_operator` member naming the node's operator, `null` mode, empty leaves and the Boolean `semantic_type`; the reader admits it |
| 2 | **W**; **W** with `on origin`; **W** with `on each` and no guard; **W** with `on each ... when ({true})`; **W** with no `terminal`, no `scheduling`, no relationship, no `requires temporal` and no template | the clause has thirteen arguments in order; `origin` carries no trigger and no guard; `on each` carries the trigger binder node and the guard only when written, and the last two units have different clause keys; `relationships` holds the model relationship node (no alias), `requirements` the temporal clause node; the stripped unit has empty sets and absent optionals and is admitted |
| 3 | **W**; **W** with a replicated role of each lifetime (`workflow`, `scope`, `until`) | the static role has an empty `replication` Group; each replicated role carries its population text, `max`, binder node and lifetime, and `until` only for `until`; the four roles have four different keys |
| 4 | **W** with `Charges` as `unordered`, then `fifo`; with each delivery (`unknown`, `at-most-once`, `at-least-once`, `exactly-once-premise`); with capacity `literal 4` and `symbolic Bound`; with overflow `reject`, `block`, `loss head`, `loss tail`, `loss new` | the channel carries `from`, `to`, `carries`, the ordering Group (a `fifo` has the message binder and the key, an `unordered` neither) and the delivery text; each channel has one queue with the capacity Group, the overflow text and a fourth argument only for `loss`; the three selectors give three queue keys; the queue has no `declaration` and one `generated` occurrence at the channel's region |
| 5 | **W**; **W** with `terminal when {...}`; **W** with no `terminal` and no `scheduling` | `finish` carries the closing binder and the constraint; `terminal when` carries mode `when` and the condition, `terminal any` mode `any` and none; the clause carries `terminal` and `scheduling` `adversarial` only when written |
| 6 | **W**; **W** with an empty `sequence`; **W** with each join policy (`quorum(2)`, `any`, `predicate`), a `parallel` memory model and `outstanding`; **W** with a `repeat` that has a maximum, a loop proof, both, neither and `max 0` | the `sequence` emits its children in order and an empty `sequence` an empty list; the `choice` its owner, `visible` set and two cases sorted by label; the `parallel` the join Group (with `count` for `quorum`, the function node for `predicate`), the memory model and `outstanding` exactly when written and the joined labels ascending; the `repeat` an `exhausted` control exactly with a maximum, the proof Group empty or with `invariant` and `variant`, and `maximum` 0 as 0 |
| 7 | **W** with an `await` after an event node and one after a compensation, an `event` with and without a compensation, a `fence` of each ordering, an `attempt` of each memory ordering and none, `related by` clauses on a `send`, `receive`, `attempt`, `effect` and `event` | each control emits its layout: the `await` anchor Group, clock text, closed interval, awaited event, `then` and `timeout`; the `send` and `receive` channel, payload binder and received `send`; the `attempt` role, operation anchor (the FR-105 node), ordering, `contracts` and binder; the `effect` its `attempt`; `related` records of relationship node and both end terms |
| 8 | **W**; **W** with `commit never`; **W** with a second template `Partial` for `Applied` with `commit never` | the template emits its thirteen arguments in order; `commit never` has no `commit` argument; `Refund` and `Partial` are two nodes with different keys, each naming its own forward effect, captures, maximum, commit and recovery |

## Expected Results

- Row 1 admits the package and the counts are exact: no node for a construct is
  missing and none is emitted twice.
- Rows 2 to 8 match the stated arguments by typed field; an optional the
  source omits is the absent optional in every row.
- No row emits a node for a relationship alias, and none emits a node of a
  tag other than `protocol` for a protocol construct, apart from the model,
  state, temporal and value nodes the protocol references.
