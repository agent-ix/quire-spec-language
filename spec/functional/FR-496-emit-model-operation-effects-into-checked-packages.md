---
id: FR-496
title: "Emit model operation effects into checked packages"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-154
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-340
    type: depends_on
---
# FR-496: Emit model operation effects into checked packages

## Description

A model operation member has no semantic graph node, and a `state`/`frame` node
(QSpec FR-340) exists only where a clause, protocol attempt, transition or
refinement step binds that operation. A typed reader therefore cannot read the
declared effect of an operation from the graph. [FR-103](./FR-103-admit-model-operations-and-frames-on-the-spine.md)
owns admitting each operation's frame into its `OperationMemberRecord.effect`
and [FR-154](./FR-154-scope-a-modifies-entry-to-the-receiver.md) owns the scoped
entry forms; this requirement owns carrying those admitted effects through QSL
emission.

The emitter SHALL write the optional top-level `operation_effects` member
defined by QSpec [FR-322](ix://agent-ix/quire-specification/FR-322)
(FR-322-AC-303, "Operation-effect carrier"). QSpec FR-322 owns the wire shape,
the closed row schema and the reader's joins; QSL owns emission. The member is
additional checked-package data and does not change semantic graph nodes or
their identities: FR-094 builds the node of each declaration a row names
whether or not the member is carried.

## Inputs

- The operation members of every selected domain package, with their admitted
  `OperationMemberRecord.effect` (FR-103) and receiver and parameter scopes
  (FR-154).
- The declaring `object_type`, `process` or `state_machine` node of each
  operation and its local `NodeRef`.

## Outputs

- A checked package that carries `operation_effects` with one row per
  operation member, or that omits the member.

## Behavior

- The emitter SHALL emit one row `{document, declaring, operation, effect,
  provenance}` for every operation member declared by a `model` node of form
  `object_type`, `process` or `state_machine` in the selected documents.
  `document` is the selected document identity, `declaring` the local
  `NodeRef` of the declaring node, and `operation` the original member name the
  declaring node itself declares. An operation only inherited by a subtype has
  no row under the subtype; the same member name under another declaring node
  or document is another row.
- `effect` SHALL carry all four members `modifies`, `creates`, `deletes` and
  `deletes_scoped`, each possibly empty, exactly as the operation's admitted
  frame lifts under QSpec FR-013-AC-13 and FR-340: the same entry shapes,
  scopes and canonical order, minus `term`. No effect kind is dropped,
  collapsed or inferred: a field `modifies` entry keeps its `receiver` or
  `every-object` scope, a relationship `modifies` entry is kept, `creates` and
  type-wide `deletes` are kept, and the `deletes_scoped` receiver and
  parameter entries (FR-154's `deletes_receiver` and `deletes_arguments`) are
  kept. An operation whose frame has no entries has a row of four empty
  arrays, never an absent row.
- `provenance.origin` SHALL be `model_document_operation_frame` and
  `provenance.member` the IR node identity of the operation member in
  `document`, `<declaring owner node>/<operation>`. Both are taken from the
  checked declaration, never inferred from a path, a display name, the order
  of members or the position of a frame node.
- Rows SHALL be keyed and sorted by `(document, declaring, operation)`, in the
  order QSpec FR-322 and FR-340 state: `document` as UTF-8 bytes, then
  `declaring` by structural `NodeRef`, then `operation` as UTF-8 bytes. The
  entries inside each `effect` member SHALL be in FR-340's canonical order.
  Emission order is independent of source declaration order.
- A `persistence_interface` declares operations but has no FR-013 frame, so
  the emitter SHALL emit no row for it. A relationship or `systems_interface`
  node SHALL NOT be a declaring node of a row.
- When the producer cannot supply the admitted effect of any operation, the
  emitter SHALL omit the whole `operation_effects` member, which means the
  effects are unavailable. It SHALL NOT emit a partial member, an empty array
  or a placeholder effect, because a present member must cover every
  operation (QSpec FR-322-AC-303).
- QSL emission SHALL neither perform the IR reader's typed join nor claim
  reader admission; QSpec FR-322 and TC-977 own the reader refusals.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-496-AC-1 | A package whose object type declares operations that between them modify a field under receiver scope, modify a field under every-object scope, modify a relationship, create an object type, delete an object type type-wide, and delete the receiver and a reference parameter, emits `operation_effects` with exactly one row per operation; each row's `effect` equals the operation's admitted frame lift member by member (`modifies`, `creates`, `deletes`, `deletes_scoped`) with no entry dropped or merged, and its `provenance` is `{origin: model_document_operation_frame, member: <declaring owner node>/<operation>}`. This matches QSpec vectors OEC-01 to OEC-04 and OEC-07. | Test (TC-1286) |
| FR-496-AC-2 | An operation with no frame entries emits a row whose four `effect` members are empty arrays; two operations of one object emit two rows; the same operation name `settle` under two objects of one document and under an object of a second document emits three distinct rows with their own effects. This matches QSpec vectors OEC-05 to OEC-07. | Test (TC-1286) |
| FR-496-AC-3 | Rows are sorted by `(document, declaring, operation)` and the entries inside each `effect` member are in FR-340 canonical order, and permuting source declaration order, member order or frame entry order of the same package emits an equal `operation_effects` member. | Test (TC-1286) |
| FR-496-AC-4 | A `process` and a `state_machine` declaring node each emit rows for their operations; a `persistence_interface`, a relationship and a `systems_interface` node emit none, and a subtype that only inherits an operation emits no row for it. | Test (TC-1286) |
| FR-496-AC-5 | When the effect of any operation is unavailable from the producer, the package omits `operation_effects` entirely and no other member of the package differs from the package emitted with the member; the member is never emitted empty or partial. A package that carries the member has the same semantic graph nodes and node identities as one that omits it. | Test (TC-1286) |

## Dependencies

- **Upstream:** FR-103 admits each operation's frame into
  `OperationMemberRecord.effect`; FR-154 defines the receiver and parameter
  scopes of `modifies` and `deletes` entries. QSpec FR-322 (FR-322-AC-303),
  FR-340 and FR-013-AC-13 define the carrier, the canonical order and the
  frame lift.
- **Downstream:** the IR strict reader admits or refuses the member and
  exposes an operation's effect by key; it owns no mirror type (IR-1226
  family). Consumers SHALL use that read and SHALL NOT reconstruct an effect
  from frame nodes or names.
