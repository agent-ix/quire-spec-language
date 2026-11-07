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
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-503
    type: references
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
is the compiler side of QSpec FR-464, FR-465 and FR-466: those requirements
own the body forms, operation identity, operator catalog, member kinds and
reader refusals; QSL emits them from its checked clause and reads them
through its I2 reader. The nodes are the only carrier of the protocol. S4
writes no separate protocol document.

Each emitted `protocol_clause` node SHALL carry the node-level
`owner_model: NodeRef` required by QSpec FR-464. S4 obtains the exact model
declaration node from the checked clause's model context, writes it as a local
reference, and includes it in the clause's application-node identity
preimage and exact `dependencies`. It SHALL NOT derive ownership from the
`over` parameter's type during package reading. The I2 reader SHALL return the
edge unchanged and refuse a missing edge, an external or absent target, a
non-model target, or a target omitted from exact dependencies.

The clause's model context is the object type its `over` input references
(`over (input: M::T)`, a `Reference<T>` to an object type of a selected model),
and the edge is the model declaration node of that object type, as QSpec's
published fixtures write it. A clause whose `over` input is any other type (a
scalar, an `Option`, a collection) has no model declaration node to name: S4
refuses the protocol typed, `unsupported_construct`/`not-yet-implemented`
naming the construct `NoOwnerModel` (QSL-1147), and a reference to it,
and writes no clause node, rather than emit a clause the reader refuses.

## Use case

A verification operator compiles a unit with an order protocol: three roles,
a FIFO channel, a `parallel` joined on both branches, a `choice`, a bounded
`repeat`, a refund template and a `finish`. The package carries the protocol
as protocol nodes that any QSpec reader admits, so a backend, a model checker
or a conformance assessor reads the same roles, queue bounds, controls and
recovery rules from the package and from nothing else, and a reformatted
source or a reordered declaration list gives the same package.

## Semantic authority and boundary

QSpec FR-464 owns the node family, the key inputs including `owner_model`, the clause body, the
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
- The package's clause profile selections: the `protocol_profile` row of
  `profile_selections`, and its `temporal_profile` row when the protocol
  holds an `await` or a `compensate`.
- The nodes of the same graph the protocol names: its model and relationship
  nodes (FR-094), the operation anchor and state clause nodes of its attempts
  and templates (FR-105), the temporal clause nodes it requires (FR-337), its
  type nodes and its value and parameter nodes (FR-092, FR-093).

## Outputs

S4 emits one node for each row below, for every checked protocol. The closed
body and key shapes are those QSpec FR-464, FR-465 and FR-466 define; the
table records QSL's mapping from checked protocol facts to their published
arguments. `P` is the protocol's qualified name.

| Operator | `semantic_form` | `declaration.qualified_name` | Arguments, in order, written from |
| --- | --- | --- | --- |
| `protocol` | `protocol_clause` | `P` | `over`, a reference to the `over` parameter node; `activation`, `origin`, or `each` with the trigger binder node and the guard when one is written; `captures`, the protocol captures in source order; `roles`, `relationships`, `channels`, `queues`, `requirements`, `compensations`, each a set; `terminal`, the `terminal` node when one is written; `scheduling`, the text `adversarial` when written; `run`, the root control; `finish`, the `finish` node |
| `role` | `role` | `P`, `role`, the role name | the node of the role's model, or for a role on a whole model (FR-218, `role w on M`) the text `identity` of the package `M` selects and no model node (QSpec FR-465); the replication Group, empty for a static role and, for a replicated role, its population text, `max`, binder node, lifetime (`workflow`, `scope` or `until`) and the `until` term for `until`; for a role on a Port (`role p on M::port`) a third `model_member` Group of the texts `package`, the package identity that declares the Port, and `node`, the Port's declaration identity (QSpec FR-465), and no third argument for any other role |
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
  end term of one `related by` clause. A guard, constraint, condition, key,
  initializer and `until` term is the Boolean or value term FR-093 lowers the
  checked expression to: a literal, or a reference to its `expression` node.
  Every `visible` entry is a reference to its FR-093 `expression` node: FR-218
  has already refused bare literals and bare captures at S3, so S4 has no
  `NotEmitted{VisibleEntryNotAnExpression}` refusal path.
- A binder is a `value`/`parameter` node with QSpec FR-341's body, `name` and
  `level`.
- The `relationships` set holds the nodes of the model relationships the
  `relationship` declarations bind (FR-218). The `requirements` set holds the
  temporal clause nodes the `requires temporal` members bind.
- The I2 reader returns the nodes it decodes in its fixed-depth types
  (FR-264), or the QSpec refusal with its locus.
- S4 writes the rows above. A protocol with a selected profile whose checked
  graph holds static and replicated roles on a model type, channels with their
  queues, a run tree of any of the thirteen control forms, requirements,
  compensation templates, a `finish`, an `origin` or `each` activation (with or
  without a guard), protocol captures and a `scheduling` member emits the clause with its
  thirteen arguments in order (every set it does not hold an empty Group),
  with `relationships` naming its checked model relationship nodes once each
  in ascending node id order, the `over` and trigger binders at levels 0 and 1,
  the capture binders at the consecutive levels that follow them (the count of
  `over` and the trigger, then each later capture the next level) with the
  `captures` argument one `binder`/`initializer` Group per capture in source
  order, an `each` guard as the `guard` member of the `activation` Group, a
  replicated role's binder, a FIFO channel's message binder and the `finish`
  binder at the level that follows the captures, and each role, channel, control, compensation template and `finish` node at its
  declaration path and each queue with none; the `roles`,
  `channels` and `queues` sets are ascending by node id (QSpec FR-322 node `id`) whatever the
  source order. A protocol that holds any other construct refuses
  `unsupported_construct`/`not-yet-implemented` at its declaration, once for
  each such construct and naming it, and so does a reference to it: a `terminal when` condition that reads `over` under
  `activation on each` (IR-1069); a function's or protocol's reference to a
  protocol that holds an `attempt`, whose node is written after the state
  clauses it lists, which follow the functions (QSL-1121g); a protocol in a cyclic component
  of function calls and protocol references, whose recursion group is not
  written yet (QSL-844: a body that calls a function, or refers to a protocol,
  is keyed after that function or protocol, QSL-1121g, so only a cycle is
  refused); a channel that carries a bounded scalar or collection
  (QSL-1137: a `bounded_domain` node, which the reader does not admit as
  `carries`; a quantity, a nominal enum, an `option`, an unbounded collection
  and a declared record are admitted); a FIFO channel whose key type is not equality-stable
  (QSL-1142: the reader accepts only Boolean, Integer, Rational, Decimal,
  Text, String, UUID, Timestamp and enum scalars, references, and composites
  of them, so a key that is or contains a bounded domain, a quantity, a float
  or a model-owned type is refused); and a role on a whole model (IR-874: the reader admits
  only the reference form of the role's `model`, so the text form QSpec FR-465
  specifies cannot be proved admitted). A protocol is never written without a
  part of its graph.

## Behavior

### Protocol term locations

A *slot walk* is the sequence of positions a pre-order walk visits in the
checked expression tree of one protocol slot expression, from the slot's root.
The tree is the checked clause's tree held in memory: a node reached through two parents
is one graph node but two positions, each walked in full. Children are taken in
the operand order of the node's FR-400 `NodeKind` (a `let`'s value before its
body, a conditional's condition, then-branch and else-branch, a case's scrutinee
and then its arms in the emitted arm order), never in the order of the node's
`dependencies` array. The walk takes every branch and arm whether or not any
input evaluates it, so a branch a short-circuit never reaches still holds its
positions. A reference to a `let` binder or other local (`Local`) is a leaf
position and does not walk the bound application again; a literal is a leaf.
A *term* is a position whose node is an `expression`/`application` node of QSpec
FR-322-AC-291 (an application, such as a connective, comparison, arithmetic
operation, field read, dereference, call or collection query). Every other
position, namely a leaf (a literal, a `Local` or another reference) and an
expression form FR-322 does not call an application (a `let`, a conditional),
is not a term.

The origin of a row is the closed origin of QSpec FR-322: a named type
declaration (`kind` `type_declaration`, `name`) or a protocol attempt (`kind`
`protocol_attempt`, `protocol` position, `attempt` position). Every
slot expression of a protocol is located at the root of a type-declaration
origin whose `name` is the slot's site name, a spaced string that names the
protocol and the slot, one distinct name per slot (QSpec FR-322-AC-313 to
AC-318). A `protocol_attempt` origin identifies one attempt's own root and is the
origin of no slot expression (FR-322-AC-316). The forms of QSpec FR-322's
site-name registry that a slot expression can have are:

| Slot | Site name |
| --- | --- |
| activation guard | `protocol <P> activation guard` |
| capture initializer | `protocol <P> capture <name> initializer` |
| registration capture, trigger guard, activation capture, retry relation, recovery predicate of a compensation `<N>` | `protocol <P> compensate <N> <stage> <index>`, the stage being `registration-capture`, `trigger-guard`, `activation-capture`, `retry` or `recovery`, and the index the capture's position (zero for the other three) |
| constraint or condition of the control at path `<path>` | `protocol <P> run <path> body`, the path from the run root with the root's name first, joined by spaces |
| `related` end | `protocol <P> run <path> related <k> from` or `... related <k> to`, `k` the zero-based position of the record in the node body's emitted `related` array (sorted and collapsed by "Order, sets and collapse", not a source position) |
| case guard | `protocol <P> run <path> case <label> guard`, `label` the case's label |
| `visible` entry | `protocol <P> run <path> decision <start>..<end>`, by the entry's source span; the span is a source-route alias, the slot's identity is its control node, member and ordinal (FR-390, FR-533) |
| `finish` constraint, `terminal when` condition, role `until`, channel key | `protocol <P> finish <name> body`, `protocol <P> terminal body`, `protocol <P> role <role> until`, `protocol <P> channel <channel> key` |

The emitter SHALL write each slot's rows under that slot's own site name as the
origin `name`, and no two slots of a package share a site name. *Origin order*
is the ascending order of the `name` in UTF-8 byte order.

The emitter SHALL write, in the `checked_term_data.protocol_locations` array of
QSpec FR-322, one row for every term of every protocol slot expression it
emits.

Each row SHALL carry the application node, the term's semantic location as the
checker assigned it to that position, its origin (above), the child-index path
of the position and a zero-based `term_ordinal`.

The emitter SHALL take each slot's origin name from the site at which the
checker located that slot, and SHALL NOT invent a name. The name of a case guard
and of a `related` end is built from the emitted control node: the case's label,
and the record's position in the `related` list as the emitter writes it (Order,
sets and collapse), so that two case guards, or two `related` ends, whose
expressions are one graph node have distinct names and each pairs with its own
case or end (QSpec FR-322-AC-388).

A case's label is known where the checker locates the guard, so the checker
locates a case guard at its final name. A record's position is not: it depends
on the canonical bytes of the relationship node and of both ends, which exist
only once lowering has built those nodes. The checker SHALL therefore locate
each `related` end at a *provisional site*, `protocol <P> run <path> related
<start> <end>` with the end's source span, and lowering SHALL fix the name:

- WHEN lowering writes a control's `related` records sorted and collapsed, it
  SHALL give each written record its zero-based position `k` and SHALL replace,
  in every `Location` of the checked term of that record's first end, the
  provisional origin by `protocol <P> run <path> related <k> from`, and in every
  `Location` of its second end's term by `... related <k> to`. The replacement
  changes the origin `name` only; each `Location`'s child-index path, each
  occurrence and each source-map row are kept.
- Two `related by` clauses that collapse into one record are one record `k`. The
  ends of the clause written first are the record's ends; the ends of each later
  repeat are re-located to the same two names, and the emitter writes rows for
  the record's ends only, so no origin holds rows of two clauses.
- The re-location is part of `check`: the checked package that `check` returns,
  the in-process route that evaluates over it (FR-390), and the
  `protocol_locations` rows the emitter writes all carry the position name. No
  provisional site name is held by a checked package, written to a row, or
  returned in an evaluation's `Location` on either route. A check refusal raised
  while the checker types an end, before lowering, keeps the provisional site,
  because no record position exists for a unit that does not check.

The `Location` of an application-rooted `related` end is therefore the
position-named one on the source route as well as the bytes route, which is the
exact parity of FR-533-AC-13. A `repeat`'s
`proof.invariant` and `proof.variant` are not slots (FR-390), and the emitter
writes no row for them.

The *visit number* `k` of a term is the zero-based count of earlier positions of
the same application in its slot's walk.

A row's *occurrence* is the source occurrence key (`node_id`, role, ordinal) of
QSpec FR-322 that the emitter keys the term to, and its *occurrence ordinal* is
the third component of that key. The group of FR-322-AC-291 is the rows of one
application and one occurrence, and its `term_ordinal` values run consecutively
from zero across every origin the group holds; they do not restart for each
origin. A group holds rows of several origins when the general rule below keys
visits of several slots to one occurrence, which happens where the checker
recorded no `expression` occurrence at those visits' positions. Within a group the
emitter SHALL write the rows of the origins in origin order, and within one
origin in visit order.

For each application and slot, the number of rows of the slot's origin SHALL
equal the number of visits of that application in the slot's walk; no row is
dropped, repeated or padded. The emitter SHALL key the visits of one application
in a slot's walk to the application's occurrences in non-decreasing occurrence
ordinal (document order of the occurrences), and within one occurrence give
consecutive `term_ordinal` values in visit order, so that the ascending
(occurrence ordinal, `term_ordinal`) order of that origin's rows of the
application is the order of its visits: the row at that order's index `k`
belongs to the visit numbered `k`. A node with one occurrence is the case in
which every visit takes that occurrence.

The checker records one `expression` occurrence of an application each time it
lowers the term at a position of a slot expression, so an application written at
two positions, in one slot or in two, has two occurrences, numbered in the order
the checker lowers them (measured as document order for fixtures E and E-C,
FR-383-AC-61). WHERE the checker recorded an `expression` occurrence at a
visit's position, the emitter SHALL key that visit to it; otherwise the general
rule above applies. The emitter SHALL NOT key a visit whose position has a
recorded occurrence to an occurrence recorded at another position.

The emitter SHALL NOT derive a row's location from a source region, a node
order, an operand position or a default value.

WHEN a unit holds no protocol slot expression, the emitter SHALL write the
array empty; WHEN a package has no checked-term data, the emitter SHALL keep the
sidecar absent as before.

In TC-1070 fixture E the retry relation of `Refund` (origin `protocol Refunds
compensate Refund retry 0`, the *retry origin* of `Refund`) holds one
`earlier.tally` application node reached at two positions, in `earlier.tally < 3`
(visit 0) and in `earlier.tally + 1` (visit 1). No other slot holds it, so it has
two rows of the retry origin: the first carries the location of the term in
`earlier.tally < 3` and the second that in `earlier.tally + 1`. The checker
records two `expression` occurrences of the node, ordinal 0 at the term in
`earlier.tally < 3` and ordinal 1 at the term in `earlier.tally + 1`. The first
row is keyed to occurrence 0 and the second to occurrence 1. Each row is the
only row of its group, so both have `term_ordinal` 0. Their ascending (occurrence
ordinal, `term_ordinal`) order, (0, 0) then (1, 0), is the visit order. This
is fixture E's two-occurrence case.

In the same fixture the trigger guard (origin `protocol Refunds compensate Refund
trigger-guard 0`) and the retry relation both read the capture `target`, so
`target >= 0` is one application node held by two slots. The checker records two
`expression` occurrences of it: ordinal 0 at its term in the trigger guard, which
comes first in document order, and ordinal 1 at its term in the retry relation.
The trigger-guard row is keyed to occurrence 0 and the retry row to occurrence 1.
Each row is alone in its group with `term_ordinal` 0.

Assumption A1 (non-term locations, one place): a position that is not a term
has no row and takes its location from its own admitted node. For a leaf this is
QSpec FR-322-AC-310 to AC-312: `protocol_locations` is application-only, a leaf
takes its location from its own admitted node and never from an enclosing
application's row, and a row naming a leaf is refused `invalid_package`/
`invalid-value` at the row's `application` member. A row naming any other
non-application node stays under FR-322-AC-291. FR-322 states the leaf rule only;
this text extends the same location source to a `let` and a conditional, which
FR-322 does not call applications. Every clause of this and the reader's text
that depends on A1 refers to it.

### Cyclic reference components

A protocol node that
belongs to a cyclic strongly connected component of checked-node references
uses the same `recursion_group`, `{size, ordinal}` preimage coordinates and
in-group `group_reference` terms that FR-092 and QSpec FR-322 define for other
recursive nodes. The group includes every node in that component, regardless
of its `node_tag`. A protocol node outside a cyclic component keeps
`recursion_group` absent and `recursion` `null`. Acyclic node keys and bodies
are unchanged. This option admits a predicate-join function that refers back
to its own protocol when those references form a cycle.

Finding cyclic components SHALL use an iterative graph algorithm with an
explicit work stack and SHALL NOT use native call-stack recursion. Component
discovery and membership assignment SHALL take O(V + E) time in the checked
reference graph, where V is its nodes and E its edges. The emitter SHALL NOT
merge components by repeatedly scanning or copying existing groups, and SHALL
compute each group's structural identity once after its canonical membership and
coordinates are established. This work SHALL be charged to the caller's
existing shared `s3.work_units` budget (ADR-030); the emitter SHALL add no
fixed depth, group-size, node-count, or group-count limit. When that budget is
exhausted, the existing typed limit refusal SHALL name `s3.work_units`. The
emit path SHALL NOT use a global lock or serialize independent package
emissions.

### Nodes and keys

- `check` SHALL mint the key of every protocol node by FR-092 rule 2
  (`quire.application-node/v1`: tag, form, semantic type, declaration,
  `recursion` `null` for acyclic nodes or the FR-322 group coordinate for a
  cyclic component, and body with same-group references represented by
  `group_reference`), and
  S4 SHALL emit exactly the nodes `check` built. A node's key therefore
  depends on its own content, on the content of every node it references and
  on its place in its protocol (its declaration path), and on nothing else
  in the source text. Reformatting the source, adding a comment or adding a
  second reference to the protocol changes no key.
- The emitter SHALL write each composite subterm as its own node reached by
  `reference` (FR-264), so no protocol body nests an application or a
  binding beyond QSpec FR-322's body grammar.
- The emitter SHALL write each node's `dependencies` as the unique
  node-id-ascending `reference` targets of its body.
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
  `contracts`) in strictly ascending node-id order. It SHALL write
  `queues` as one queue for each channel, ordered by each queue's own
  node id and not by its channel's, and SHALL write the cases of a `choice` and
  the branches of a `parallel` sorted by label as UTF-8 bytes and the joined
  labels in ascending label order. It SHALL write `related` records sorted by
  the relationship node's id, then the canonical bytes of the first end,
  then those of the second.
- The emitter SHALL write a predicate join's `predicate` member as a
  reference to the `function` node whose key the checked policy
  `Predicate{function}` holds (FR-218, Predicate joins), and the package
  SHALL hold that `function` node as FR-092 emits every function. Two joins
  that name one function reference one node. The function's parameters bind
  the joined labels in the ascending order the `joined` Group is written in,
  so reordering the source list changes neither the bytes nor the meaning.
  The read-back of the `parallel` node gives the same checked policy.
- The emitter SHALL write the lists whose order is meaning in the order the
  checked clause holds them: a `sequence`'s children and every capture list.
  Reordering two children, or two captures, changes the emitted node bytes;
  reordering any set, any cases, any branches or any `related`
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
catalog. The package is all or nothing for the unit (FR-105): a unit with any
refused protocol produces no package. The protocols are independent of one
another (FR-042-AC-8): each protocol has its own result, and one protocol's
refusal changes no other protocol's result (see "Per-protocol results").

- A key bound at S3 that no node of the emitted graph carries (a required
  temporal clause, a model relationship, an operation anchor, a state clause)
  is stale. S4 SHALL refuse it with `missing_declaration`/`missing-name` at
  the construct that bound it, naming the bound key, and SHALL NOT look the
  name up again (FR-218).
- A protocol that holds an `await` or a `compensate` in a package that
  selects no `temporal_profile` is refused with `missing_declaration`/
  `missing-selection` at the first such construct.
- A unit that selects no protocol profile emits no protocol node and no
  refusal for the declaration (FR-092-AC-18).
- A requirement cycle between a protocol and a temporal clause is refused at S3
  (FR-218). S4 treats it as unreachable and asserts that the emitted graph is
  acyclic.
- A protocol with an S3 refusal emits no protocol node of its own (FR-113).
  It does not suppress the nodes of any other protocol of the unit, which
  keep their own results (see "Per-protocol results").
- The emitter adds no limit of its own, and a protocol of any size and nesting
  depth emits. The node count is bounded when the package is read, by the
  `i2.*` limits (FR-264).

### Per-protocol results

- The protocols are processed in source order. S1 to S4 SHALL produce exactly
  one `ProtocolResult` for each protocol declaration the parser delimited, in
  declaration order: `Emitted { declaration, clause key, nodes }`, `Refused {
  declaration, code, cause, locus }` whose locus pairs the source index and span
  of the owning declaration (FR-042-AC-8), or `Incomplete { declaration, limit }`
  carrying the typed limit refusal (`StageFailure::Limit`, `stage_limit_exceeded`)
  that names `s3.work_units`. An operation that stops before S4 (`parse`,
  `check`; FR-503) reports `Cleared { declaration, stage }` for a declaration
  that passed every stage it ran, and only `compile` produces `Emitted`.
- Independence SHALL hold at every stage whose work is per protocol: a refusal
  at S1, S2, S3 or S4 that the stage attributes to one protocol declaration
  gives that protocol a `Refused` result and changes no result of a protocol that
  does not reach it. A refusal that the stage cannot attribute to one protocol
  (a syntax error the parser cannot confine to one declaration, the unit
  header, a profile selection or a model import) refuses the unit with no
  per-protocol results.
- A protocol that reaches a `Refused` protocol X, by reference or requirement,
  SHALL be `Refused` `missing_declaration`/`missing-name` at the reference. The
  catalog has no dependency cause, so the record SHALL carry the field
  `refused_dependency`, naming X and its own code and cause, and the field
  `consequence` set true; the record says the declaration exists and was
  refused, not that it is missing.
- The shared `s3.work_units` budget is consumed in source order. When a protocol
  exhausts it, that protocol and every later protocol SHALL be `Incomplete`, and
  every earlier result stands. Declaration order therefore changes results
  only when the budget decides them; every non-budget outcome is independent of
  declaration order.
- The unit aggregate SHALL be `Refused` when any result is `Refused`,
  `Incomplete` when none is `Refused` and any is `Incomplete`, and `Emitted`
  otherwise. Only an `Emitted` aggregate carries a package and a `package_id`.
  A `Refused` aggregate is carried as `StageFailure::Refused` whose code, cause
  and locus are those of the first `Refused` result in declaration order and
  whose payload holds every `ProtocolResult`, so the results of the protocols
  that did emit remain readable. An `Incomplete` aggregate is carried as
  `StageFailure::Limit` with the same payload. The aggregate never reports a
  unit with a refused or incomplete protocol as admitted.
- The unit's diagnostics SHALL list one record (code, cause, locus) for each
  `Refused` result in declaration order and none for an `Emitted` result. The
  exit is FR-285's: the items are the per-protocol results, and the multi-item
  rule of FR-285-AC-2 decides the unit's exit, so a unit with a refused
  protocol never exits 0.
- A protocol refused `unsupported_construct`/`not-yet-implemented` (the
  `NoOwnerModel` case) has the same standing as an S3 refusal.

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
| FR-383-AC-1 | Compiling TC-1061's fixture **W** emits, for protocol `OrderFlow`, exactly one `protocol_clause`, one `role` node for each of its three roles, one `channel`, one `queue`, one `finish` and one `terminal` `obligation`, one `control` node for each of the 15 controls of its `run` tree, one `compensation` node and the binder and expression nodes they reference. Every body is a `quire.op.protocol.control` application with one `protocol_profile` law equal to the package's row, a `profile_operator` member naming the node's operator, `null` mode and empty leaves; every `semantic_type` is the Boolean scalar type node; the clause has `owner_model` equal to its exact model declaration node in the clause's dependencies and key preimage; and the IR v2 reader admits the package. | Test |
| FR-383-AC-2 | The clause node of **W** has the thirteen arguments in order. With `activation on origin` it carries `origin`, no trigger and no guard; with `on each` it carries the trigger binder node and, only when a guard is written, the guard; `on each` with no guard and `on each ... when (true)` emit different clause keys. `relationships` holds the model relationship node and no alias, `requirements` holds the temporal clause node, `terminal` and `scheduling` are present exactly when written, and a protocol with none of them has empty sets and absent optionals. | Test |
| FR-383-AC-3 | A static role of **W** emits an empty `replication` Group and a model node of its model; in variants of **W**, replicated roles with lifetime `workflow`, `scope` and `until` emit the population text, `max`, the binder node, the lifetime text and, for `until` only, the `until` term; the three replicated roles and the static role have four different keys. | Test |
| FR-383-AC-4 | In **W** and its variants, each channel emits `from`, `to`, `carries`, an `unordered` ordering with no binder or key or a `fifo` ordering with the message binder node and the key term, and the delivery text for each of `unknown`, `at-most-once`, `at-least-once` and `exactly-once-premise`; there is one `queue` for each channel, with a `literal` capacity value or a `symbolic` capacity name, overflow `reject`, `block` or `loss`, and a fourth `loss` argument exactly for `loss`, with `head`, `tail` and `new` giving three different queue keys; a queue has no `declaration` and one `generated` occurrence at its channel's region. | Test |
| FR-383-AC-5 | `finish` emits its closing-record binder node and constraint; `terminal when {...}` emits mode `when` with the condition and `terminal any` mode `any` with none; a protocol without `terminal` has no `terminal` argument, and `scheduling adversarial` emits the text `adversarial` in the clause, a protocol without it the absent optional. | Test |
| FR-383-AC-6 | In **W** and its variants, `sequence`, `choice`, `parallel` and `repeat` emit their layouts: a `sequence` its children in order and an empty `sequence` an empty list; a `choice` its owner, its set of visible expression-node references and at least two cases sorted by label; a `parallel` each of the join policies `all`, `any`, `quorum` (with `count`) and `predicate` (with the function node), the memory model and `outstanding` exactly when written, and the joined labels in ascending order; a `repeat` with a maximum and an `exhausted` control, with a loop proof, with both and with neither, with no `exhausted` argument when no maximum is written and `maximum` 0 emitted as 0. | Test |
| FR-383-AC-7 | In **W** and its variants, `await`, `check`, `send`, `receive`, `attempt`, `effect`, `event`, `commit` and `fence` emit their layouts: an `await` anchored at an event node and at a compensation template with its clock text, closed interval, awaited event, `then` and `timeout`; a `send` and a `receive` with the channel, the payload binder, the received `send` and the `related` records; an `attempt` with its role, operation anchor, each memory ordering or none, `contracts` and binder; an `effect` of its `attempt`; an `event` with and without a compensation; a `commit`; a `fence` with each ordering. | Test |
| FR-383-AC-8 | Each `compensate` template emits its thirteen arguments: the forward effect and binder, role, operation anchor, clock, registration captures, trigger, activation captures, `within`, attempts maximum and record type, the retry binders and relation, the commit control or, for `commit never`, no commit, and the recovery binder and predicate; a `Full` template with commit `Finalized` and a `Partial` template with `commit never` for one effect emit two nodes with different keys. | Test |
| FR-383-AC-9 | A role, a channel, a compensation template and a control all named `X` in one protocol emit four nodes with four distinct `declaration.qualified_name` values; a control's declaration is its holder's followed by its own name, with no segment for a branch, a case or the `then` and `timeout` positions; every declaring construct has one `declaration` occurrence at its region and a source-map row keyed (node id, `declaration`, 0), a queue has one `generated` occurrence and none else, and a binder node has one `expression` occurrence for each introducing construct, in source order. | Test (TC-1062) |
| FR-383-AC-10 | Binders emit with the levels QSpec FR-464 fixes: in **W**, `view` is level 0, `opened` 1, the capture `order` 2, `sent` the entry count of `Requested`; two constructs that introduce a binder of equal name, level and type share one node with two `expression` occurrences, and a binder is never numbered by source position. | Test (TC-1062) |
| FR-383-AC-11 | Every guard, constraint, condition, key, initializer and `until` term is a literal or a reference to its FR-093 `expression` node, and a walk of the emitted JSON classifies every protocol body position as a Leaf, Group, Tuple, Member or Body of QSpec FR-322's stratified grammar, with no nested application. | Test (TC-1062) |
| FR-383-AC-12 | Declaring the roles, channels, compensations, `visible` entries, `contracts`, cases, branches, joined labels and `related by` clauses of **W** in a different order emits the same nodes in the same bytes; every set is in ascending node-id order, the queues are ordered by their own node ids (a source whose channel order differs from the queue node-id order still emits the queue node-id order), and cases and branches are sorted by label as UTF-8 bytes. Swapping two children of a `sequence`, or two captures, changes the emitted node bytes. | Test (TC-1062) |
| FR-383-AC-13 | Two `relationship` declarations binding one model relationship emit one `relationships` entry; two equal `related by` clauses on one `send`, two equal `visible` entries and `requires temporal Due` written twice each emit one entry; each such source emits the same bytes as the source without the repeat. | Test (TC-1062) |
| FR-383-AC-14 | Reformatting **W**, adding a comment and adding a second `protocol OrderFlow` reference leave every protocol node unchanged; renaming a role, retargeting a reference, changing one literal, changing a capacity, a selector, a join policy or a loop-proof term, changing the clause's `owner_model`, and adding `scheduling adversarial` each change the changed node and the clause node that reaches it. | Test (TC-1062) |
| FR-383-AC-15 | Emitting **W** twice, and in a unit that declares an unrelated protocol before it and one after it, gives the same protocol nodes and bytes for **W**; the clause node changes when, and only when, any node it reaches changes. | Test (TC-1062) |
| FR-383-AC-16 | No emitted node carries a causal edge, a relationship alias, a runtime subject, an instance, an ordinal, a delivery or clock observation or an `internal`/`external` member. The causal relation derived from **W**'s emitted nodes by QSpec FR-464's rules holds the same (source port, target port) pairs as the compiled edge relation of **W**'s checked clause, back edges of the `repeat` included. | Test (TC-1062) |
| FR-383-AC-17 | A graph in which **W**'s required temporal clause key, its model relationship key or an attempt's operation anchor key is bound to a key no emitted node carries refuses `missing_declaration`/`missing-name` at the construct that bound it, naming the bound key, with no protocol node and no package; **W** with an `await` in a package that selects no `temporal_profile` refuses `missing_declaration`/`missing-selection` at the `await`, and so does a `compensate`; a unit that selects no protocol profile emits no protocol node and no refusal for the declaration; a requirement cycle never reaches S4 (FR-218-AC-25). | Test (TC-1063) |
| FR-383-AC-18 | A unit that holds a good protocol **W** and a protocol **X** whose join names a missing branch has two results in declaration order: **W** `Emitted` with nodes and clause key byte-identical to those of emitting **W** alone, and **X** `Refused` with `missing_declaration`/`missing-name` at the label, locus the source index and span of **X**'s declaration. The aggregate is `Refused`, with no package and no `package_id`, and the results of both protocols stay readable from it. Emitting **W** alone succeeds with a package. | Test (TC-1063, TC-1297) |
| FR-383-AC-19 | Reading **W**'s package through QSL's I2 reader admits it and returns every protocol node with the key, tag, form, type, declaration, body, dependencies and occurrences emitted; the protocol rebuilt from the read-back nodes equals the protocol rebuilt from the in-process checked clause in keys, set order, ordered lists, binder levels, channel premises, queue bounds and derived causal relation. | Test (TC-1063) |
| FR-383-AC-20 | Over **W**'s package, each mutation of the table in TC-1063 (a set out of ascending node-id order, a repeated set entry, a channel with no queue, a second queue for one channel, a control held by nothing, a `receive` sequenced before its `send`, an argument missing, a layout out of order, a model of the wrong kind) is refused by QSL's I2 reader with the code, cause and locus QSpec FR-464, FR-465 or FR-466 gives it, and QSL's conformance run replays every executable row of the QSpec protocol-node mutation manifest `protocol-node-mutation-manifest.json` (and the `protocol-node-layout-rows.json` rows it defers to) in place: each `admit` row is admitted, each `refuse` row is refused with exactly the row's typed `code`, `cause` and `locus` at the row's `stage` (an earlier unrelated refusal, in particular a wire refusal where the row expects a later semantic one, fails the row), and each `property` row's assertions hold. The executed set is the manifest's `rows` plus every row of `protocol-node-layout-rows.json` (25 `admit`, 90 `refuse` and 4 `property` rows at QSpec main), a `pending` entry is executed through its same-id layout row (or, for `CH14a`, rows `CH14a-1` to `CH14a-8`), and the run reports a nonzero executed count per class; only a `pending` or `later` entry with no executable row counts as not executed, and it is never counted as passing. | Test (TC-1063) |
| FR-383-AC-21 | A protocol whose controls outnumber `i2.nodes`, emitted with the limit raised, reads back under the default limit as `StageFailure::Limit` of kind node count with setting `i2.nodes`, and verifies when the setting is raised; a protocol with a `parallel` inside a `repeat` inside a branch of another `parallel`, three levels, and a `sequence` of 5,000 children each emit with no limit or depth refusal at S4. | Test (TC-1063) |
| FR-383-AC-22 | **W** with `Shipments` joined `predicate(both) [west, east]` over `function both using v(x: Boolean, y: Boolean): Boolean pure { x and y }` emits a `parallel` node whose join Group holds `policy` `predicate` and a `predicate` reference to `both`'s `function` node, which the package holds, and `joined` `east`, `west`; the same unit written `[east, west]` emits the same bytes and the same `package_id`; a second `parallel` joined on `both` references the same node; the read-back of each `parallel` node gives `Predicate{function}` with `both`'s key. Renaming `both` to `either` keeps the `parallel` node's reference equal to the renamed function's node key. | Test |
| FR-383-AC-23 | ADR-027 §7's `Fill` with `role w on M` emits a `role` node whose `model` argument is the text `identity` of the package `M` selects, with an empty `replication` Group and no `model` dependency, and QSL's I2 reader admits it (QSpec FR-465-AC-9); a clause with `role w on M` and `role u on N` emits two role nodes whose texts are the two selections' identities, with different keys, and a role on the whole model and `role v on M::Cell` have different keys. | Test |
| FR-383-AC-24 | Under Option A, a protocol whose predicate-join function refers to that protocol emits the full cyclic reference component with one shared `recursion_group`, `{size, ordinal}` coordinates and `group_reference` edges; the I2 reader admits it. An acyclic protocol in the same package has no `recursion_group` and `recursion` `null`, and its key inputs are unchanged. | Test (TC-1062) |
| FR-383-AC-25 | SCC discovery is iterative and O(V + E), does not merge groups quadratically, computes each completed group's identity once, and uses no global lock on the emit path. There is no fixed recursion depth, group size, node count or group count cap: the only applicable work bound is the caller's shared `s3.work_units` budget, and exhaustion produces the existing typed limit refusal naming `s3.work_units`. A 100,000-node cycle group emits and reads back within the supplied budget without native stack overflow; the budget refuses at zero and one unit short and succeeds at the exact measured work; permuting authored node order leaves group identity unchanged. | Test (TC-1062) |
| FR-383-AC-26 | Each protocol clause's node-level `owner_model` is the exact local model declaration node bound by the checker for its `over` subject, enters its application-node key preimage and exact dependencies, and is read back unchanged. Two clauses over different selected models have distinct owner edges; retargeting one changes its clause key. The edge is derived from the object type `over`'s `Reference` input names (QSpec FR-464), and a clause whose `over` names none has no edge to write (AC-27). | Test (TC-1062, TC-1063) |
| FR-383-AC-27 | A protocol whose `over` input is `Boolean`, `Integer`, `Option<M::T>` or `Sequence<M::T>` is refused once with `unsupported_construct`/`not-yet-implemented` naming `NoOwnerModel` (QSL-1147) and, with the emission gate open, writes no `protocol_clause` node; the same protocol over `M::T` writes the clause. Over the emitted package, a clause without the edge, an edge on another node, an external edge and an edge omitted from `dependencies` each refuse as IR's reader does (`malformed_wire`, `malformed_wire`, `malformed_wire`, `invalid_semantic_graph`), a non-model target refuses `invalid_model_binding`/`malformed-declaration` at the edge (QSpec FR-464-AC-20), and retargeting only the edge refuses `invalid_package`/`stale-node-key` until the key and `package_id` are rebuilt. | Test (TC-1063) |
| FR-383-AC-28 | In AC-18's unit the diagnostics hold exactly one record, **X**'s, and none for **W**; the unit exits as FR-285-AC-2 gives for the items {0, 20}, which is 20, and never 0. Declaring **X** before **W** gives the same two results and the same records. A protocol **Y** whose predicate-join function names **X** is `Refused` `missing_declaration`/`missing-name` at the reference with `refused_dependency` naming **X** (`missing_declaration`/`missing-name`) and `consequence` true, **W** is unchanged, and the aggregate is `Refused` with **X**'s code, cause and locus as the `StageFailure::Refused` primary. | Test (TC-1297) |
| FR-383-AC-29 | Removing **X** from AC-18's unit changes **W**'s result in no byte. Removing **W** leaves **X**'s refusal identity unchanged: the refusal code, typed cause and declaration/reference names are equal, while `source_index`, declaration span and any cause span remain relative to the containing unit and reflect its new declaration order and source layout. An `unsupported_construct`/`not-yet-implemented` refusal (`NoOwnerModel`, AC-27) of one protocol beside a good one gives the same independence and the same `Refused` aggregate. | Test (TC-1297) |
| FR-383-AC-30 | **W**, **Z** and **V** are three renamed copies of one protocol with equal measured work `n`. With an `s3.work_units` budget of exactly `n` and the source order **W**, **Z**, **V**, the results are **W** `Emitted`, **Z** `Incomplete` naming `s3.work_units` and **V** `Incomplete` naming `s3.work_units`; the aggregate is `Incomplete` (`StageFailure::Limit`) and exits 22. With the source order **Z**, **W**, **V** the results are **Z** `Emitted`, then **W** and **V** `Incomplete`. With a budget of `3n` all three are `Emitted` in either order. | Test (TC-1297) |
| FR-383-AC-31 | A refusal at S1 (a syntax error the parser confines to **X**'s declaration), at S2, at S3 and at S4 (the `NoOwnerModel` refusal) each give **X** a `Refused` result with that stage's code and cause and leave **W**'s result byte-identical to **W** alone; a syntax error in the unit header gives a `Refused` unit with no `ProtocolResult`. FR-503 owns the stage obligations. | Test (TC-1297, TC-1298) |
| FR-383-AC-32 | For every slot of **W**, of TC-1070 fixture E and of protocol L (fixture L), the number of `protocol_locations` rows under the slot's site name for an application equals the number of that application's visits in the slot's walk. | Test (TC-1063) |
| FR-383-AC-33 | No `protocol_locations` row under a slot's site name names an application that slot's walk does not visit. | Test (TC-1063) |
| FR-383-AC-34 | In fixture E, the retry origin of `Refund` holds one `earlier.tally` application node with exactly two rows, and the retry origin of `Partial` one `partialEarlier.tally` node with exactly two rows of its own. | Test (TC-1063) |
| FR-383-AC-35 | Each row, read back through QSL's I2 reader, has a `Location` equal to the `Location` of the term at the position it numbers in the checked clause held in memory. | Test (TC-1063) |
| FR-383-AC-36 | The two retry-origin `earlier.tally` rows of `Refund` carry different `Location` values, the first that of the term in `earlier.tally < 3` and the second that in `earlier.tally + 1`. | Test (TC-1063) |
| FR-383-AC-37 | A unit with no protocol slot expression writes `protocol_locations` as an empty array. | Test (TC-1063) |
| FR-383-AC-38 | A package with no checked-term data keeps `checked_term_data` absent. | Test (TC-1063) |
| FR-383-AC-39 | In fixtures E, L and E-C every slot that holds an application has its rows under a type-declaration origin whose `name` is that slot's site name. | Test (TC-1063) |
| FR-383-AC-40 | In fixtures E, L and E-C no two slots have the same site name. | Test (TC-1063) |
| FR-383-AC-41 | In fixtures E, L and E-C no row has a `protocol_attempt` origin. | Test (TC-1063) |
| FR-383-AC-42 | Inspection finds no emitter path that computes a `protocol_locations` row's location from a source region, a node order or an operand position, or fills one by default. | Inspection |
| FR-383-AC-43 | Removing the retry-origin `earlier.tally` row of `Refund` keyed to occurrence 1 from fixture E's emitted package (leaving the occurrence-0 row) leaves a package QSL's I2 reader admits. | Test (TC-1063) |
| FR-383-AC-44 | Changing the `term_ordinal` of the retry-origin `earlier.tally` row of `Refund` keyed to occurrence 1 from 0 to 1 (a group with no ordinal-zero row) is refused by QSL's I2 reader with `invalid_package`/`invalid-value` at `/checked_term_data/protocol_locations`. | Test (TC-1063) |
| FR-383-AC-45 | Rekeying the occurrence-1 retry-origin `earlier.tally` row to occurrence 0 with `term_ordinal` 2 (group ordinals 0 and 2) is refused by QSL's I2 reader with `invalid_package`/`invalid-value` at `/checked_term_data/protocol_locations`. | Test (TC-1063) |
| FR-383-AC-46 | Duplicating a retry-origin `earlier.tally` row (same key, written next to the original) is refused by QSL's I2 reader with `invalid_package`/`conflicting-definition` at `/checked_term_data/protocol_locations/<index of the duplicate>` (QSpec vector PL-R1-duplicate-term, locus `/checked_term_data/protocol_locations/1` for a duplicate at index 1; FR-322-AC-291). | Test (TC-1063) |
| FR-383-AC-47 | For fixture E-C, every table slot's row count for an application equals its visits in the slot walk (each table slot has its own site name and shares no application with another slot), including `m.count` twice in `Composed.if`, `m.note` twice in `Composed.option`, `m.items` twice in `Composed.coll` and once in `Composed.let`. | Test (TC-1063) |
| FR-383-AC-48 | For fixture E-C, each row's `Location` equals the checked clause's `Location` of the term at the position it numbers. | Test (TC-1063) |
| FR-383-AC-49 | For fixture E-C, the arms of `Composed.if` and `Composed.case` that the input never selects have their rows, with the checked clause's `Location` values. | Test (TC-1063) |
| FR-383-AC-50 | In fixture E the two retry-origin `earlier.tally` rows of `Refund` are keyed to the node's `expression` occurrences 0 and 1, in visit order, and each has `term_ordinal` 0. | Test (TC-1063) |
| FR-383-AC-51 | In fixture E, `target >= 0` is one graph node that `Refund`'s retry relation and trigger guard both hold. | Test (TC-1063) |
| FR-383-AC-52 | In fixture E `target >= 0` has one row of the trigger-guard origin of `Refund`, keyed to its `expression` occurrence 0, and one row of the retry origin, keyed to occurrence 1; each group holds exactly one row. | Test (TC-1063) |
| FR-383-AC-53 | In fixture E both rows of `target >= 0` have `term_ordinal` 0, and the retry row is not keyed to the trigger-guard row's occurrence. | Test (TC-1063) |
| FR-383-AC-54 | In fixture E each of the two rows of `target >= 0` carries the `Location` of the term in its own slot. | Test (TC-1063) |
| FR-383-AC-55 | In fixture E-C, `order >= 0` is one graph node that the constraints of `Tried` and of `Picked` both hold. | Test (TC-1063) |
| FR-383-AC-56 | In fixture E-C `order >= 0` has one row of the `Tried` constraint's origin (`protocol Composed run Main Tried body`), keyed to its `expression` occurrence 0, and one of the `Picked` constraint's origin (`protocol Composed run Main Picked body`), keyed to occurrence 1 (`Tried` precedes `Picked` in document order); each group holds exactly one row. | Test (TC-1063) |
| FR-383-AC-57 | In fixture E-C both rows of `order >= 0` have `term_ordinal` 0. | Test (TC-1063) |
| FR-383-AC-58 | For every application of **W** and fixtures E, L and E-C, the (occurrence ordinal, `term_ordinal`) pairs of a slot's rows of that application, taken along the slot walk's visits, strictly increase. | Test (TC-1063) |
| FR-383-AC-59 | In fixture E-C no `protocol_locations` row names a `Local` read (`x`, `b`, `c`), a literal, a `let`, a conditional or a case node. | Test (TC-1063) |
| FR-383-AC-60 | In fixture E-C each arm of `Composed.case` holds exactly one row (`x >= 1` and `not b`). | Test (TC-1063) |
| FR-383-AC-61 | In fixture E the checked package records exactly two `expression` occurrences of the `earlier.tally` application of `Refund`'s retry relation, ordinal 0 at its term in `earlier.tally < 3` and ordinal 1 at its term in `earlier.tally + 1`, and exactly two of `target >= 0`, ordinal 0 at its term in the trigger guard and ordinal 1 at its term in the retry relation; and in fixture E-C's `Tried` and `Picked` controls exactly two of `order >= 0`, ordinal 0 at the `Tried` constraint and ordinal 1 at the `Picked` constraint (declaration order, although `Picked` sorts first by origin name); so each position of an application in a slot's walk has an occurrence of its own. | Test (TC-1063) |
| FR-383-AC-62 | Over a unit that imports a model package `M` in which the component `Sys` owns two Ports, `pump_out` (direction out) and `tank_in` (direction in), and holds `role Outlet on M::pump_out;`, `role Inlet on M::tank_in;`, `role Operator on M::Sys;` and a role on the whole model `M` emits `Outlet` and `Inlet` with a `model_member` Group holding the `package` identity of `M` and the declaration identity of their own Port, and a `model` reference to the node of `Sys`, so the two roles have different node ids; `Operator` and the whole-model role emit no third argument and the same bytes they emitted before the argument existed. After the I2 read, the typed role read back holds the same `package` and `node` as the checked role's selected Port key (`CheckedRole.model_member`). | Test (TC-1062) |
| FR-383-AC-63 | In fixture L (TC-1070) the rows of `Decide`'s case guards are under `protocol Pipeline run Main Decide case yes guard` and `... case no guard`, the rows of `Decide.visible[0]` under `... decision 2641..2650`, and no row is under any other `decision` origin; in the variant of fixture L whose two `related` ends of `Got` are one identity-function call, the ends' rows are under `protocol Pipeline run Main Got related 0 from` and `... related 0 to`. | Test (TC-1063) |
| FR-383-AC-64 | In the two-record variant of fixture L, whose `Got` holds `related by Owns (same(view), view)` and `related by Owns (same(view), same(view))` with `same` the pure reference-valued identity function, compiled once in each source order of the two clauses: the shared `same(view)` call of the two `from` ends has its rows under `protocol Pipeline run Main Got related 0 from` and `... related 1 from`, the `to` end that is a call has its rows under `... related <k> to` where `k` is the emitted position of its record, and the record whose `to` end is the reference `view` has no `to` row; in both orders the record holding each `to` end has the same emitted position and each origin holds rows of one end only; the checked package holds no `Location` and the `protocol_locations` array holds no row under a `related <start> <end>` origin; and a third compile that writes the second clause twice gives the same origins and the same number of rows under each as the compile without the repeat. | Test (TC-1063) |
## Dependencies

- FR-383-AC-15 rests on QSpec FR-322 "Operand index segments" (QSL-1695): inside a node every operand is an operand slot `{"operand": k}` into the document's required `semantic_graph.operand_rows`, numbered by first occurrence in the node's serialization, and only ids outside nodes keep `{"node": i}`. A node's bytes therefore hold no graph index of its own graph, so adding a node ahead of an operand a protocol reaches changes no protocol node's bytes, only `operand_rows`; node equality across documents is equality of resolved structural keys (each operand slot replaced by the key of the node it names). The path-stability gate (STD-653) is closed by that wire form.
- Contract IR IR-1310: the reader sees the declaration kinds of the selected model package, which a Port role's `model_member` check needs (QSpec FR-465).
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
