---
id: FR-383
title: "Emit a checked protocol as v2 protocol nodes"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-095
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-264
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-381
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-464
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-465
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-466
    type: depends_on
---
# FR-383: Emit a checked protocol as v2 protocol nodes

## Description

The S4 package emitter (ADR-011 layer 4 `package`, edge E4) SHALL emit each
checked protocol (FR-218, FR-381) as a family of
`quire.checked-semantic-graph/v2` nodes with `node_tag: "protocol"`: one
`protocol_clause` node, one node for each role, channel, queue, obligation,
control and compensation template, and the `value`/`parameter` and
`expression` nodes they reference. QSL's I2 reader SHALL read them back. This
is the compiler side of QSpec FR-464, FR-465 and FR-466: QSpec owns every
body, the operation identity, the operator catalog, the member kinds and the
reader's refusals; QSL emits them from its checked clause and reads them
through its I2 reader. The nodes are the only carrier of the protocol. S4
writes no separate protocol document.

## Use case

A verification operator compiles a unit with an order protocol: three roles,
a FIFO channel, a `parallel` joined on both branches, a `choice`, a bounded
`repeat`, a refund template and a `finish`. The package carries the protocol
as protocol nodes that any QSpec reader admits, so a backend, a model checker
or a conformance assessor reads the same roles, queue bounds, controls and
recovery rules from the package and from nothing else, and a reformatted
source or a reordered declaration list gives the same package.

## Semantic authority and boundary

QSpec FR-464 owns the node family, the key inputs, the clause body, the
binders and the reader order; FR-465 the role, channel, queue, `finish` and
`terminal` bodies; FR-466 the thirteen control bodies and the compensation
body (References). This requirement specifies which checked fact S4 writes
into which argument, what S4 sorts, collapses and refuses, and what QSL's
reader returns. FR-384 specifies the occurrence schema and the runtime binding
requirements that S4 derives from these nodes.

## Inputs

- A checked protocol: the `CheckedProtocolClause` of FR-218 and FR-381
  (control tree, channels with their premises, roles, relationships, required
  temporal clauses, captures, activation, compensation templates, `terminal`
  and `scheduling` members, binders with their scopes), each optional member
  present or absent as written.
- The package's lock evidence: the `protocol_profile` row of
  `profile_selections`, and its `temporal_profile` row when the protocol
  holds an `await` or a `compensate`.
- The nodes of the same graph the protocol names: its model and relationship
  nodes (FR-094), the operation anchor and state clause nodes of its attempts
  and templates (FR-105), the temporal clause nodes it requires (FR-337), its
  type nodes and its value and parameter nodes (FR-092, FR-093).

## Outputs

One node for each row below, for every checked protocol. Every node has
`node_tag` `protocol`, the Boolean scalar type node as `semantic_type`, and
`recursion` `null`. Every body is one `quire.op.protocol.control`
application whose `protocol_profile` law equals the lock's row, whose member
is `{kind: "profile_operator", operator}` and whose arguments are the
operator's bindings in the order QSpec FR-464, FR-465 or FR-466 fixes. `P` is
the protocol's qualified name.

| Operator | `semantic_form` | `declaration.qualified_name` | Arguments, in order, written from |
| --- | --- | --- | --- |
| `protocol` | `protocol_clause` | `P` | `over`, a reference to the `over` parameter node; `activation`, `origin`, or `each` with the trigger binder node and the guard when one is written; `captures`, the protocol captures in source order; `roles`, `relationships`, `channels`, `queues`, `requirements`, `compensations`, each a set; `terminal`, the `terminal` node when one is written; `scheduling`, the text `adversarial` when written; `run`, the root control; `finish`, the `finish` node |
| `role` | `role` | `P`, `role`, the role name | the node of the role's model; the replication Group, empty for a static role and, for a replicated role, its population text, `max`, binder node, lifetime (`workflow`, `scope` or `until`) and the `until` term for `until` |
| `channel` | `channel` | `P`, `channel`, the channel name | the `from` and `to` role nodes; the `carries` type node; the ordering Group (`unordered`, or `fifo` with the message binder node and the key term); the delivery text |
| `queue` | `queue` | none | the channel node; the capacity Group (`literal` with its value, or `symbolic` with its unresolved name); the overflow text; the loss selector when the overflow is `loss` |
| `finish` | `obligation` | `P`, `finish`, the finish name | the closing-record binder node; the constraint term |
| `terminal` | `obligation` | `P`, `terminal` | the mode (`when` or `any`); the condition term for `when` |
| `sequence` | `control` | `P`, `run`, the control names from the root down | the children, in execution order |
| `choice` | `control` | as `sequence` | the owner role node; the `visible` set of expression nodes; the cases, each `label`, `guard` term and `body` control, sorted by label |
| `parallel` | `control` | as `sequence` | the memory model when written; the branches, each `label` and `body`, sorted by label; the join Group (policy, `count` for `quorum`, the predicate function node for `predicate`); the joined labels, ascending; `outstanding` when written |
| `repeat` | `control` | as `sequence` | the owner role node; the `visible` set; the maximum when written; the loop proof Group, empty or `invariant` and `variant`; the guard term; the body control; the `exhausted` control when a maximum is written |
| `await` | `control` | as `sequence` | the anchor Group (`event` or `compensation` and its target node); the clock text; the closed interval; the awaited event control; the `then` control; the `timeout` control |
| `check` | `control` | as `sequence` | the condition term |
| `send` | `control` | as `sequence` | the channel node; the payload binder node; the `related` records; the constraint term |
| `receive` | `control` | as `sequence` | the channel node; the received `send` control; the payload binder node; the `related` records; the constraint term |
| `attempt` | `control` | as `sequence` | the role node; the operation anchor node (FR-105); the memory ordering when written; the `contracts` set of state clause nodes; the record binder node; the `related` records; the constraint term |
| `effect` | `control` | as `sequence` | the `attempt` control the effect is of; the record binder node; the `related` records; the constraint term |
| `event` | `control` | as `sequence` | the role node; the compensation template node when written; the record binder node; the `related` records; the constraint term |
| `commit` | `control` | as `sequence` | the role node; the record binder node; the constraint term |
| `fence` | `control` | as `sequence` | the ordering text |
| `compensate` | `compensation` | `P`, `compensate`, the template name | the forward `effect` control and its binder node; the role node; the operation anchor node; the clock text; the registration captures; the trigger Group (binder node, guard term); the activation captures; the closed `within` interval; the attempts Group (the maximum and the attempt record type node); the retry Group (the `earlier` and `later` binder nodes and the relation term); the `commit` control, absent for `commit never`; the recovery Group (binder node, predicate term) |

- A `related` record is a relationship node, the first end term and the second
  end term of one `related by` clause. A `visible` entry, a guard, a
  constraint, a condition, a key, an initializer and an `until` term is the
  Boolean or value term FR-093 lowers the checked expression to: a literal, or
  a reference to its `expression` node.
- A binder is a `value`/`parameter` node with QSpec FR-341's body, `name` and
  `level`.
- The `relationships` set holds the nodes of the model relationships the
  `relationship` declarations bind (FR-218). The `requirements` set holds the
  temporal clause nodes the `requires temporal` members bind.
- The I2 reader returns the nodes it decodes in its fixed-depth types
  (FR-264), or the QSpec refusal with its locus.

## Behavior

### Nodes and keys

- `check` SHALL mint the key of every protocol node by FR-092 rule 2
  (`quire.application-node/v1`: tag, form, semantic type, declaration,
  `recursion` `null` and body, with the body's references by node key), and
  S4 SHALL emit exactly the nodes `check` built. A node's key therefore
  depends on its own content, on the content of every node it references and
  on its place in its protocol (its declaration path), and on nothing else
  in the source text. Reformatting the source, adding a comment or adding a
  second reference to the protocol changes no key.
- The emitter SHALL write each composite subterm as its own node reached by
  `reference` (FR-264), so no protocol body nests an application or a
  binding beyond QSpec FR-322's body grammar.
- The emitter SHALL write each node's `dependencies` as the unique
  digest-ascending `reference` targets of its body.
- The emitter SHALL write the clause node's `declaration` as `P` and each
  member's as the table gives. A role, a channel, a template and a control of
  one name in one protocol therefore have four distinct declarations. A
  control's path is its holder's path followed by its own name, and a branch
  label, a case label and the `then` and `timeout` positions of an `await`
  add no segment.
- The emitter SHALL write every optional the source does not write as the
  absent optional and every default as nothing: a `parallel` with no
  `outstanding`, a `repeat` with no maximum, an `attempt` with no ordering, an
  `event` with no compensation, a `commit never`, an activation without a
  guard, and a protocol without `terminal` or `scheduling` carry the absent
  optional. A protocol with `scheduling adversarial` and one without are two
  packages.
- The emitter SHALL write `commit never` as no `commit` argument and
  `commit N` as a reference to the control `N` names, and SHALL write a
  template's registration captures, its activation captures and the protocol
  captures as ordered records in source order.

### Binders

- The emitter SHALL write one `value`/`parameter` node for each distinct
  (name, level, type) of a binder, so every place that introduces a binder of
  those three shares the node. A binder's `semantic_type` is the type its
  construct declares.
- A binder's level is the number of binders readable where its introducing
  construct is entered (QSpec FR-464 "Level"), taken from the checked
  clause's scopes (FR-218 Scopes, FR-381 stages): `over` is level 0, the
  `each` trigger 1, each protocol capture the next, and the binders of a
  construct from its entry count in the order the construct lists them. The
  emitter SHALL NOT renumber a binder by source position.

### Order, sets and collapse

- The emitter SHALL write each set of the table (`roles`, `relationships`,
  `channels`, `queues`, `requirements`, `compensations`, `visible`,
  `contracts`) in strictly ascending node-id digest order. It SHALL write
  `queues` as one queue for each channel, ordered by each queue's own
  digest and not by its channel's, and SHALL write the cases of a `choice` and
  the branches of a `parallel` sorted by label as UTF-8 bytes and the joined
  labels in ascending label order. It SHALL write `related` records sorted by
  the relationship node's digest, then the canonical bytes of the first end,
  then those of the second.
- The emitter SHALL write the lists whose order is meaning in the order the
  checked clause holds them: a `sequence`'s children and every capture list.
  Reordering two children, or two captures, changes the node key and the
  `package_id`; reordering any set, any cases, any branches or any `related`
  clauses of the source changes neither.
- The emitter SHALL write an entry that repeats once: two `relationship`
  declarations that bind one model relationship give one `relationships`
  entry, two `related by` clauses of one node with the same relationship and
  equal ends give one record, two `visible` entries of equal expression give
  one entry, and a required temporal clause named twice gives one entry
  (FR-218). A source that differs only in such a repeat emits the same
  nodes.
- The emitter SHALL write no relationship alias, no causal edge, no
  runtime subject, no instance, no ordinal, no delivery or clock observation
  and no `internal` or `external` member on any node. The causal relation of a
  protocol is derived from its nodes by QSpec FR-464's rules, and the
  relation so derived equals the compiled edge relation of the checked clause
  (FR-218), pair of ports for pair of ports. Runtime subjects and the
  occurrence schema are derived from the nodes (FR-384).

### Occurrences

- Each protocol node SHALL carry occurrences, each with a source-map row keyed
  (node id, role, ordinal) (FR-095): one `declaration` occurrence for the
  clause, role, channel, `finish`, `terminal`, control and compensation
  nodes, at the region of the declaring construct; one `generated` occurrence
  for a queue, at the region of its channel's declaration; and, for a
  `value`/`parameter` binder node, one `expression` occurrence for each
  construct that introduces it, in source order. A queue has no
  `declaration`.
- A queue's `capacity` and `overflow` are the checked channel's, and its
  `loss` argument is present exactly when the overflow is `loss`, and is
  then `head`, `tail` or `new` for the selector the checked channel records
  (FR-218). The queue enters the key with that argument, so channels that
  differ only in selector have different queue keys.

### Refusals

The emitter refuses with typed code, cause and member from the existing
catalog, and produces no package for a refused protocol (all or nothing,
FR-105).

- A key bound at S3 that no node of the emitted graph carries (a required
  temporal clause, a model relationship, an operation anchor, a state clause)
  is stale. S4 SHALL refuse it with `missing_declaration`/`missing-name` at
  the construct that bound it, naming the bound key, and SHALL NOT look the
  name up again (FR-218).
- A protocol that holds an `await` or a `compensate` in a package whose lock
  selects no `temporal_profile` is refused with `missing_declaration`/
  `missing-selection` at the first such construct.
- A unit that selects no protocol profile emits no protocol node and no
  refusal for the declaration (FR-092-AC-18).
- A requirement cycle between a protocol and a temporal clause is refused at S3
  (FR-218). S4 treats it as unreachable and asserts that the emitted graph is
  acyclic.
- A protocol with any S3 refusal emits no protocol node (FR-113).
- The emitter adds no limit of its own, and a protocol of any size and nesting
  depth emits. The node count is bounded when the package is read, by the
  `i2.*` limits (FR-264).

### Reading the nodes back

- QSL's I2 reader SHALL admit an emitted protocol, and SHALL return, for every
  protocol node, the key, tag, form, type, declaration, body, dependencies and
  occurrences the emitter wrote.
- The protocol rebuilt from the read-back nodes SHALL equal the protocol
  rebuilt from the in-process checked clause: the same keys, the same sets
  and ordered lists in the same order, the same binder levels, the same
  channel premises and queue bounds, and the same derived causal relation.
- QSL's I2 reader SHALL refuse a node that breaks a QSpec FR-464, FR-465 or
  FR-466 rule with that requirement's code and cause at its locus, in its
  reader order. QSL's conformance run reads the QSpec protocol vectors file
  `proposals/checked-package-v2/fixtures/protocol-node-vectors.json`,
  admits each positive one and refuses each mutation with the recorded code
  and locus. It copies none of them.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-383-AC-1 | Compiling TC-1061's fixture **W** emits, for protocol `OrderFlow`, exactly one `protocol_clause`, one `role` node for each of its three roles, one `channel`, one `queue`, one `finish` and one `terminal` `obligation`, one `control` node for each of the 15 controls of its `run` tree, one `compensation` node and the binder and expression nodes they reference. Every body is a `quire.op.protocol.control` application with one `protocol_profile` law equal to the lock's row, a `profile_operator` member naming the node's operator, `null` mode and empty leaves; every `semantic_type` is the Boolean scalar type node; and the IR v2 reader admits the package. | Test (TC-1061) |
| FR-383-AC-2 | The clause node of **W** has the thirteen arguments in order. With `activation on origin` it carries `origin`, no trigger and no guard; with `on each` it carries the trigger binder node and, only when a guard is written, the guard; `on each` with no guard and `on each ... when (true)` emit different clause keys. `relationships` holds the model relationship node and no alias, `requirements` holds the temporal clause node, `terminal` and `scheduling` are present exactly when written, and a protocol with none of them has empty sets and absent optionals. | Test (TC-1061) |
| FR-383-AC-3 | A static role of **W** emits an empty `replication` Group and a model node of its model; in variants of **W**, replicated roles with lifetime `workflow`, `scope` and `until` emit the population text, `max`, the binder node, the lifetime text and, for `until` only, the `until` term; the three replicated roles and the static role have four different keys. | Test (TC-1061) |
| FR-383-AC-4 | In **W** and its variants, each channel emits `from`, `to`, `carries`, an `unordered` ordering with no binder or key or a `fifo` ordering with the message binder node and the key term, and the delivery text for each of `unknown`, `at-most-once`, `at-least-once` and `exactly-once-premise`; there is one `queue` for each channel, with a `literal` capacity value or a `symbolic` capacity name, overflow `reject`, `block` or `loss`, and a fourth `loss` argument exactly for `loss`, with `head`, `tail` and `new` giving three different queue keys; a queue has no `declaration` and one `generated` occurrence at its channel's region. | Test (TC-1061) |
| FR-383-AC-5 | `finish` emits its closing-record binder node and constraint; `terminal when {...}` emits mode `when` with the condition and `terminal any` mode `any` with none; a protocol without `terminal` has no `terminal` argument, and `scheduling adversarial` emits the text `adversarial` in the clause, a protocol without it the absent optional. | Test (TC-1061) |
| FR-383-AC-6 | In **W** and its variants, `sequence`, `choice`, `parallel` and `repeat` emit their layouts: a `sequence` its children in order and an empty `sequence` an empty list; a `choice` its owner, its `visible` set and at least two cases sorted by label; a `parallel` each of the join policies `all`, `any`, `quorum` (with `count`) and `predicate` (with the function node), the memory model and `outstanding` exactly when written, and the joined labels in ascending order; a `repeat` with a maximum and an `exhausted` control, with a loop proof, with both and with neither, with no `exhausted` argument when no maximum is written and `maximum` 0 emitted as 0. | Test (TC-1061) |
| FR-383-AC-7 | In **W** and its variants, `await`, `check`, `send`, `receive`, `attempt`, `effect`, `event`, `commit` and `fence` emit their layouts: an `await` anchored at an event node and at a compensation template with its clock text, closed interval, awaited event, `then` and `timeout`; a `send` and a `receive` with the channel, the payload binder, the received `send` and the `related` records; an `attempt` with its role, operation anchor, each memory ordering or none, `contracts` and binder; an `effect` of its `attempt`; an `event` with and without a compensation; a `commit`; a `fence` with each ordering. | Test (TC-1061) |
| FR-383-AC-8 | Each `compensate` template emits its thirteen arguments: the forward effect and binder, role, operation anchor, clock, registration captures, trigger, activation captures, `within`, attempts maximum and record type, the retry binders and relation, the commit control or, for `commit never`, no commit, and the recovery binder and predicate; a `Full` template with commit `Finalized` and a `Partial` template with `commit never` for one effect emit two nodes with different keys. | Test (TC-1061) |
| FR-383-AC-9 | A role, a channel, a compensation template and a control all named `X` in one protocol emit four nodes with four distinct `declaration.qualified_name` values; a control's declaration is its holder's followed by its own name, with no segment for a branch, a case or the `then` and `timeout` positions; every declaring construct has one `declaration` occurrence at its region and a source-map row keyed (node id, `declaration`, 0), a queue has one `generated` occurrence and none else, and a binder node has one `expression` occurrence for each introducing construct, in source order. | Test (TC-1062) |
| FR-383-AC-10 | Binders emit with the levels QSpec FR-464 fixes: in **W**, `view` is level 0, `opened` 1, the capture `order` 2, `sent` the entry count of `Requested`; two constructs that introduce a binder of equal name, level and type share one node with two `expression` occurrences, and a binder is never numbered by source position. | Test (TC-1062) |
| FR-383-AC-11 | Every guard, constraint, condition, key, initializer and `until` term is a literal or a reference to its FR-093 `expression` node, and a walk of the emitted JSON classifies every protocol body position as a Leaf, Group, Tuple, Member or Body of QSpec FR-322's stratified grammar, with no nested application. | Test (TC-1062) |
| FR-383-AC-12 | Declaring the roles, channels, compensations, `visible` entries, `contracts`, cases, branches, joined labels and `related by` clauses of **W** in a different order emits the same nodes in the same bytes and the same `package_id`; every set is in ascending digest order, the queues are ordered by their own digests (a source whose channel order differs from the queue-digest order still emits the queue-digest order), and cases and branches are sorted by label as UTF-8 bytes. Swapping two children of a `sequence`, or two captures, changes the node key and the `package_id`. | Test (TC-1062) |
| FR-383-AC-13 | Two `relationship` declarations binding one model relationship emit one `relationships` entry; two equal `related by` clauses on one `send`, two equal `visible` entries and `requires temporal Due` written twice each emit one entry; each such source emits the same bytes as the source without the repeat. | Test (TC-1062) |
| FR-383-AC-14 | Each protocol node's key equals the SHA-256 of the RFC 8785 bytes of its `quire.application-node/v1` preimage computed independently. Reformatting **W**, adding a comment and adding a second `protocol OrderFlow` reference leave every protocol key and the `package_id` unchanged; renaming a role, retargeting a reference, changing one literal, changing a capacity, a selector, a join policy or a loop-proof term, and adding `scheduling adversarial` each change the changed node's key, the clause key and the `package_id`. | Test (TC-1062) |
| FR-383-AC-15 | Emitting **W** twice, and in a unit that declares an unrelated protocol before it and one after it, gives the same protocol nodes, bytes and `package_id` for **W**; the clause key changes when, and only when, any node it reaches changes. | Test (TC-1062) |
| FR-383-AC-16 | No emitted node carries a causal edge, a relationship alias, a runtime subject, an instance, an ordinal, a delivery or clock observation or an `internal`/`external` member. The causal relation derived from **W**'s emitted nodes by QSpec FR-464's rules holds the same (source port, target port) pairs as the compiled edge relation of **W**'s checked clause, back edges of the `repeat` included. | Test (TC-1062) |
| FR-383-AC-17 | A graph in which **W**'s required temporal clause key, its model relationship key or an attempt's operation anchor key is bound to a key no emitted node carries refuses `missing_declaration`/`missing-name` at the construct that bound it, naming the bound key, with no protocol node and no package; **W** with an `await` in a package whose lock selects no `temporal_profile` refuses `missing_declaration`/`missing-selection` at the `await`, and so does a `compensate`; a unit that selects no protocol profile emits no protocol node and no refusal for the declaration; a requirement cycle never reaches S4 (FR-218-AC-25). | Test (TC-1063) |
| FR-383-AC-18 | A protocol with an S3 refusal (a join naming a missing branch) in a unit that also holds a good protocol emits no node of either protocol and no package; emitting the good protocol alone succeeds. | Test (TC-1063) |
| FR-383-AC-19 | Reading **W**'s package through QSL's I2 reader admits it and returns every protocol node with the key, tag, form, type, declaration, body, dependencies and occurrences emitted; the protocol rebuilt from the read-back nodes equals the protocol rebuilt from the in-process checked clause in keys, set order, ordered lists, binder levels, channel premises, queue bounds and derived causal relation. | Test (TC-1063) |
| FR-383-AC-20 | Over **W**'s package, each mutation of the table in TC-1063 (a set out of ascending digest order, a repeated set entry, a channel with no queue, a second queue for one channel, a control held by nothing, a `receive` sequenced before its `send`, an argument missing, a layout out of order, a model of the wrong kind) is refused by QSL's I2 reader with the code, cause and locus QSpec FR-464, FR-465 or FR-466 gives it, and QSL's conformance run admits each positive and refuses each mutation of the QSpec protocol vectors file `protocol-node-vectors.json`. | Test (TC-1063) |
| FR-383-AC-21 | A protocol whose controls outnumber `i2.nodes`, emitted with the limit raised, reads back under the default limit as `StageFailure::Limit` of kind node count with setting `i2.nodes`, and verifies when the setting is raised; a protocol with a `parallel` inside a `repeat` inside a branch of another `parallel`, three levels, and a `sequence` of 5,000 children each emit with no limit or depth refusal at S4. | Test (TC-1063) |

## Dependencies

- ADR-011 §6.1 layer 4 `package` and §2.1 E4; ADR-027 §9 DS-1.
- [FR-218](FR-218-check-every-protocol-control-construct-at-s3.md) and
  [FR-381](FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md)
  (the checked clause), [FR-092](FR-092-key-type-parameter-and-declared-nodes.md)
  (node keys and the clause node),
  [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md)
  (lowering a term), [FR-095](FR-095-occurrence-keyed-source-map-and-locus.md)
  (occurrences and the source map),
  [FR-105](FR-105-emit-state-nodes.md) (operation anchors and state clauses),
  [FR-264](FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md) (the
  stratified grammar and the read limits).
- [FR-384](FR-384-derive-the-protocol-occurrence-schema-and-runtime-binding-requirements.md)
  (what S4 derives from these nodes).

## References

- QSpec FR-464 (the protocol node family: operation, keys, clause body,
  binders, reader order), FR-465 (role, channel, queue, `finish` and
  `terminal` bodies), FR-466 (control and compensation bodies), FR-322 (the
  artifact and its reader order) and the vectors file `protocol-node-vectors.json` and QSpec TC-946, TC-947 and TC-948.
- [FR-337](FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md)
  (the temporal clause emission this requirement follows).
