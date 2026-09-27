---
id: TC-462
title: "S4 emits state clause, operation anchor and frame nodes with their bodies"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: verifies
---
# TC-462: S4 emits state clause, operation anchor and frame nodes with their bodies

## Description

Verify the node set, bodies, types, dependencies and occurrences S4 emits for
state clauses, and the totality of the clause-kind mapping.

Scope: FR-105-AC-1, FR-105-AC-2, FR-105-AC-5.

## Test Procedure

1. Spine-compile FR-108's unit without `sameIdentity` against TC-458's
   fixture package, and decode the emitted `quire.checked-package/v2` graph.
2. For each `state` node, compare its `semantic_type`, `body`,
   `dependencies` and occurrences with FR-105's Outputs table.
3. Walk `VersionUnchanged`'s, `NoCycle`'s and `ParentOrder`'s condition
   terms.
4. Run `CheckedClauseKind`'s mapping forward over all seven variants and
   backward over every triple the forward pass gives and over
   (`state`, `frame`); run `cargo mutants` over the mapping functions.

Tag the tests `#[trace("TC-462", "FR-105-AC-n")]`.

## Expected Results

- Step 1: exactly three `state`/`state_clause` nodes, whose kind members are
  `invariant`, `invariant` and `postcondition`; one `state`/`operation_anchor`
  with context the `ConfigVersion` `object_type` node and operation the text
  literal `"attemptUpdate"`; one `state`/`frame`. No `model`/`field_declaration`,
  `model`/`operation_declaration`, `state`/`snapshot` or `state`/`transition`
  node.
- Step 2: each matches the table. The frame's `modifies` is exactly
  (`ConfigVersion` node, `versionNumber`), `creates` and `deletes` are empty,
  and the `ConfigVersion` node is in the frame's `dependencies`.
  `VersionUnchanged`'s anchor argument references the `operation_anchor`
  node; each invariant's references the `ConfigVersion` node. Each clause
  application's operation is `quire.op.state.clause`.
- Step 3: `VersionUnchanged`'s condition holds a `quire.op.state.pre`
  application over the field read of `versionNumber`. `NoCycle`'s holds a
  `quire.op.model.reaches_field` application whose member is
  `{kind: "field", declaration: <ConfigVersion node>, name: "parent"}`, and
  `ParentOrder`'s `self.parent` read carries a member of the same shape.
- Step 4: each variant gives one triple, no two share one, each triple
  decodes back to its variant, (`state`, `frame`) decodes to none, and
  `StateTransition` gives (`state`, `transition`); no mutant survives.

## Status

Implemented (QSL-279): `s4_emits_exactly_the_fr_105_state_nodes` covers step
1 and most of step 2 (node set, kinds, anchor operation, frame
`modifies`/`creates`/`deletes`, the `ConfigVersion` node found independently
through `resolve_declaration`, each invariant's anchor argument, the
forbidden forms); `s4_emitted_frame_node_lists_configversion_in_dependencies`
covers the frame's own emitted `dependencies`, decoding the wire directly.
`triple_mapping_is_total_and_injective_over_all_seven_variants` covers step
4. Step 3 (the `VersionUnchanged`/`NoCycle`/`ParentOrder` condition-term
walk) and the remainder of step 2 (each non-frame node's own `dependencies`
and occurrences, the `state_clause` `semantic_type`, `VersionUnchanged`'s
anchor argument) are not yet covered; tracked by QSL-308.
