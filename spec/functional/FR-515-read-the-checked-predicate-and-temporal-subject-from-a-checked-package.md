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
| `clock()` | `clock() -> Option<&TemporalBinding>` | the `temporal_bindings()` row for this declaration's occurrence when the node has one occurrence; `None` when the lock selects no clock configuration and `None` for a node with several occurrences, whose rows `clock_of(&ClauseOccurrence)` returns by the key `{ declaration, role, ordinal }` |
| `activation()` | `activation() -> Activation` | `Origin`, or `Each { trigger: CheckedNodeId, guard: Option<CheckedNodeId> }` from the clause's activation argument |
| `captures()` | `captures() -> &[CheckedNodeId]` | the clause's captures argument, in source order |
| `operators()`, `required_history()` | `formula() -> Vec<OperatorUse>`, each `{ operation: &str, interval: Option<Interval> }` in formula pre-order; and `formula_graph() -> Vec<FormulaNodeUse>`, each `{ operation: &str, interval: Option<Interval>, operands: &[usize] }` in formula pre-order, `operands` being the positions in that list of the node's operand entries | `formula()` lists the `quire.op.temporal.*` applications and their `temporal_interval` members (FR-337) only; `formula_graph()` lists every formula node the clause reads: the temporal applications, the `holds` applications, the connectives and the constants, a `holds` atom, a constant and a leaf having no operands; the history span is derived by FR-043, not carried |
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
| `source()` | `source_span() -> &CheckedSourceMapEntry`: for a `HoldsLeaf` or `ClauseLeaf`, the entry of the leaf node whose regions lie inside the source regions of the clause's formula root node, which picks this clause's reading of a leaf shared with another clause; for an `ActivationGuard`, an entry of the guard node, because the guard is written in the clause's activation and not in its formula |
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
  row per clause occurrence, keyed by the declaration and the occurrence
  `{ role, ordinal }` with the producer-assigned ordinal (QSpec FR-322 "Temporal
  clock bindings"), and the reader admits one row per occurrence of a shared
  node. `occurrences()` lists a node's occurrences in the order of the node's
  `occurrences` array as the package carries it, which QSpec FR-322 does not
  constrain, while the lock lists the clock rows ascending by occurrence. QSL's own
  emitter assigns ordinals in ascending order, so both orders agree for every
  package it emits; a package from another producer may carry them differently.
  `clock_of(&ClauseOccurrence) -> Option<&TemporalBinding>` returns the row
  whose key equals that occurrence's `{ role, ordinal }`; it never pairs by the
  position of the occurrence or of the row, because producer-assigned ordinals
  need not run consecutively from zero. `clock()` returns the row of the node's
  only occurrence and is `None` for a node with several occurrences, which
  `clock_of` answers.

### The in-process package

The evaluation entry is the type `EvidencePackage` in `qsl-eval`, built from an
`Emission` (FR-337) with `EvidencePackage::from_emission(&Emission,
&AdmittedPackageRead)` or from a read-back package with
`EvidencePackage::from_read(&AdmittedPackageRead)`. Both answer both views, and
every view of the first equals the view of the second. The in-memory trait the
checker's package implements is a sealed in-process view of the package and is
not an entry. The pre-emission `CheckedPackage` of the checker carries no clock
rows and no profile selections, because the clock configuration is an input to
emission (FR-050), so it is not a package this requirement reads; evaluation
starts from an emission.

- **Construction refusals.** Construction refuses a package whose parts
  disagree with a typed `PackageMismatch`, returned at construction, which is
  not a `CallFailure` and charges nothing. Each variant has one route:
  `ClauseRow`, returned by `from_emission` only, when the clause at an `Emission`
  clause reference's position carries another name than the name row (`from_read`
  builds its clause references from the name rows, so the case cannot arise
  there); `AmbiguousName`, returned by `from_read` only, when two clauses carry
  one name or two name rows pair to one clause (`from_emission` pairs by the
  `clause` position, so duplicate names do not affect it); `ProfileSelections`,
  returned by `from_emission` only, when the `Emission`'s `profile_selections` do
  not match the package; and `Bindings`, returned by `from_emission` only, when
  the `Emission`'s `temporal_bindings` rows do not match the package.

- **Rows at construction.** `EvidencePackage::from_emission` takes the
  `profile_selections` rows and the `temporal_bindings` rows (one per clause
  occurrence) from the `Emission` exactly as the emitter wrote them, and holds
  them from construction. It recomputes none of them and takes no clock from any
  other argument, so no call can read a clock the lock does not hold.
- **Ids.** `from_emission` also takes the `TemporalClauseRef` values of the
  `Emission` (FR-337): the `CheckedNodeId` of each clause occurrence is the id the
  emitted bytes assign, so a caller holding the `Emission` names a clause by the
  same `CheckedNodeId` a caller holding the read-back package names it by.
  `CheckedNodeId` stays hash-assigned during emission, and no caller derives one.
  This choice keeps the in-process and the read-back route identical, because the
  in-process route uses the emitter's own values and no second derivation exists.
- **Clause and row by key.** A `CheckedNodeId` plus an occurrence `{ role, ordinal }`
  selects the clause occurrence, and its clock row by the key `{ declaration, role,
  ordinal }`, never by position among the node's occurrences or rows. The
  in-process route pairs a `TemporalClauseRef` to the in-memory clause by its
  `clause` position and the read-back route pairs a `temporal_clause_names` row to
  its clause by `{ declaration, occurrence }`, and the two pair the same clause.
  Two clauses with different nodes never read each other's rows.
- **One evaluator formula.** The evaluator's formula is built from the subject
  view's `formula_graph()` entries and their `operands` edges by one construction that
  both routes share, and from no other data, so the `EvidencePackage` built from
  an `Emission` and the one built from the emitted bytes give the evaluator the
  same formula. The cross-check is one comparison on both
  routes, so truth evaluation runs only on a clause proven equal to the package's
  graph. The comparison is between the package's `formula_graph()` and one source
  clause: the clause at the clause reference's `clause` position in the admitted
  read's source (`AdmittedPackageRead::source()`, the source package the bytes
  were emitted from), compared in pre-order for operations, intervals and operand
  edges. `EvidencePackage::from_emission(&Emission, &AdmittedPackageRead)` reaches
  that clause through the `Emission`'s clause reference (an `Emission` holds
  rows and references, not a clause), and
  `EvidencePackage::from_read(&AdmittedPackageRead)` reaches it through the name
  row pairing. On any difference the first call that reaches the clause refuses
  `clause-unreadable` (FR-516), with an empty charge log and the caller's
  ledgers unchanged. An evaluator over `formula_graph()` alone with no source
  graph is not part of this requirement; it is a separate future capability.
  A cycle among the `operands` edges and an operand
  position that names no entry are detected on those edges (FR-516
  `clause-unreadable`).
- **Instance-bound atoms are derived.** The capture binders of a clause are
  `captures()`; the trigger binder is the `trigger` node of `activation()`
  (`Each`) and the `over()` binder is neither. A `holds` atom cannot read the
  trigger binder (the checker refuses it as `MissingName`), so the instance-bound
  atoms of a clause are exactly the `holds` atoms whose operand `binders()`
  (checked predicate view) include a capture binder; an atom that reads only the
  `over()` binder, a constant or no binder is not instance-bound, and the
  activation guard is not a `holds` atom and has no atom index. The mapping to
  the evaluator's atom indices is by formula order: the `k`-th entry of
  `predicate_leaves()` (one per `holds` atom in formula pre-order) is the `k`-th
  `holds` node in the clause's formula, whose index is the atom index the
  evaluator reads valuations by. `EvidenceSubject.instance_bound` is the set of
  those indices computed from the view in both routes, and no caller supplies it.
  When `predicate_leaves()` and the formula's `holds` nodes differ in count, the
  clause is unreadable (FR-516 `clause-unreadable`).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-515-AC-1 | Over each of the three packages of **R** (event position, fixed sample, timestamped event; FR-517) and each of its temporal clauses (`Due` and `DueLate`, `DueSample`, `DueTimestamp`), `temporal_subject` returns a view whose `declaration`, `occurrences`, `over`, `requirements`, `source_span`, `profile`, `clock`, `activation`, `captures` and `formula` each equal the value read directly from the node or lock by the test, and whose `clock()` equals the row of `CheckedPackageV2::temporal_bindings()` for that declaration. | Test (TC-1310) |
| FR-515-AC-2 | For a clause with `on each (q: T) when (g)`, `activation()` is `Each` with the trigger binder node and the guard node; for `on origin` it is `Origin`; and for `on each` without a guard it is `Each` with `guard: None`. | Test (TC-1310) |
| FR-515-AC-3 | For `always[0,30] holds(Allowed(view.ratio))` (the recipe's `Due`), `formula()` is the `always` operation with interval `[0, 30]` followed by the `holds` application, and `predicate_leaves()` holds the one Boolean operand node; for a formula with three `holds` atoms it holds three nodes in formula pre-order. | Test (TC-1310) |
| FR-515-AC-4 | `checked_predicate` for the `HoldsLeaf` of AC-3 returns a view with `kind() = HoldsLeaf`, the leaf, the clause, `binders()` naming `view` and no other binder, and a `source_span` whose regions lie inside the source regions of the clause's formula root node. | Test (TC-1310) |
| FR-515-AC-5 | `temporal_subject` of a node id that is absent, that names a non-temporal node (a `protocol_clause`), or that names a temporal node of another form refuses `UnresolvedClause`; `checked_predicate` of a leaf the clause does not read refuses `LeafNotInClause`, of an integer-typed node `LeafNotBoolean`; none returns a partial view. | Test (TC-1310) |
| FR-515-AC-6 | A view read from package **P1** answers `belongs_to(P1)` true and `belongs_to(P2)` false for a second package **P2** emitted from the same source, and the lifetime of the view ends with the borrow of its package. | Test (TC-1310) |
| FR-515-AC-7 | Two structurally equal clauses of one package under an infinite-trace temporal profile give one node and two `ClauseOccurrence` values with different ordinals, returned by the view of that node; changing one interval of the formula changes `formula()` and the node id, and adding a source comment changes neither. | Test (TC-1310) |
| FR-515-AC-8 | The views expose no method that takes a closure or evaluates a term, no public constructor, and none of `digest`, `bytes`, `identity`, `package_digest`, `SCHEMA_BYTES`; a compile-fail test that constructs either view outside the module, and compile-fail tests that call `digest()`, `bytes()`, `identity()`, `package_digest()` and a closure-taking method on either view, each fail to compile. | Test (TC-1310) |
| FR-515-AC-9 | `quire_spec_language::protocol_artifact::checked_predicate` and `quire_spec_language::protocol_artifact::temporal_subject` do not resolve; a compile-fail `use` of each fails. | Test (TC-1310) |
| FR-515-AC-10 | For a clause whose guard `g` is a different node from every `holds` operand, `checked_predicate` of `g` is `ActivationGuard` and `g` is absent from `predicate_leaves()`. | Test (TC-1310) |
| FR-515-AC-11 | For a clause whose guard is equal to a `holds` operand, so that the two are one node, `checked_predicate` of that node is `ActivationGuard` and the node is present in `predicate_leaves()` once for each `holds` atom that reads it. | Test (TC-1310) |
| FR-515-AC-12 | `checked_predicate` of a leaf the clause reads and for which the package source map holds no entry refuses `LeafHasNoSource` and returns no partial view. | Test (TC-1310) |
| FR-515-AC-13 | Two structurally equal clauses under a clock-configured temporal profile give one node and two `ClauseOccurrence` values, and the package is admitted by `read_checked_package` with one `temporal_bindings` row per occurrence (QSpec FR-322 vectors BO-P1 to BO-R9). | Test (TC-1310) |
| FR-515-AC-14 | With a reader that implements IR-1278, `name()` of a view returns the clause's declared name, equal to the name in the selection sidecar. This is gated on IR-1278. | Test (TC-1310) |
| FR-515-AC-15 | `requirements()` of a temporal clause that no protocol `requirements` entry names is empty, and of a clause that a protocol's `requirements` entry names holds that protocol clause's node id. | Test (TC-1310) |
| FR-515-AC-16 | `checked_predicate` of an activation guard returns a view whose `source_span` is one of the source map entries of the guard node. | Test (TC-1310) |
| FR-515-AC-17 | `checked_predicate` of a `ClauseLeaf` returns a view whose `source_span` is the entry of the leaf node whose regions lie inside the source regions of the clause's formula root node. | Test (TC-1310) |
| FR-515-AC-18 | Over **R**'s three packages (one per clock kind), holding four temporal clauses (FR-517), `temporal_subject` over the `EvidencePackage` constructed from the `Emission` returns `profile()`, `clock()` and `occurrences()` equal to those over the package read back from the emitted bytes, and the `profile_selections` and `temporal_bindings` rows it holds equal the `Emission`'s rows byte for byte. | Test (TC-1310) |
| FR-515-AC-19 | In a unit the test authors over the `config-version` model (`examples/config-version`), whose temporal clauses (capture, `on each` activation, guard) the test writes, since the example holds none, with two temporal clauses of different nodes and different clock rows, each node's `clock()` is its own occurrence's row, in the evaluation package constructed from the `Emission` and in the read-back package alike. | Test (TC-1310) |
| FR-515-AC-20 | Over a constructed V2 package of twins of one node, whose node `occurrences` array order is ordinal 7 then ordinal 3 while the lock's clock rows ascend as ordinal 3 then ordinal 7, with different clock configurations, read back through the reader (QSL's emitter cannot produce this order, so the package is constructed and no `Emission` or in-process counterpart exists for it), `clock_of` of the occurrence with ordinal 3 returns the row keyed ordinal 3, `clock_of` of ordinal 7 returns the row keyed ordinal 7, and `clock()` is `None`; a pairing by position fails this criterion. | Test (TC-1310) |
| FR-515-AC-21 | For each `TemporalClauseRef` of an `Emission`, the `CheckedNodeId` equals the `declaration` of the `temporal_clause_names` row read back at the same occurrence, and the `CheckedNodeId` of a clause in the evaluation package constructed from the `Emission` equals the one in the read-back package; a derivation of ids outside the emitter does not exist in the API. | Test (TC-1310) |
| FR-515-AC-22 | In a unit the test authors over the `config-version` model (`examples/config-version`), whose temporal clauses (capture, `on each` activation, guard) the test writes, since the example holds none, for a clause with a guard that reads the trigger binder and four `holds` atoms in formula order at indices 0 to 3, one reading a capture binder (index 0), one reading only `over()` (index 1), one reading no binder (index 2) and one reading a capture binder and `over()` (index 3), the derived instance-bound set is exactly `{0, 3}`, the guard has no index, and the set equals the one the read-back package gives; swapping the formula order of the first and the second atom gives `{1, 3}`; a caller-supplied set is not accepted. | Test (TC-1310) |
| FR-515-AC-23 | For each clause of **R** and the authored clause of AC-22, the evaluator's formula built from the evaluation package constructed from the `Emission` equals, node for node with operations, intervals and operand edges, the formula built from the read-back package; `formula_graph()` of AC-3's clause lists the `always` entry with one operand position that names the `holds` entry, while `formula()` of that clause lists the `always` and `holds` applications exactly as AC-3 states; and a view whose `operands` hold a cycle or an out-of-range position makes the clause unreadable (FR-516). | Test (TC-1310) |
| FR-515-AC-24 | In a unit the test authors over the `config-version` model (`examples/config-version`), whose temporal clauses (capture, `on each` activation, guard) the test writes since the example holds none, the test writes the package through an emitter fixture that mutates one written operation, one interval or one operand edge (each in its own run), so the written package differs from the source clause, and passes the result to the read. The `EvidencePackage` built by `from_emission` and the one built by `from_read` each refuse `clause-unreadable` at the first call that reaches the clause, with an empty charge log; the unmutated package is evaluated on each route. | Test (TC-1310) |
| FR-515-AC-25 | Over an `Emission` whose clause reference at a position points at a clause carrying another name than the name row, `from_emission` returns `PackageMismatch::ClauseRow` at construction and no call is made or charged. | Test (TC-1310) |
| FR-515-AC-26 | Over a read package with two clauses of one name, and over one with two name rows pairing to one clause, `from_read` returns `PackageMismatch::AmbiguousName` in each; `from_emission` over the two-clauses-of-one-name package pairs by `clause` position and constructs. | Test (TC-1310) |
| FR-515-AC-27 | Over an `Emission` whose `profile_selections` do not match the package, `from_emission` returns `PackageMismatch::ProfileSelections`. | Test (TC-1310) |
| FR-515-AC-28 | Over an `Emission` whose `temporal_bindings` rows do not match the package, `from_emission` returns `PackageMismatch::Bindings`. | Test (TC-1310) |

## Dependencies

- **Upstream:** FR-337, FR-383, FR-449, FR-051 (retired modules), QSL-303,
  QSL-1270 (digest removal), `quire_contract_model::CheckedPackageV2`; IR-1278
  (the clause name, gates FR-515-AC-14); QSpec FR-322 (the per-occurrence clock
  rows FR-515-AC-13 and AC-20 read).
- **Delivery:** FR-515-AC-18 to AC-28 and FR-516-AC-18, AC-29 and AC-30 are delivered by QSL-1608, which builds the `Emission`-constructed package and the shared formula construction.
- **Downstream:** quire-protocol `src/assessment.rs`, `src/result/record.rs`
  (QSL-1549).
