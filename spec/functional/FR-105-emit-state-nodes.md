---
id: FR-105
title: "Emit state clause, operation anchor and frame nodes in checked-package/v2"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-340
    type: depends_on
---
# FR-105: Emit state clause, operation anchor and frame nodes in checked-package/v2

## Description

When a checked graph holds state clauses (FR-104), S4 SHALL emit, all or
nothing, the `state`/`state_clause`, `state`/`operation_anchor` and
`state`/`frame` nodes below (ADR-012 §15.3, §15.4). S4 SHALL name every model
member by its declaring object type's `model`/`object_type` node and its
member name, as ADR-013 O-06, FR-094 and QSpec FR-322 "Model-owned members"
already do, and SHALL emit no `model`/`field_declaration` or
`model`/`operation_declaration` node. The package's `package_id` covers these
nodes.

Today QSL emits no `state` node. `NodeTag::State` appears only in the tag map
(`qsl-package/src/emit.rs:254`) and the declaration-absent rule
(`emit.rs:388-396`); `CheckedClauseKind::StateTransition` is identity
vocabulary with no emitter (`qsl-semantics/src/check/identity.rs:150-195`).
The pinned IR vocabulary already decodes all five `StateForm`s
(`quire-contract-model` 48ab5dc, `checked_package/v2/vocabulary.rs:238-246`).

The wire spellings of these bodies, the (object type, member name) frame
entry, `quire.op.model.reaches_field` and `quire.op.state.clause` are QSpec
changes that STD-111 carries. v2 is prerelease and QSpec revises it in place
(ADR-012 §12.1).

## Inputs

- A `CheckedGraph` with checked state clauses and the model correspondence
  (ADR-013 O-04, FR-088).

## Outputs

The nodes below, each keyed by FR-092/FR-093's preimage rules (the
application-node preimage when its body holds an application), with its
`dependencies` listing every node its body references except a
`dependency_reference` (FR-093-AC-12), and its occurrences in the package
source map (FR-095).

| Node | `semantic_type` | `body` | Occurrences |
| --- | --- | --- | --- |
| `state`/`state_clause` | the `Boolean` scalar type node | an application of `quire.op.state.clause` whose `member` is `{kind: "state_clause", clause: <kind>}` and whose arguments are, in order: `aggregate` of the parameter references (`self`, then `result` when present, then the operation's parameters in declared order), a `reference` to the anchor, and the condition term | one `claim` occurrence per declaration that checks to this node, ordinals 0, 1, ... in source order |
| `state`/`operation_anchor` | the declaring `model`/`object_type` node | `aggregate[ binding("context", reference(declaring object_type)), binding("operation", literal(text, "<operation name>")), binding("frame", reference(frame)) ]` | one `anchor` occurrence per clause that names the operation, ordinals in source order |
| `state`/`frame` | the declaring `model`/`object_type` node | QSpec FR-340's frame term, with each `modifies` entry an (object type node, field name) pair as STD-111 spells it, and each `creates`/`deletes` entry an `object_type` node | `generated`, 0 |

- `<kind>` is `invariant`, `precondition` or `postcondition`. The anchor is
  the context's `model`/`object_type` node for an invariant, and the
  operation's `state`/`operation_anchor` node otherwise.
- `self`, `result` and each operation parameter are `value`/`parameter`
  nodes, as a function's parameters are (FR-092). `result` is present only in
  a postcondition of an operation with a result.
- The condition is the checked Boolean body lowered by FR-093 exactly as a
  `pure_function` body is lowered. A field read lowers as the `deref` and
  member application FR-093 gives `deref(self).f`, whose member is
  `{kind: "field", declaration: <object_type node>, name: "<field>"}`.
  `pre(e)` lowers as `quire.op.state.pre`. `reaches(a, b, edge)` lowers as a
  `quire.op.model.reaches_field` application whose member is
  `{kind: "field", declaration: <object_type node>, name: "<edge>"}`.
- The text literal's `type` is the `text` scalar type node.

## Behavior

- S4 SHALL emit the nodes above for every checked state clause, or none of
  them and a refusal (ADR-012 §2 "`package` is all-or-nothing"). The spine
  already refuses a compile with any omitted node
  (`qsl-replay/src/spine.rs:866-870`), and that rule SHALL hold for state
  nodes too.
- S4 SHALL emit one `state_clause` node per distinct clause node id. Two
  declarations that check to one node id (FR-104-AC-6) SHALL give one node
  with two `claim` occurrences, never two nodes with one id.
- S4 SHALL emit one `operation_anchor` and one `frame` per (declaring object
  type, operation name), however many clauses name the operation and from
  whichever subtype they name it, and none for an operation no clause names.
  An operation inherited from `M::Base` and named as `M::Sub::op` anchors at
  `M::Base`.
- The checker SHALL add `Invariant`, `Precondition` and `Postcondition` to
  `CheckedClauseKind` (FR-088, as amended), each spelled with the node pair
  (`state`, `state_clause`), the clause operation `quire.op.state.clause` and
  its own kind member, so the three spellings differ only in the kind member.
- The checker SHALL spell `StateTransition` with the node pair
  (`state`, `transition`).
- The decoder of `CheckedClauseKind` SHALL map (`state`, `frame`) to no clause
  kind: a frame node's body is FR-340's frame term, which holds no clause
  application, so a frame under an `operation_anchor` never decodes as
  `StateTransition` or any other clause kind.
- S4 SHALL list a frame's `modifies`, `creates` and `deletes` entries in
  ascending order of (object type node digest, field name), and SHALL list
  each entry's object type node in the frame's `dependencies` (QSpec FR-340's
  dependency rule).
- S4 SHALL emit no `state`/`snapshot` and no `state`/`transition` node for a
  state clause (ADR-012 §15.4).
- S4 SHALL emit bytes that are independent of any snapshot or invocation: the
  same unit and packages give the same `package_id` whatever observations a
  later run supplies.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-105-AC-1 | The ConfigVersion unit of FR-104-AC-1 emits exactly three `state_clause` nodes (kinds `invariant`, `invariant`, `postcondition`), one `operation_anchor` (context `ConfigVersion`, operation `"attemptUpdate"`) and one `frame`, whose `modifies` is exactly (`ConfigVersion` node, `versionNumber`) and whose `creates` and `deletes` are empty. The package holds no `model`/`field_declaration`, `model`/`operation_declaration`, `state`/`snapshot` or `state`/`transition` node. | Test (TC-462); emission pending STD-111 |
| FR-105-AC-2 | Each emitted node's body, `semantic_type`, `dependencies` and occurrences match the Outputs table; `VersionUnchanged`'s condition holds a `quire.op.state.pre` application over the field read of `versionNumber`; `NoCycle`'s holds a `quire.op.model.reaches_field` application whose member is `{kind: "field", declaration: <ConfigVersion node>, name: "parent"}`, the same member shape `ParentOrder`'s `self.parent` read carries. | Test (TC-462); emission pending STD-111 |
| FR-105-AC-3 | The emitted package passes QSL's I2 read (`qsl-package` `checked_v2` reader), including its frame step, and its recomputed `package_id` equals the emitted one. | Test (TC-463); pending STD-111 |
| FR-105-AC-4 | Compiling the unit twice gives identical bytes. Renaming `ParentOrder` changes no node id; changing its `<` to `<=` changes its `state_clause` node id and the `package_id`; adding `ParentOrder2` with `ParentOrder`'s body adds no node and a second `claim` occurrence (ordinal 1) to `ParentOrder`'s node; adding a second `post` clause on `attemptUpdate` adds one `state_clause` node and no second anchor or frame. Over a package where `Sub` specializes `ConfigVersion`, `pre A ... on Config::ConfigVersion::attemptUpdate` and `pre B ... on Config::Sub::attemptUpdate` share one anchor whose context is `ConfigVersion`. | Test (TC-463) |
| FR-105-AC-5 | `CheckedClauseKind`'s mapping is total over all seven variants: forward, each variant gives one (node pair, clause operation, kind member) triple and no two variants give the same triple; backward, each triple decodes to its variant; (`state`, `frame`) decodes to no variant; `StateTransition` gives (`state`, `transition`). A mutant that swaps two kind members, or deletes an arm, fails the test. | Test (TC-462) |
| FR-105-AC-6 | When the emitter cannot emit one of a clause's nodes (fault injected at the `frame` node), the compile refuses and emits no `state` node and no package bytes. | Test (TC-463) |

## Dependencies

- FR-088 (clause kind, frame identity; amended by this PR), FR-092 to FR-095
  (keys, lowering, model-owned members, source map), FR-104 (checked
  clauses). FR-094 needs no amendment: no field or operation member gets a
  node of its own.
- STD-111 (QSpec): the `state_clause`, `operation_anchor` and `frame` body
  rules, the (object type, member name) frame entry,
  `quire.op.model.reaches_field` with the `field` member kind and its
  reference-edge constraint, and `quire.op.state.clause`. Only this FR's
  emission criteria and FR-108's reading of the emitted package wait on it;
  FR-102 to FR-104 and FR-106 to FR-109 read the in-process `CheckedPackage`.
- IR `lower` returns no form for any `state` node at 48ab5dc, so these nodes
  reach no backend. That is IR's own work (ADR-012 §8, §15.7).

## Status

Specified under QSL-273. AC-1 to AC-3 are pending STD-111 (QSpec wire).
AC-4 to AC-6 do not wait on it.
