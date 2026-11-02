---
id: FR-515
title: "Read the checked predicate and the temporal subject from a checked package"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-051
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-449
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-499
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-500
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-513
    type: references
---
# FR-515: Read the checked predicate and the temporal subject from a checked package

## Description

FR-051 and its modules `protocol_artifact::checked_predicate` and
`protocol_artifact::temporal_subject` were retired by QSL-303 (M-6d) with the
statement "nothing outside the deleted modules' own tests called them". That
statement is false for quire-protocol: `src/assessment.rs` and
`src/result/record.rs` take a `ValidatedCheckedPredicate` and a
`ValidatedTemporalSubject` (declaration, leaf, predicate leaves, package
binding and document identity) as the subject of every temporal assessment
(QSL-1501 symbol map, row A). The QSL-1501 repoint therefore needs a successor
that is a **view over the checked package**, not a document: no JSON contract,
no schema bytes, no producer and no `read(bytes, ...)` round trip.

This requirement specifies the two views, the node facts each accessor returns,
and which accessors of the retired modules end and why. It adds no checked-package
member and no Contract IR code: the views read the nodes FR-337 and FR-383 already
emit through the public surface of `quire_contract_model::CheckedPackageV2`.

## Use case

quire-protocol reads a handoff package with Contract IR's reader and holds a
`CheckedPackageV2`. For a temporal clause `Due` of that package it needs the
clause's identity triple, its `over` node, its profile and clock row, its
activation, its captures, the operators and intervals of its formula and the
Boolean operand node of every `holds` atom, so that an assessment names exactly
the obligation the source authored. It calls QSL for that view and does not
parse the clause body itself.

## Semantic authority and boundary

QSpec FR-370 owns the clause body; FR-337 states what QSL emits; Contract IR owns
the reader. This requirement states only what QSL returns from an admitted
package. The views select no truth, hold no evaluation callback and assert no
observation or result state (the FR-051-AC-6 history text, kept).

## Inputs

- A `&quire_contract_model::CheckedPackageV2` returned by
  `read_checked_package`, or the retained package of QSL's successful I2 read
  (`V2Read`). A caller cannot construct either view from bytes, from a decoded
  graph or from a term.
- For the temporal subject, the `CheckedNodeId` of a `temporal`/`temporal_clause`
  node.
- For the predicate, a `PredicateSelection { clause: CheckedNodeId, leaf:
  CheckedNodeId }`.

## Outputs

In `qsl_package::checked_views` (std only; no new dependency):

```rust
pub fn temporal_subject<'p>(package: &'p CheckedPackageV2, clause: &CheckedNodeId)
    -> Result<TemporalSubjectView<'p>, ViewRefusal>;
pub fn checked_predicate<'p>(package: &'p CheckedPackageV2, selection: &PredicateSelection)
    -> Result<CheckedPredicateView<'p>, ViewRefusal>;
```

Both views borrow the package, have no public constructor and no method that
evaluates. `ViewRefusal` (shared with FR-519) is a closed enum; the variants of this requirement are:

- `UnresolvedClause`: FR-337's `clause-root-unresolved` lookup refusal, reused;
- `LeafNotInClause`: the leaf is no term the clause reads (it is reachable
  from neither the formula root nor the activation guard);
- `LeafHasNoSource`: the leaf is a term of the clause, and the package source
  map holds no entry for it;
- `LeafNotBoolean`: the leaf's semantic type is not the `boolean` node.

## Behavior

### The temporal subject

| Retired accessor of `ValidatedTemporalSubject` | Successor | Source in the package |
| --- | --- | --- |
| `declaration()` (u32 index) | `declaration() -> &CheckedNodeId` | the clause node id |
| `root()` (handle) | `occurrences() -> &[ClauseOccurrence]`, each `{ node, role: claim, ordinal }` | FR-337's identity triples; one per declaration of a structurally equal clause |
| none (implicit in `root`) | `over() -> &CheckedNodeId` | the first argument of the `quire.op.temporal.clause` application (FR-337) |
| `clause()` (requirement, name, execution) | `requirements() -> &[&CheckedNodeId]`; `name() -> &str` once IR-1278 decides the package carries the clause's name, and none until then | `requirements()`: the node ids of the protocol clauses whose `requirements` entries name this clause (the FR-449 rows whose `required_temporal` is this clause), in row order. The package holds no declared name for a temporal clause: the clause node has no `declaration` member and the reader refuses one (IR-1278) |
| `source()` (source, span) | `source_span() -> &CheckedSourceMapEntry` | the package source map entry of the clause node with role `claim`; its region covers the clause's name token only, not the formula |
| `profile_identities()` | `profile() -> &CheckedSelection` | the lock `profile_selections` row whose definition equals the clause's `temporal_profile` law |
| `clock()` | `clock() -> Option<&TemporalBinding>` | the `temporal_bindings()` row for this declaration; `None` when the lock selects no clock configuration |
| `activation()` | `activation() -> Activation` | `Origin`, or `Each { trigger: CheckedNodeId, guard: Option<CheckedNodeId> }` from the clause's activation argument |
| `captures()` | `captures() -> &[CheckedNodeId]` | the clause's captures argument, in source order |
| `operators()`, `required_history()` | `formula() -> Vec<OperatorUse>`, each `{ operation: &str, interval: Option<Interval> }` in formula pre-order | the `quire.op.temporal.*` applications and their `temporal_interval` members (FR-337); the history span is derived by FR-043, not carried |
| `predicate_leaves()` | `predicate_leaves() -> &[CheckedNodeId]` | the Boolean operand node of every `holds` atom in formula pre-order, one entry per atom. The activation guard is a separate node of the clause's activation argument; equal terms share one node (FR-337), so a guard equal to a `holds` operand is that same node and is among the entries |
| `history_boundary()` | none | a property of an evaluation input (FR-516), not of the package |
| `values()`, `bindings()`, `type_indices()`, `temporal_nodes()` | none | the checked graph is the closed table; a caller resolves a node id with `CheckedPackageV2::term` |
| `package_artifact()`, `package_digest()`, `document().digest()`, `document().identity()`, `document().bytes()`, `SCHEMA_BYTES`, `SCHEMA_SHA256`, the `identity` label | none | the digest, the schema and the document go with the retired contract (QSL-1270 removed digests); the package binding is by lifetime and by `belongs_to` below |

`belongs_to(&self, package: &CheckedPackageV2) -> bool` is true exactly for the
package the view was read from (the check quire-protocol made by comparing
`package_digest()` values).

### The checked predicate

| Retired accessor of `ValidatedCheckedPredicate` | Successor |
| --- | --- |
| `declaration()` | `declaration() -> &CheckedNodeId` (the clause) |
| `leaf()` | `leaf() -> &CheckedNodeId` |
| none | `kind() -> PredicateKind`: `HoldsLeaf`, `ActivationGuard`, `ClauseLeaf` |
| `parent_kind()` | `parent_operation() -> Option<&str>` (the operation identity of the leaf's parent application) |
| `parameters()` | `binders() -> &[CheckedNodeId]`: the binder nodes of the clause's `over` and activation trigger and its captures that the leaf term reads, in first-reference order |
| `source()` | `source_span() -> &CheckedSourceMapEntry`: the entry of the leaf node, the one whose regions lie inside the source regions of the clause's formula root node when the leaf is shared with another clause |
| `clause()` | `name()` once IR-1278 decides the package carries the clause's name, and none until then |
| `type_indices()` | none: the leaf's semantic type is the `boolean` node, checked at selection |
| `profile_identities()` | `profile()` |
| `package_*`, `document()` | as for the temporal subject |

`kind()` is decided by node identity, in this order: a leaf that is the
activation guard node is `ActivationGuard`; otherwise a leaf that is among
`predicate_leaves()` is `HoldsLeaf`; otherwise a leaf the formula reads is
`ClauseLeaf`. A guard that shares its node with a `holds` operand therefore
reads as `ActivationGuard` and is also among `predicate_leaves()`.

A `ClauseLeaf` selection is admitted for a Boolean term that is neither a `holds`
operand nor the activation guard but is a complete Boolean leaf of the clause's
formula. `ActivationGuard` is the guard of an `Each` activation and is admitted
only when one is written.

### Determinism and independence

- Both views are functions of the package alone: the view read from the
  in-process package's emitted bytes through the reader equals the view read from
  the same bytes again, and a comment or whitespace change in the source leaves
  every returned id unchanged (FR-337).
- Two structurally equal clauses share one node; each declaration keeps its own
  `ClauseOccurrence`, and the view of the shared node returns both. Under a
  clock-configured temporal profile the writer emits one `temporal_bindings`
  row per declaration and Contract IR's reader refuses the second row for the
  shared node (`invalid_package`/`timed-node-reference`), so that case has no
  admitted package until the Contract IR decision named in FR-515-AC-7 lands.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-515-AC-1 | Over each of the three packages of **R** (event position, fixed sample, timestamped event; FR-517) and each of its temporal clauses (`Due` and `DueLate`, `DueSample`, `DueTimestamp`), `temporal_subject` returns a view whose `declaration`, `occurrences`, `over`, `source_span`, `profile`, `clock`, `activation`, `captures` and `formula` each equal the value read directly from the node or lock by the test, and whose `clock()` equals the row of `CheckedPackageV2::temporal_bindings()` for that declaration. | Test (TC-1310) |
| FR-515-AC-2 | For a clause with `on each (q: T) when (g)`, `activation()` is `Each` with the trigger binder node and the guard node; for `on origin` it is `Origin`; and for `on each` without a guard it is `Each` with `guard: None`. | Test (TC-1310) |
| FR-515-AC-3 | For `always[0,30] holds(Allowed(view.ratio))` (the recipe's `Due`), `formula()` is the `always` operation with interval `[0, 30]` followed by the `holds` application, and `predicate_leaves()` holds the one Boolean operand node; for a formula with three `holds` atoms it holds three nodes in formula pre-order. | Test (TC-1310) |
| FR-515-AC-4 | `checked_predicate` for the `HoldsLeaf` of AC-3 returns a view with `kind() = HoldsLeaf`, the leaf, the clause, `binders()` naming `view` and no other binder, and a `source_span` whose regions lie inside the source regions of the clause's formula root node. | Test (TC-1310) |
| FR-515-AC-5 | `temporal_subject` of a node id that is absent, that names a non-temporal node (a `protocol_clause`), or that names a temporal node of another form refuses `UnresolvedClause`; `checked_predicate` of a leaf the clause does not read refuses `LeafNotInClause`, of an integer-typed node `LeafNotBoolean`; none returns a partial view. | Test (TC-1310) |
| FR-515-AC-6 | A view read from package **P1** answers `belongs_to(P1)` true and `belongs_to(P2)` false for a second package **P2** whose clause has the same name, and the lifetime of the view ends with the borrow of its package. | Test (TC-1310) |
| FR-515-AC-7 | Two structurally equal clauses of one package under an infinite-trace temporal profile give one node and two `ClauseOccurrence` values with different ordinals, returned by the view of that node; changing one interval of the formula changes `formula()` and the node id, and adding a source comment changes neither. | Test (TC-1310) |
| FR-515-AC-8 | The views expose no method that takes a closure or evaluates a term, no public constructor, and none of `digest`, `bytes`, `identity`, `package_digest`, `SCHEMA_BYTES`; a compile-fail test that constructs either view outside the module, and compile-fail tests that call `digest()`, `bytes()`, `identity()`, `package_digest()` and a closure-taking method on either view, each fail to compile. | Test (TC-1310) |
| FR-515-AC-9 | `quire_spec_language::protocol_artifact::checked_predicate` and `temporal_subject` do not resolve; QSL-303 retired them. | Test (TC-1310, TC-1316) |
| FR-515-AC-10 | For a clause whose guard `g` is a different node from every `holds` operand, `checked_predicate` of `g` is `ActivationGuard` and `g` is absent from `predicate_leaves()`. | Test (TC-1310) |
| FR-515-AC-11 | For a clause whose guard is equal to a `holds` operand, so that the two are one node, `checked_predicate` of that node is `ActivationGuard` and the node is present in `predicate_leaves()` once for each `holds` atom that reads it. | Test (TC-1310) |
| FR-515-AC-12 | `checked_predicate` of a leaf the clause reads and for which the package source map holds no entry refuses `LeafHasNoSource` and returns no partial view. | Test (TC-1310) |
| FR-515-AC-13 | Two structurally equal clauses under a clock-configured temporal profile give one node and two `ClauseOccurrence` values, and the package is admitted by `read_checked_package`. This is gated on a Contract IR decision for the shared node's `temporal_bindings` row (IR-1279). | Test (TC-1310) |
| FR-515-AC-14 | With a reader that implements IR-1278, `name()` of a view returns the clause's declared name, equal to the name in the selection sidecar. This is gated on IR-1278. | Test (TC-1310) |

## Dependencies

- **Upstream:** FR-337, FR-383, FR-449, FR-051 (retired modules), QSL-303,
  QSL-1270 (digest removal), `quire_contract_model::CheckedPackageV2`; IR-1278
  (the clause name, gates FR-515-AC-14); IR-1279 (the shared
  node's clock row, gates FR-515-AC-13).
- **Downstream:** quire-protocol `src/assessment.rs`, `src/result/record.rs`
  (QSL-1549).
