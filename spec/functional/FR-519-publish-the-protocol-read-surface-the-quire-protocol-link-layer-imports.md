---
id: FR-519
title: "Publish the protocol read surface the quire-protocol link layer imports"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-384
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-449
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-509
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-511
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-515
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-499
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-381
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-520
    type: references
---
# FR-519: Publish the protocol read surface the quire-protocol link layer imports

## Description

quire-protocol's link layer (`src/link/mod.rs`, owner chunk C-12) builds a finite
linked protocol from QSL's typed `wire::Package`: `Handle`, `Locus`, `EdgeKind`,
`CausalEdge`, `Declaration`, `Body`, `ControlOperation`, `Binder`, `BindingKind`
and others. Its own rule is "an owner publishes it, import
it; do not mirror it": QSL derived and compared the causal edge set, resolved
references and checked joins, and the link layer refused only what QSL did not.
`wire::Package` has no checked-package equivalent (QSL-1501 symbol map, row B):
the checked package holds protocol nodes (FR-383) and carries **no** causal edge
(FR-383-AC-16); the relation is derived from the nodes by QSpec FR-464's rules.
QSL derives that relation in production from two other inputs: the checker's
`compile` (`qsl-semantics/src/check/protocol_clause/graph.rs`) over the source
forms of a clause, and the expansion `edges`
(`src/protocol_artifact/native/controls.rs`) over the controls of a `wire`
package until QSL-838 retires it. Nothing in production derives it from the
nodes of a `CheckedPackageV2`; QSL's emit tests do
(`qsl-package/src/emit/tests/protocol_graph.rs`).

This requirement divides the work. **QSL provides** the derivation as a
production function and a record-to-node map, so the link layer imports the owner's
derivation. **quire-protocol owns** a typed read view over `CheckedPackageV2` for
the link layer; QSL owns none of that code, and the requirement on it is the
cross-repository ticket QSL-1549, stated below for the record.

## Semantic authority and boundary

QSpec FR-464 owns the causal relation and the node bodies; FR-383 states what QSL
emits; Contract IR owns the reader (`quire_contract_model::read_checked_package`,
which quire-protocol calls itself: QSL does not wrap it, fork it or add IR code).
QSL's functions here take `&CheckedPackageV2`.

## Inputs

A `&CheckedPackageV2` and the `CheckedNodeId` of a `protocol`/`protocol_clause`
node; for the attempts maximum, the `CheckedNodeId` of a compensation node.

## Outputs

In `qsl_package::protocol_view` (std only):

```rust
pub fn attempts_maximum(package: &CheckedPackageV2, compensation: &CheckedNodeId)
    -> Result<u64, ViewRefusal>;
pub fn causal_relation(package: &CheckedPackageV2, protocol: &CheckedNodeId)
    -> Result<CausalRelation, ViewRefusal>;
pub fn protocol_view<'p>(package: &'p CheckedPackageV2, protocol: &CheckedNodeId)
    -> Result<ProtocolView<'p>, ViewRefusal>;
```

`CausalRelation` is the strictly ascending, duplicate-free sequence of
`CausalEdge { from: PortRef, to: PortRef, kind: EdgeKind }`, with this
vocabulary, which is the vocabulary of FR-218's compiled edge relation:

- `Port` is `Enter` or `Exit`: the point at which a protocol node is entered
  or left.
- `PortRef { node: CheckedNodeId, port: Port }` names one port of one protocol
  node of the clause (a control, a `parallel` branch or a `choice` case body,
  a `send`, a `receive`, an `attempt`, an `effect`).
- `EdgeKind` is one of `Sequence`, `Branch`, `Join`, `RepeatProgress`,
  `RepeatBack`, `AwaitSuccess`, `AwaitTimeout`, `AwaitAnchor`, `Message`,
  `Effect`, the rules of QSpec FR-464 "Derived causal relation".
- The order is ascending by `from`, then `to`, then `kind`; a `PortRef` orders
  by node id, then `Enter` before `Exit`.

`ProtocolView` borrows
the package and lists, as node ids in the order the nodes carry them: the clause,
its roles, channels, queues, controls in layout order, compensations,
`requirements` (temporal clauses), `terminal` and `scheduling` when present, and
the FR-449 activation rows (`protocol_activation_relation`).

`attempts_maximum` returns the value of the `maximum` member of the `attempts`
group of the compensation node `compensation`, the retry bound FR-381 checks as
`maximum_attempts`: a `u64` of at least one, counting the initial attempt, held in
the integer encoding of QSpec FR-322. It refuses with `ViewRefusal`, and returns
no value, when `compensation` is absent from the package or names a node that is
not a compensation (`ViewRefusal::NotACompensation`, naming the id), when the
node's `attempts` group lacks `maximum` (`MissingMember` naming
`attempts.maximum`), and when `maximum` is not an integer or is zero
(`MalformedMember` naming `attempts.maximum`). FR-520-AC-4 compares the retry
bound it reads through this function, so the two requirements state one value.

## Behavior

### The record-to-node map

The map ends in one of two dispositions. A record is **read by a QSL view
function** when QSL's `protocol_view` module returns its successor, or it is
**consumed by name** when the successor is a Contract IR type that the caller
reads directly and QSL publishes no view for it. The view functions are
`protocol_view`, `causal_relation`, `control_operation`, `operation_identity`,
`binder`, `locus`, `delivery`, `queue_capacity`, `owner_model`, `declaration`,
`temporal_binding`, `dependency`, `source`, `selection` and `attempts_maximum`.
`protocol_view`, `causal_relation`, `binder` and `attempts_maximum` take an
admitted package; the others take the node, lock or source map they read.

| `wire` record used by quire-protocol | Successor | Disposition |
| --- | --- | --- |
| `Package` | `CheckedPackageV2` | Successor, consumed by name: the argument of every view function; the typed link view over it is quire-protocol's |
| `Handle` (index into a table) | `CheckedNodeId` | Successor, consumed by name: the id every view and relation returns |
| `Declaration`, `Body::Protocol { .. }` | the `protocol_clause`, `role`, `channel`, `queue`, `obligation`, `control` and `compensation` nodes and their arguments (FR-383-AC-1 to AC-8), with the declaration's identity triple | Successor, read by `protocol_view` and `declaration` |
| `ControlOperation`, `TemporalOperation`, `ValueOperation` | the `quire.op.protocol.control`, `quire.op.temporal.*` and FR-093 value operation identities of the applications | Successor, read by `control_operation` and `operation_identity` |
| `Binder` (and its level) | the binder node and level of FR-383-AC-10 | Successor, read by `binder` |
| `Locus` | the package source-map entry of the node | Successor, read by `locus` |
| `Type`, `Value`, `Number` | `semantic_type` nodes, literal and reference terms; numbers in the FR-322 encoding (the FR-038 codec is not used by the checked package, FR-511-AC-3) | Successor, consumed by name through `CheckedPackageV2::term`; no QSL view |
| `Delivery` | the channel node's delivery text: a policy word or, for a declared interval, FR-509's `[lo,hi]` | Successor, read by `delivery` |
| `EdgeKind`, `CausalEdge` | `CausalRelation` above | Successor, read by `causal_relation` |
| `ActivationMapping` | the FR-449 activation rows | Successor, read by `protocol_view` (`activation`) |
| `TemporalBinding`, `ClockConfiguration` | the Contract IR types of the same names | Successor, read by `temporal_binding` |
| `Model` | the `model` node and the clause's `owner_model` (FR-383-AC-26) | Successor, read by `owner_model` |
| `Dependency` | the lock `dependency_selections` | Successor, read by `dependency` |
| `ArtifactRef`, `ArtifactKind`, `Revision`, `Language`, `Definition` | `CheckedSourceRef`, `CheckedArtifactRef` and `CheckedSelection` (`{ authority, identity }` only) | The `digest`, `revision` and kind fields end with QSL-1270; the `Definition` table ends because definitions are lock selections. `source` reads a `CheckedSourceRef` by identity and `selection` reads a `CheckedArtifactRef` by selection role |
| `BindingKind::Population` | the population declaration node of the model (FR-106, FR-513-AC-14) | Successor, consumed by node id; no QSL view |
| `BindingKind::Clock` | the `temporal_bindings` row | Successor, read by `temporal_binding` |
| `BindingKind::Window` | none: a window is an evaluation input (FR-049-AC-5, FR-516), not a package member | No QSL view; keep the `wire.rs` spelling until QSL-1506 (FR-513's row) |
| `ExportRef` | none: exports are the domain package document's declarations, selected by node id | Ended; no consumer use after QSL-1549 |

### Required of quire-protocol (not QSL code; ticket QSL-1549)

1. Read the handoff with `read_checked_package`, evidence assembled from the
   handoff's files, then `Selection::check` (FR-517); no second reader.
2. A typed link view over `CheckedPackageV2` nodes that replaces `wire::Package`
   in `src/link/mod.rs`, importing `causal_relation` and `protocol_view` and
   mirroring neither, and reading a compensation's retry bound only through
   `attempts_maximum`, never from the compensation node body.
3. Its own refusals stay its own: zero-progress iteration, duplicate role
   authorities, conflicting channel premises, and the others its module
   documentation lists as "this module refuses, because A does not".

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-519-AC-1 | For **R**'s protocol, `causal_relation` returns the (source port, target port) pairs of the checked clause's compiled edge relation, back edges of every `repeat` included, strictly ascending and duplicate-free, and the relation derived from the in-process package equals the one derived from the package read back. | Test (TC-1314) |
| FR-519-AC-2 | `qsl_package::protocol_view::causal_relation` is the only function in `qsl-*/src` and `src` that derives a causal relation from the nodes of a `CheckedPackageV2`; the other derivations, the checker's `compile` over source forms and the expansion `edges` over `wire` controls, start from their own inputs. The verification is a test that scans the production sources of those directories for the signatures that take a `CheckedPackageV2` or its nodes and return a causal relation, and asserts that exactly one exists. | Test (TC-1314) |
| FR-519-AC-3 | `protocol_view` of **R**'s protocol lists the clause, each role, channel, queue, control (layout order), the `Full` and `Partial` compensations, the required temporal clause and the activation rows, each equal to the node set the in-process checked clause rebuilds (FR-383-AC-19). | Test (TC-1314) |
| FR-519-AC-4 | A node id that is absent, or that names a non-protocol node, refuses `UnknownProtocol`; a `protocol_reference` carrier refuses `NotAProtocolClause`; neither returns a partial view or relation. | Test (TC-1314) |
| FR-519-AC-5 | For each record the map marks read by a QSL view function, the view function over **R** returns the node, argument or lock member the map names, equal to the value the checked clause holds in memory. | Test (TC-1314) |
| FR-519-AC-6 | QSL's resolved dependency graph holds no dependency on quire-protocol (`arch-lint direction`, FR-059), and no function of `protocol_view` or `checked_views` reads checked-package bytes (the reader is IR's). | Test (TC-1314) |
| FR-519-AC-7 | **RETIRED (a tracker obligation, not a property of the code).** The cross-repository ticket for quire-protocol's link layer, its relations and its three obligations are held by the tracker, and the three obligations remain stated in "Required of quire-protocol" above and in Dependencies; no repository symbol exists for an inspection to read, so the criterion could not clear in the matrix. | Test (TC-1314), retired |
| FR-519-AC-8 | For each view function that takes a node, a lock or a source map, a copy of that value with the member the view reads removed is refused with a `ViewRefusal` that names the member and returns no partial value. An admitted package never lacks a member, so the removal is made on the value the view reads and not on an admitted package. | Test (TC-1314) |
| FR-519-AC-9 | No QSL export names a record the map marks Ended after QSL-838; a compile-fail use of each of `ExportRef`, `ArtifactKind`, `Revision`, `Language` and `Definition` through `qsl_package` fails to resolve. | Test (TC-1314) |
| FR-519-AC-10 | Each of `protocol_view`, `causal_relation` and `binder` takes `&CheckedPackageV2`, and every node reference a view or relation returns is a `CheckedNodeId`; a test that passes a `CheckedPackageV2` and a `CheckedNodeId` to each compiles. | Test (TC-1314) |
| FR-519-AC-11 | For **R**, the `semantic_type` nodes and the literal and reference terms that stand for `Type`, `Value` and `Number` records are read through `CheckedPackageV2::term` and equal the values the checked clause holds in memory, with numbers in the FR-322 encoding. | Test (TC-1314) |
| FR-519-AC-12 | For the model of TC-1054 (TC-1046's fixture with the populations `orders` and `people`; **R** of FR-517 declares no population), the population declaration node of the emitted package, selected from the `CheckedPackageV2` by its node id, equals the node the checked clause holds in memory. | Test (TC-1314) |
| FR-519-AC-13 | `qsl_package::protocol_view` exports no item for a window binding; a compile-fail `use` of `qsl_package::protocol_view::Window` fails to resolve. | Test (TC-1314) |
| FR-519-AC-14 | For **R** of FR-517, `attempts_maximum` returns 3 for the `Full` compensation and 3 for the `Partial` compensation, each equal to the count the checked clause holds in memory; a compensation whose authored count is 1 returns 1 and one whose count is 18446744073709551615 returns that value. | Test (TC-1314) |
| FR-519-AC-15 | With `maximum` removed from the `attempts` group of the compensation node, `attempts_maximum` refuses with `ViewRefusal::MissingMember` naming `attempts.maximum` and returns no value. An admitted package never lacks the member (FR-383-AC-8), so the removal is made on a copy of the node the function reads and not on an admitted package. | Test (TC-1314) |
| FR-519-AC-16 | For an id absent from the package, or naming a role, a control or any other node that is not a compensation, `attempts_maximum` refuses with `ViewRefusal::NotACompensation` naming the id and returns no value. | Test (TC-1314) |
| FR-519-AC-17 | With a `maximum` member that is not an integer, or is zero (FR-381-AC-7 admits only a count of at least one), `attempts_maximum` refuses with `ViewRefusal::MalformedMember` naming `attempts.maximum` and returns no value; the malformed value is set on a copy of the node the function reads, as in FR-519-AC-15. | Test (TC-1314) |

## Dependencies

- **Upstream:** FR-383, FR-384, FR-218, FR-449, FR-509, FR-511, FR-515, QSL-1270. FR-519-AC-7's tracker obligation is retired as a criterion; the link layer's ticket QSL-1549 carries it.
- **Downstream:** quire-protocol `src/link/mod.rs` (QSL-1549).
