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
state clauses, and the totality of the clause-kind wire mapping.

Scope: FR-105-AC-1, FR-105-AC-2, FR-105-AC-5.

## Test Procedure

1. Spine-compile FR-108's unit without `sameIdentity` against TC-458's
   fixture package, and decode the emitted `quire.checked-package/v2` graph.
2. For each `state` node and each `model`/`field_declaration` and
   `model`/`operation_declaration` node, compare its `semantic_type`, `body`,
   `dependencies` and occurrences with FR-105's Outputs table.
3. Walk `VersionUnchanged`'s and `NoCycle`'s condition terms.
4. Run `CheckedClauseKind`'s mapping in both directions over all seven
   variants, and run `cargo mutants` over the mapping functions.

Tag the tests `#[trace("TC-462", "FR-105-AC-n")]`.

## Expected Results

- Step 1: exactly three `state`/`state_clause` nodes, whose kind bindings are
  `invariant`, `invariant` and `postcondition`; one `state`/`operation_anchor`
  and one `state`/`frame`; `model`/`field_declaration` nodes for
  `versionNumber` and `parent`; one `model`/`operation_declaration` for
  `attemptUpdate`. No `state`/`snapshot` or `state`/`transition` node.
- Step 2: each matches the table. The frame body's `modifies` is exactly the
  `versionNumber` field node, `creates` and `deletes` are empty, and each
  entry is also in the frame's `dependencies`, in ascending digest order.
  `VersionUnchanged`'s anchor binding references the `operation_anchor` node;
  each invariant's references the `ConfigVersion` `object_type` node.
- Step 3: `VersionUnchanged`'s condition holds a `quire.op.state.pre`
  application over the `deref` read of `versionNumber`; `NoCycle`'s holds a
  `quire.op.model.reaches` application whose `member` names the `parent`
  field node.
- Step 4: each variant maps to one (pair, kind binding) and back to itself;
  no two variants share one; no mutant survives.

## Status

Planned (QSL-273).
