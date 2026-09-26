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
nothing, one `state`/`state_clause` node per clause, one `state`/`operation_anchor`
node per operation a `pre` or `post` clause names, one `state`/`frame` node per
such operation, and the `model`/`field_declaration`, `model`/`operation_declaration`
and `model`/`object_type` nodes they reference (ADR-012 §15.3, §15.4). The
package's `package_id` covers these nodes, and QSL's I2 read admits it.

Today QSL emits no `state` node. `NodeTag::State` appears only in the tag map
(`qsl-package/src/emit.rs:254`) and the declaration-absent rule
(`emit.rs:388-396`); `CheckedClauseKind::StateTransition` is identity
vocabulary with no emitter (`qsl-semantics/src/check/identity.rs:150-195`);
and the only `model` forms QSL emits are `object_type` and
`systems_interface`. The pinned IR vocabulary already decodes all five
`StateForm`s (`quire-contract-model` 48ab5dc,
`checked_package/v2/vocabulary.rs:238-246`).

## Inputs

- A `CheckedGraph` with checked state clauses and the model correspondence
  (ADR-013 O-04, FR-088).

## Outputs

The nodes below, each keyed by FR-092/FR-093's preimage rules (the
application-node preimage when its body holds an application), with its
`dependencies` listing every node its body references except a
`dependency_reference` (FR-093-AC-12), and its occurrences in the package
source map (FR-095).

| Node | `semantic_type` | `body` | Occurrence |
| --- | --- | --- | --- |
| `state`/`state_clause` | the `Boolean` scalar type node | `aggregate[ binding("parameters", aggregate[reference(self), reference(result)?, reference(parameter)...]), binding("anchor", reference(anchor)), binding(kind, condition) ]` | `claim`, 0 |
| `state`/`operation_anchor` | the context's `model`/`object_type` node | `aggregate[ binding("context", reference(object_type)), binding("operation", reference(operation_declaration)), binding("frame", reference(frame)) ]` | `anchor`, 0 |
| `state`/`frame` | the context's `model`/`object_type` node | QSpec FR-340's `{term: "frame", modifies, creates, deletes}` | `generated`, 0 |
| `model`/`field_declaration` | the field's value type node | keyed by FR-094's `ModelOwner` rule, as `object_type` is | as FR-094 |
| `model`/`operation_declaration` | the result type node, or `Boolean` when the operation declares none | keyed by FR-094's `ModelOwner` rule | as FR-094 |

- `kind` is `invariant`, `precondition` or `postcondition`. `anchor` is the
  context's `model`/`object_type` node for an invariant and the operation's
  `state`/`operation_anchor` node otherwise.
- `self`, `result` and each operation parameter are `value`/`parameter`
  nodes, as a function's parameters are (FR-092). `result` is present only in
  a postcondition of an operation with a result.
- `condition` is the checked Boolean body lowered by FR-093 exactly as a
  `pure_function` body is lowered. A field read of `self` lowers as the
  `deref` application FR-093 gives `deref(self).f`; `reaches` lowers as a
  `quire.op.model.reaches` application whose `member` names the edge's
  `model`/`field_declaration` node.

## Behavior

- S4 SHALL emit the nodes above for every checked state clause, or none of
  them and a refusal (ADR-012 §2 "`package` is all-or-nothing"). The spine
  already refuses a compile with any omitted node
  (`qsl-replay/src/spine.rs:866-870`), and that rule SHALL hold for state
  nodes too.
- `CheckedClauseKind` SHALL gain `Invariant`, `Precondition` and
  `Postcondition`. `node_tag_and_semantic_form` SHALL map each to (`state`,
  `state_clause`), and the body's kind binding SHALL tell them apart. Both
  directions of the mapping (checked kind → (pair, kind binding) and back)
  SHALL be total over the enum with no `_` arm (FR-088-AC-4, ADR-013 O-10).
- A frame's `modifies`, `creates` and `deletes` SHALL each list their node
  keys in ascending digest order, and each SHALL also be a `dependency` of the
  frame node (QSpec FR-340).
- S4 SHALL emit one `operation_anchor` and one `frame` per operation, however
  many clauses name it, and SHALL emit no anchor or frame for an operation no
  clause names.
- S4 SHALL emit no `state`/`snapshot` and no `state`/`transition` node for a
  state clause (ADR-012 §15.4).
- The emitted bytes SHALL be independent of any snapshot or invocation: the
  same unit and packages give the same `package_id` whatever observations a
  later run supplies.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-105-AC-1 | The ConfigVersion unit of FR-104-AC-1 emits exactly three `state_clause` nodes (kinds `invariant`, `invariant`, `postcondition`), one `operation_anchor` and one `frame` (for `attemptUpdate`), and `field_declaration` nodes for `versionNumber` and `parent`; the frame's `modifies` is exactly the `versionNumber` field node, and its `creates` and `deletes` are empty. | Test (TC-462) |
| FR-105-AC-2 | Each emitted node's body, `semantic_type`, `dependencies` and occurrence match the Outputs table; `VersionUnchanged`'s condition holds a `quire.op.state.pre` application over the `deref` read of `versionNumber`; `NoCycle`'s holds a `quire.op.model.reaches` application whose `member` names the `parent` field node. | Test (TC-462) |
| FR-105-AC-3 | The emitted package passes QSL's I2 read (`qsl-package` `checked_v2` reader), including its frame step (FR-340), and its recomputed `package_id` equals the emitted one. | Test (TC-463) |
| FR-105-AC-4 | Compiling the unit twice gives identical bytes. Renaming `ParentOrder` changes no node id; changing its `<` to `<=` changes its `state_clause` node id and the `package_id`; adding a second `post` clause on `attemptUpdate` adds one `state_clause` node and no second anchor or frame. | Test (TC-463) |
| FR-105-AC-5 | `CheckedClauseKind`'s wire mapping is total and injective in both directions over all seven variants, and each of `Invariant`, `Precondition` and `Postcondition` round-trips through (pair, kind binding); a mutant that swaps two kind spellings fails the test. | Test (TC-462) |
| FR-105-AC-6 | When the emitter cannot emit one of a clause's nodes (fault injected at the `frame` node), the compile refuses and emits no `state` node and no package bytes. | Test (TC-463) |

## Dependencies

- FR-088 (clause kind, frame identity), FR-092 to FR-095 (keys, lowering,
  model-owned nodes, source map), FR-104 (checked clauses).
- QSpec FR-322 and FR-340 (the v2 wire). Two QSpec changes are prerequisites
  for AC-3's read of these bodies by any reader other than QSL's and for the
  IR reader: a body rule for `state_clause` and `operation_anchor`, which
  this FR proposes with existing term kinds only; and a `field_declaration`
  member for `quire.op.model.reaches`, whose catalog entry today admits
  `relationship_end` only (quire-verification-contracts
  `checked-operation-catalog-v1.json` at 61f4a44). v2 is prerelease and
  QSpec revises it in place (ADR-012 §12.1). QSL's own I2 read admits the
  bodies once its reader carries the rule.
- IR `lower` returns no form for any `state` node at 48ab5dc, so these nodes
  reach no backend. That is IR's own work (ADR-012 §8, §15.7).
