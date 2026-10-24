---
id: FR-384
title: "Derive the protocol occurrence schema and runtime binding requirements from the protocol nodes"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-050
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-051
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-210
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-465
    type: depends_on
---
# FR-384: Derive the protocol occurrence schema and runtime binding requirements from the protocol nodes

## Description

QSL SHALL derive, from the protocol nodes of a checked package (FR-383), a
static occurrence schema for each protocol and the runtime binding
requirements a later runtime satisfies, and SHALL carry neither on any
node. The derivation reads the nodes only, so the schema and the requirements
derived from the in-process checked package equal those derived from the
package read back through QSL's I2 reader. A package fabricates no workflow
instance, role instance, repeat ordinal, delivery or clock observation, and
compiling needs none of them (QSpec FR-465 "Runtime subjects").

## Use case

A protocol assessor receives a package for a reusable payment protocol. It
needs to know, before any workflow runs, which role slots, which authored
control occurrences and which enclosing repeat ordinal domains a runtime key
has, and which bindings (a workflow instance, a role instance per role, a
message, a delivery, an operation attempt) a runtime must supply. Two
protocols that share a provider, a model or a local name must give distinct
schema identities. The assessor reads all of it from the package and supplies
the concrete instances itself.

## Semantic authority and boundary

QSpec FR-465 owns the statement that runtime subjects are a function of the
operators and carried by no node, and QSpec FR-210 the subject vocabulary.
QSpec FR-050 and FR-051 own role instances and relationship binding. This
requirement specifies what QSL derives and returns. It binds nothing: the
concrete instance, ordinal, delivery and observation values are the
consumer's.

## Inputs

- The protocol nodes of one `protocol_clause` (FR-383) and the nodes they
  reach: from the in-process `CheckedPackage` or from QSL's I2 read.

## Outputs

`ProtocolOccurrenceSchema` for each `protocol_clause` node:

- `workflow`: one slot, the `workflow-instance` requirement of the clause.
- `roles`: one slot for each `role` node, ordered by the segments of the role's
  declaration path, each as UTF-8 bytes, each holding the role node's
  key, its declaration region and its replication (static, or replicated
  with its population, `max` and lifetime).
- `controls`: one entry for each `control` node, ordered by
  the segments of the declaration path, each as UTF-8 bytes, each holding the control's
  key, its operator, its declaration region, its owner and its repeat
  dimensions.
- `compensations`: one entry for each `compensation` node, ordered the same
  way, each holding the template's key, its role, its forward `effect`
  control and its maximum attempts.

`RuntimeBindingRequirement` records for the protocol: one record for each
(subject kind, subject node) the operators of the protocol's nodes imply,
each holding its subject kind, the subject node's key, its scope (the key of
the `protocol_clause` node) and its authority and contract, both absent.

## Behavior

### Occurrence schema

- A control's **owner** SHALL be a role node, from the nodes alone: the owner
  of a `choice` or `repeat`, the role of an `attempt`, an `event` or a
  `commit`, the `from` role of a `send`'s channel, the `to` role of a
  `receive`'s channel, and the role of the `attempt` an `effect` is of. A
  `sequence`, `parallel`, `await`, `check` or `fence` is **structural**: S4
  derives no participant owner for it.
- A control's **repeat dimensions** SHALL list, from the outermost to the
  innermost, one dimension for each `repeat` that holds the control in its
  `body` or its `exhausted` control. A control held in a `repeat`'s `body`
  has an `Iteration` dimension whose domain is `[0, maximum)` and that is
  unbounded when the `repeat` writes no maximum; a maximum of 0 is an empty
  domain. A control held in the `exhausted` control has an `Exhaustion`
  dimension whose single value is the maximum. A control held by no `repeat`
  has no dimension. A `parallel` branch, a `choice` case and the `then` and
  `timeout` positions of an `await` add no dimension.
- The schema SHALL hold no concrete workflow identity, role instance and
  ordinal. Each is a runtime input.
- The schema is a function of the nodes and not of the source: declaring the
  roles, channels or compensations in another order, or reformatting the
  source, gives the same schema. Two protocols that name a role, a control or
  a model alike give distinct node keys, so their schema entries are distinct.

### Runtime binding requirements

- The subject kinds a node's operator implies SHALL be:

  | Operator | Subject kinds bound at runtime |
  | --- | --- |
  | `protocol` | `workflow-instance` |
  | `role` | `role-instance` (one for each assessed workflow; one for each spawned object for a replicated role) |
  | `send` | `message`, `send` |
  | `receive` | `receive`, `delivery` |
  | `attempt` | `operation-attempt` |
  | `effect` | `business-effect` |
  | `commit` | `commit` |
  | `compensate` | `compensation`, `compensation-attempt`, `compensation-effect` |
  | `finish` | the workflow-closure observation |

  Every other operator implies none.
- A record's scope SHALL be the key of the `protocol_clause` node, and its
  authority and contract SHALL be absent: the consumer's binding supplies
  them (QSpec FR-465), and the derivation never guesses an authority or a
  contract. A record names no concrete value.
- A requirement is derived for a node the package holds. A construct the
  package does not hold has no requirement, and the derivation reads no
  source text, display string or name for a node's key.
- Compiling a protocol needs no workflow instance, no delivery, no clock
  observation and no concrete ordinal, and the package holds none.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-384-AC-1 | Over fixture **W** (TC-1061), the schema holds one `workflow` slot, three role slots ordered by declaration path, the 15 control entries and the one compensation entry; the owners are `Orders` for `Requested`, `Payments` for `Received`, `Charged` and `Applied`, `Fulfillment` for `East`, `West`, `Outcome`, `Poll` and `Polled`, `Orders` for `Finalized`, and structural for `Main`, `Shipments`, `Done`, `Failed` and `Limit`; a `fence` and an `await` in variants are structural. | Test (TC-1064) |
| FR-384-AC-2 | In **W**, `Polled` has one `Iteration` dimension with domain `[0, 3)` and `Limit` one `Exhaustion` dimension with value 3; a control inside a `repeat` inside a `repeat` has two dimensions, outer first; a `repeat` with `maximum` 0 gives an empty `Iteration` domain; a `repeat` with no maximum gives an unbounded `Iteration` dimension and no `Exhaustion` entry; a control in a `parallel` branch, a `choice` case or an `await`'s `then` outside any `repeat` has none. | Test (TC-1064) |
| FR-384-AC-3 | The schema and requirements of **W** hold no workflow identity, role instance, ordinal, delivery or clock observation; compiling **W** with no runtime input succeeds; the schema and requirements derived from the in-process package equal those derived from the package read back through the I2 reader. | Test (TC-1064) |
| FR-384-AC-4 | Two protocols in one package that each declare a role `Payer` on the same model and a control `Charge` give distinct role keys and distinct control keys, so four distinct schema entries; **W** with its roles, channels and compensations declared in another order gives the same schema. | Test (TC-1064) |
| FR-384-AC-5 | Over a variant of **W** with a `commit`, an `effect`, a `send`, a `receive`, an `attempt`, a replicated role and a template, the requirements hold exactly the subject kinds of the table for each node: one `workflow-instance` for the clause; `role-instance` for each role, with the replicated role's marked as one for each spawned object; `message` and `send` for the `send`; `receive` and `delivery` for the `receive`; `operation-attempt`; `business-effect`; `commit`; `compensation`, `compensation-attempt` and `compensation-effect` for the template; the workflow-closure observation for `finish`; and none for a `sequence`, `choice`, `parallel`, `repeat`, `await`, `check`, `fence`, `event`, channel, queue or `terminal`. | Test (TC-1064) |
| FR-384-AC-6 | Each requirement names its subject node's key, the `protocol_clause` key as its scope and an absent authority and contract. | Test (TC-1064) |
| FR-384-AC-7 | No protocol node holds a requirement, a schema entry, an instance or an ordinal: the nodes of **W** contain none, and an `internal` or `external` member added to a role node is refused by QSL's I2 reader with `malformed_wire`. | Test (TC-1064) |
| FR-384-AC-8 | A `Full` and a `Partial` template for one effect give two compensation entries and two sets of `compensation`, `compensation-attempt` and `compensation-effect` requirements, each naming its own template's key. | Test (TC-1064) |

## Dependencies

- [FR-383](FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md) (the nodes
  and their keys, ordering and declarations).
- [FR-218](FR-218-check-every-protocol-control-construct-at-s3.md) (the
  checked clause the nodes are written from).
- QSpec FR-465 "Runtime subjects", FR-210, FR-050 and FR-051.
