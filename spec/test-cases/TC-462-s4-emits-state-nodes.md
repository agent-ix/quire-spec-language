---
id: TC-462
title: "S4 emits state clause, operation anchor and frame nodes with their bodies"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-104-AC-15
    type: verifies
---
# TC-462: S4 emits state clause, operation anchor and frame nodes with their bodies

## Description

Verify the node set, bodies, types, dependencies and occurrences S4 emits for
state clauses, and the totality of the clause-kind mapping.

Scope: FR-104-AC-15; FR-105-AC-1, FR-105-AC-2, FR-105-AC-5, FR-105-AC-8 to FR-105-AC-12.

## Test Procedure

1. Spine-compile FR-108's unit without `sameIdentity` against TC-458's
   fixture package, and decode the emitted `quire.checked-package/v2` graph.
2. For each `state` node, compare its `semantic_type`, `body`,
   `dependencies` and occurrences with FR-105's Outputs table.
3. Walk `VersionUnchanged`'s, `NoCycle`'s and `ParentOrder`'s condition
   terms.
4. Run `CheckedClauseKind`'s mapping forward over all eight variants and
   backward over every triple the forward pass gives and over
   (`state`, `frame`); run `cargo mutants` over the mapping functions.
5. Spine-compile a unit holding TC-459 step 7's `P`, `Q` and `I` against
   that step's package, and decode the emitted graph; then compile it with
   only `I`.
6. Read the `source_map` entry of step 1's frame node's `generated`
   occurrence; recompile with a second `post` clause naming `attemptUpdate`
   placed before `VersionUnchanged`; compile TC-1043's fixture R; emit a package
   whose frame occurrence has its region removed (FR-105-AC-9).
7. Spine-compile step 1's unit with `constraint Low using v on
   Config::ConfigVersion at current { self.versionNumber <= 1 }` added, then
   with `constraint` replaced by `invariant`; read each package with QSL's I2
   read; decode (`state`, `state_clause`, `quire.op.state.clause`,
   `constraint`).
8. Compile a unit with only an attempt naming `attemptUpdate`, then units with
   both the attempt and a point-bearing `pre` clause for `attemptUpdate`, once
   with the attempt before the clause and once after it. Supply the `pre`
   clause's accepted point through TC-475's input; compare the frame
   occurrence's region and its `(node id, role, ordinal)`.
9. Supply the accepted points for authored clauses through TC-475's S3 input,
   keyed by each clause's exact source name. For accepted initialization and
   handler invariants and pre/post clauses, compare each emitted execution
   point to the accepted ClauseBinding; mutate
   its variant and anchor name independently, and compare the resulting
   `state_clause` and `operation_anchor` identities. Include a protocol-only
   attempt anchor for the same operation as a point-bearing precondition.


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
- Step 5: one `state`/`operation_anchor` for `go`, with context the `Flow`
  `model`/`process` node and operation the text literal `"go"`; one
  `state`/`frame`, whose `semantic_type` is the `Flow` node and whose
  `modifies` is exactly (`Flow` node, `n`, scope `every-object`), with
  `creates`, `deletes` and `deletes_scoped` empty; one `operation-contract`
  frame record for `go`; two `anchor` occurrences of the anchor, for `P` and
  `Q`. `I`'s `state_clause` references the `Flow` node. With only `I`, the
  package holds no anchor, frame node or frame record for `go`. As for step
  1, the emitted-node assertions wait on STD-111; the checked-graph
  assertions do not.
- Step 6: one region, the span of `attemptUpdate`'s reference in
  `VersionUnchanged`; with the earlier clause, that clause's span and the
  same (node id, role, ordinal); in R, each operation's first naming span;
  the regionless occurrence refuses
  `UnlocatedOccurrence`.
- Step 7: one more `state_clause` node, kind member `constraint`, parameter
  aggregate `[self]`, anchor the `ConfigVersion` node, one `claim`
  occurrence, no new anchor or frame; the `invariant` variant has another
  node id; both packages read; the triple decodes to `Constraint`.
- Step 8: with only the attempt, the frame's `generated` occurrence region is
  the qualified operation reference in the attempt, not its declared name.
  With both declarations, attempt-first selects the attempt reference and
  clause-first selects the clause reference. The frame occurrence's
  `(node_id, role, ordinal)` is identical after reordering. The point-bearing
  precondition's `state_clause` and its operation anchor carry the same point;
  the protocol-only attempt anchor omits it and has a distinct node identity.
- Step 9: initialization and handler invariants with equal bodies retain
  distinct points and node identities; the constraint in TC-475 also carries
  its accepted initialization point. Each operation point exactly matches
  the ClauseBinding and the operation authored in `on M::T::op`; a change to
  the variant or anchor name changes the affected node identity. The
  operation clause's `state_clause` and `operation_anchor` carry the same
  point. A point-bearing precondition and the same operation's protocol-only
  attempt have distinct anchor identities; the attempt anchor omits the
  point.
