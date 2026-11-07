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
compiling needs none of them (QSpec FR-465 "Runtime subjects"). A runtime
supplies its concrete values to the schema's key binder, which validates them
against the schema and retains them as one occurrence key without storing any
of them in the schema or the package.

## Use case

A protocol assessor receives a package for a reusable payment protocol. It
needs to know, before any workflow runs, which role slots, which authored
control occurrences and which enclosing repeat ordinal domains a runtime key
has, and which bindings (a workflow instance, a role instance per role, a
message, a delivery, an operation attempt) a runtime must supply. Two
protocols that share a provider, a model or a local name must give distinct
schema identities. The assessor reads all of it from the package and supplies
the concrete instances itself. Two later workflows, O1 and O2, share one
external payment provider and run the same protocol: each builds its own
occurrence keys from its workflow identity and one ordinal for each repeat
domain, and the two sets of keys never collapse.

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
  key, its declaration path, the selected model type node the role is `on`,
  for a role on a Port the selected Port's `model_member` identity (its
  `package` and `node`, FR-383) as well, and its
  replication (static, or replicated with its population, `max` and a
  lifetime of `workflow`, `scope` or `until`).
- `controls`: one entry for each `control` node, ordered by
  the segments of the declaration path, each as UTF-8 bytes, each holding the control's
  key, its declaration path, its operator (one of the thirteen control forms),
  its owner and its repeat dimensions.
- `compensations`: one entry for each `compensation` node, ordered the same
  way, each holding the template's key, its role, its forward `effect`
  control and its maximum attempts.

No entry holds a source position, source text or display string: the
declaration path and the key locate each entry, and the schema is a function
of the nodes alone.

`RuntimeBindingRequirement` records for the protocol: one record for each
(subject kind, subject node) the operators of the protocol's nodes imply,
each holding its subject kind, the subject node's key and its scope (the key of
the `protocol_clause` node). A record holds no authority and no contract: the
package lock's `binding_contract` selection supplies both (QSpec FR-465
"Runtime subjects"), so the record has no member for either.

`WorkflowInstanceIdentity`: a caller-owned UTF-8 text of 1 to 4096 bytes,
retained as given and never interpreted.

`OccurrenceKey`: the schema's protocol key, the caller's workflow identity, a
control entry's key and owner, and the caller's ordinals, one for each repeat
dimension of that entry, outermost first. The key borrows the caller's
identity and ordinals.

`OccurrenceKeyError`, the typed refusal of the key binder:
`EmptyWorkflowIdentity`; `WorkflowIdentityTooLong` with the offered byte count;
`UnknownNode` with the offered control key; `WrongRole` with the entry's owner
and the offered owner; `RepeatCount` with the expected and the offered ordinal
count; `RepeatOrdinal` with the outer-to-inner dimension index and the refused
ordinal.

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
- The `event` control an `await` awaits is a control entry like any other: it
  is held by the `await`, so it takes the dimensions that enclose the `await`
  and adds none, and its owner is the role it is `by`. The `await` itself is
  structural.
- The schema SHALL hold no concrete workflow identity, role instance and
  ordinal. Each is a runtime input.
- The schema is a function of the nodes and not of the source: declaring the
  roles, channels or compensations in another order, or reformatting the
  source, gives the same schema. Two protocols that name a role, a control or
  a model alike give distinct node keys, so their schema entries are distinct.

### Runtime key binding

- `ProtocolOccurrenceSchema::bind` SHALL take a `WorkflowInstanceIdentity`, a
  control key, a `ControlOwner` and a slice of ordinals, and return one
  `OccurrenceKey` or the first refusal in this order: the control key is no
  entry of this schema (`UnknownNode`: this covers an absent key and the key of
  a control of another protocol, whose node key is distinct even when the
  control's name is the same); the owner differs from the entry's owner
  (`WrongRole`, a structural entry and a role-owned entry included); the
  ordinal count differs from the entry's dimension count (`RepeatCount`,
  missing and surplus alike); an ordinal lies outside its dimension
  (`RepeatOrdinal`). An `Iteration` dimension admits the integers from 0 up to
  but excluding its maximum, so a maximum of 0 admits none and a missing
  maximum admits every non-negative integer; an `Exhaustion` dimension admits
  exactly its value.
- `WorkflowInstanceIdentity::new` SHALL refuse the empty text and a text over
  4096 bytes, and accept every length between.
- `bind` SHALL NOT default, generate, order or read any identity or ordinal from
  the schema or the package: a key holds exactly what the caller offered. Two
  keys that differ only in workflow identity, or only in one ordinal, are
  unequal.
- A schema exists only for a `protocol_clause` node, so no node of another
  family can be bound against: the derivation returns one schema for each
  protocol clause and none for any other node.
- The derivation is bounded by the package it reads. A package is read within
  the I2 ceilings (`i2.nodes`, `i2.occurrences`, `i2.edges`; FR-513-AC-10), a
  schema holds one entry for each role, control and compensation node of its
  protocol, and the control tree is walked iteratively, visiting each control
  once. A control tree of any depth or with a reference cycle therefore ends
  with one entry for each control, and a reference to a node the package does
  not hold is refused as `UnknownNode` of the derivation (`OccurrenceRefusal`).

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
- A record's scope SHALL be the key of the `protocol_clause` node, and it
  SHALL hold no authority and no contract: the package lock's
  `binding_contract` selection supplies them (QSpec FR-465), and the
  derivation never guesses an authority or a contract. A record names no
  concrete value.
- A requirement is derived for a node the package holds. A construct the
  package does not hold has no requirement, and the derivation reads no
  source text, display string or name for a node's key.
- Compiling a protocol needs no workflow instance, no delivery, no clock
  observation and no concrete ordinal, and the package holds none.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-384-AC-1 | Over fixture **W** (TC-1061), the schema holds one `workflow` slot, three role slots ordered by declaration path, the 15 control entries and the one compensation entry; the owners are `Orders` for `Requested`, `Payments` for `Received`, `Charged` and `Applied`, `Fulfillment` for `East`, `West`, `Outcome`, `Poll` and `Polled`, `Orders` for `Finalized`, and structural for `Main`, `Shipments`, `Done`, `Failed` and `Limit`; a `fence` and an `await` in variants are structural, and an `await`'s awaited `event` is an entry owned by its role. | Test (TC-1064) |
| FR-384-AC-2 | In **W**, `Polled` has one `Iteration` dimension with domain `[0, 3)` and `Limit` one `Exhaustion` dimension with value 3; a control inside a `repeat` inside a `repeat` has two dimensions, outer first; a `repeat` with `maximum` 0 gives an empty `Iteration` domain; a `repeat` with no maximum gives an unbounded `Iteration` dimension and no `Exhaustion` entry; a control in a `parallel` branch, a `choice` case or an `await`'s `then` outside any `repeat` has none. | Test (TC-1064) |
| FR-384-AC-3 | The schema and requirements of **W** hold no workflow identity, role instance, ordinal, delivery or clock observation; compiling **W** with no runtime input succeeds; the schema and requirements derived from the in-process package equal those derived from the package read back through the I2 reader. | Test (TC-1064) |
| FR-384-AC-4 | Two protocols in one package that each declare a role `Payer` on the same model and a control `Charge` give distinct role keys and distinct control keys, so four distinct schema entries; **W** with its roles, its channel or its compensations declared in another order, or reformatted, gives the same schema in every entry, so no entry depends on a source position. | Test (TC-1064) |
| FR-384-AC-5 | Over a variant of **W** with a `commit`, an `effect`, a `send`, a `receive`, an `attempt`, a replicated role and a template, the requirements hold exactly the subject kinds of the table for each node: one `workflow-instance` for the clause; `role-instance` for each role, with the replicated role's marked as one for each spawned object; `message` and `send` for the `send`; `receive` and `delivery` for the `receive`; `operation-attempt`; `business-effect`; `commit`; `compensation`, `compensation-attempt` and `compensation-effect` for the template; the workflow-closure observation for `finish`; and none for a `sequence`, `choice`, `parallel`, `repeat`, `await`, `check`, `fence`, `event`, channel, queue or `terminal`. | Test (TC-1064) |
| FR-384-AC-6 | Each requirement names its subject node's key and the `protocol_clause` key as its scope, and holds no authority and no contract member. | Test (TC-1064) |
| FR-384-AC-7 | No protocol node holds a requirement, a schema entry, an instance or an ordinal: the nodes of **W** contain none, and an `internal` or `external` member bound into the body arguments of a role node is refused by QSL's I2 reader with `malformed_wire`, and the same member added as a member of the node itself, outside its body, is refused with `unknown_member`, because the node shape is closed. | Test (TC-1064) |
| FR-384-AC-8 | A `Full` and a `Partial` template for one effect give two compensation entries and two sets of `compensation`, `compensation-attempt` and `compensation-effect` requirements, each naming its own template's key. | Test (TC-1064) |
| FR-384-AC-9 | In **W**, each role slot holds the selected model type node it is `on`, equally in process and after the I2 read: `Fulfillment` holds `Shipment`, `Orders` holds `OrderView` and `Payments` holds `Payment`, and each is an emitted `object_type` node of the selected package, and none holds a `model_member`. | Test (TC-1064) |
| FR-384-AC-10 | Over FR-383-AC-62's unit, the role slot of `Outlet` holds the node of `Sys` and the `model_member` `{package, node}` of `pump_out`, that of `Inlet` the node of `Sys` and the `model_member` of `tank_in`, and those of `Operator` and the whole-model role hold none; the slots from the in-process package equal those from the package read back through the I2 reader. | Test (TC-1064) |
| FR-384-AC-11 | In **W**, the keys of two workflows `O1` and `O2` for `Polled` with the same ordinal `[0]` are unequal and each returns its own workflow identity, the control, the owner and the ordinals it was given; the same workflow with ordinal `[2]` is a third key. In **W** with a `repeat` inside a `repeat`, the ordinals `[2, 1]` bind and `[3, 1]` and `[2, 2]` are refused at dimensions 0 and 1; a `repeat` of `max 0` refuses ordinal 0 and a `repeat` with no maximum admits 0 and an integer beyond 64 bits and refuses -1; `Limit` admits exactly its exhaustion value 3 and refuses 2 and 4. | Test (TC-1064) |
| FR-384-AC-12 | A workflow identity of 0 bytes is refused as `EmptyWorkflowIdentity`; 1 byte and 4096 bytes are accepted and returned unchanged; 4097 bytes is refused as `WorkflowIdentityTooLong` naming 4097. | Test (TC-1064) |
| FR-384-AC-13 | Binding refuses, each with its typed cause: a control key absent from the schema and the `Charge` control key of the other protocol (`UnknownNode`, naming the offered key); a swapped role, a structural owner for a role-owned entry and a role owner for a structural entry (`WrongRole`, naming both owners); no ordinal and two ordinals for an entry with one dimension, and one ordinal for an entry with none (`RepeatCount`, naming both counts); an out-of-range ordinal (`RepeatOrdinal`, naming its dimension and value). The refused call yields no key. | Test (TC-1064) |
| FR-384-AC-14 | **W**, which also holds model type nodes, yields exactly one schema, keyed by its `protocol_clause` node, and no node of any other family yields one; each schema holds one entry for each role, control and compensation node of the read package; a control chain of 10 000 nested sequences yields 10 000 entries and a control reference cycle yields each control once; a role that names a node the package does not hold is refused as `UnknownNode` naming the protocol. The I2 node ceiling that bounds the package is exercised at zero, exact and one-short by FR-521's row 11 tests. | Test (TC-1064) |

## Dependencies

- Contract IR IR-1310: the reader sees the declaration kinds of the selected model package, which a Port role's `model_member` check needs (QSpec FR-465).
- [FR-383](FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md) (the nodes
  and their keys, ordering and declarations).
- [FR-218](FR-218-check-every-protocol-control-construct-at-s3.md) (the
  checked clause the nodes are written from).
- QSpec FR-465 "Runtime subjects", FR-210, FR-050 and FR-051.
