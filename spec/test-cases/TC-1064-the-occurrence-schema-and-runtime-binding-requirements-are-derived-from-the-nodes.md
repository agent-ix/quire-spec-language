---
id: TC-1064
title: "The occurrence schema and runtime binding requirements are derived from the protocol nodes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-384
    type: verifies
---
# TC-1064: The occurrence schema and runtime binding requirements are derived from the protocol nodes

## Description

Verify that the occurrence schema holds the workflow slot, the role slots, the
control entries with their owners and repeat dimensions and the compensation
entries, that the binding requirements are a function of the operators, that
neither is carried by any node or fabricates an instance, and that the
derivation is the same from the in-process package and from the read-back
package. Scope: FR-384-AC-1 to FR-384-AC-8.

Catches an owner invented for a `sequence`, an `Exhaustion` dimension missing
for the exhausted arm, a `repeat` with no maximum given a fabricated bound,
two same-named roles of different protocols sharing a schema entry, a binding
requirement written into a node, an authority guessed when the lock selects
none, and a schema that depends on the source order.

Fixture **W** is TC-1061's.

## Test Procedure

| # | Unit | Expected |
| --- | --- | --- |
| 1 | **W** | one `workflow` slot; three role slots ordered by declaration path; 15 control entries; one compensation entry. Owners: `Orders` for `Requested` and `Finalized`; `Payments` for `Received`, `Charged` and `Applied`; `Fulfillment` for `East`, `West`, `Outcome`, `Poll` and `Polled`; structural for `Main`, `Shipments`, `Done`, `Failed` and `Limit` |
| 2 | **W** with an `await` and a `fence` | each is structural |
| 3 | **W** | `Polled` has one `Iteration` dimension `[0, 3)`; `Limit` has one `Exhaustion` dimension with value 3; no other control has a dimension |
| 4 | **W** with a `repeat` inside a `repeat`; with a `repeat` of `max 0`; with a `repeat` that writes no maximum | two dimensions, outer first; an empty `Iteration` domain; an unbounded `Iteration` dimension and no `Exhaustion` entry |
| 5 | **W** | the schema and requirements hold no workflow identity, role instance, ordinal, delivery or clock observation; compiling needs none; the schema and requirements from the in-process package equal those from the package read back through the I2 reader |
| 6 | two protocols in one package, each with a role `Payer` on one model and a control `Charge`; **W** with its roles, channels and compensations declared in another order | four distinct schema entries for the first; the same schema for the second |
| 7 | a variant of **W** with a `commit`, `effect`, `send`, `receive`, `attempt`, a replicated role and a template | one `workflow-instance` for the clause; `role-instance` for each role, the replicated role's marked as one for each spawned object; `message` and `send`; `receive` and `delivery`; `operation-attempt`; `business-effect`; `commit`; `compensation`, `compensation-attempt` and `compensation-effect`; the workflow-closure observation for `finish`; none for a `sequence`, `choice`, `parallel`, `repeat`, `await`, `check`, `fence`, `event`, channel, queue or `terminal` |
| 8 | **W** | each record names its subject node key and the clause key as scope, with authority and contract absent |
| 9 | **W**; **W** with an `internal` member added to a role node | no node holds a requirement, schema entry, instance or ordinal; the I2 reader refuses the added member with `malformed_wire` |
| 10 | **W** with a second template `Partial` for `Applied` | two compensation entries, and two sets of `compensation`, `compensation-attempt` and `compensation-effect` requirements, each naming its own template key |

## Expected Results

- Each row asserts typed fields; an ordering assertion compares the declared
  path segments as UTF-8 bytes.
