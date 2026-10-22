---
id: FR-410
title: "Emit a checked state-graph claim as a v2 claim node"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-020
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-022
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-165
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-170
    type: depends_on
---
# FR-410: Emit a checked state-graph claim as a v2 claim node

## Description

The S4 package emitter (ADR-011 layer 4 `package`, edge E4) SHALL emit each
checked state-graph claim (FR-165) as one `quire.checked-semantic-graph/v2`
node with `node_tag: "claim"` and `semantic_form: "state_graph"`, whose body
is the aggregate QSpec FR-475 states, and QSL's I2 reader SHALL read that
node back. QSpec FR-475 owns the body, the predicate nodes, the obligation
identity and the reader's refusals; this requirement states what QSL's
emitter writes from its `CheckedStateGraphClaim` and what its I2 reader
returns.

## Use case

A verification operator compiles a unit holding `possible ReachesTwo` and
`always possible CanStillWin`. The package carries each claim as a
state-graph claim node, so a provider request names each item by
`claim {node, occurrence}`, replay (FR-170) resolves the claim from the
recompiled package, and the settlement (FR-169) binds the same obligation
identity the request named.

## Inputs

- A checked state-graph claim (FR-165): its `StateGraphForm`, its predicates
  and its `over` binding.
- The node keys FR-092 mints and FR-093's lowering of a Boolean expression.

## Outputs

- One v2 node per distinct checked claim, body as QSpec FR-475 states, with
  `semantic_type` the package's `scalar_type`/`boolean` node, its
  `declaration` the claim's qualified name, and every occurrence of role
  `claim`.
- One `expression` node per distinct predicate, its body the predicate's
  Boolean term as FR-093 lowers it.
- One identity triple `[node, "claim", ordinal]` per `claim` occurrence, as
  FR-337 states for a temporal clause.
- From the I2 reader: the claim's form, its predicates and its `over` node,
  or FR-475's refusal with its locus.

## Behavior

- The emitter SHALL write `kind` as `possible` for `Possible`,
  `always-possible` for `AlwaysPossible` and `unique-path` for
  `UniquePath`.
- The emitter SHALL write `over` as a reference to the `over` parameter's
  `value`/`parameter` node when the claim's `over` is `Some`, and as the
  empty aggregate when it is `None`.
- The emitter SHALL write `from` as a reference to the `expression` node of
  `AlwaysPossible.from` or `UniquePath.from`, and as the empty aggregate for
  `Possible`. An `always possible` claim without `from` carries the constant
  `true` predicate FR-165 checks it with, so it emits the node of the same
  claim written `from (true)`.
- The emitter SHALL write `target` as a reference to the `expression` node
  of `Possible.target`, `AlwaysPossible.target` or `UniquePath.to`.
- The emitter SHALL key the node by FR-322's rules over its body, so the form
  and every predicate enter the node id (FR-165-AC-1).
- S4 SHALL NOT refuse a checked state-graph claim as not emitted.
- The I2 reader SHALL admit an emitted state-graph claim node and return its
  form, predicates and `over` node, and SHALL refuse a node that breaks an
  FR-475 rule with FR-475's code at FR-475's locus, in FR-475's reader order.
- If a looked-up claim triple names a NodeId the package does not hold, a
  node that is not a `claim`/`state_graph` node, or an occurrence whose role
  or ordinal differs, then the I2 reader SHALL return the lookup refusal
  `clause-root-unresolved` at the looked-up triple, as FR-337 does for a
  temporal clause.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-410-AC-1 | Compiling ADR-022 §7's units emits `ReachesTwo` as a `claim`/`state_graph` node with `kind` `possible`, `over` a reference to `c`'s parameter node, `from` the empty aggregate and `target` a reference to the `expression` node of `c.versionNumber = 2`; `CanStillWin` with `from (x.phase != Lost)` with `kind` `always-possible` and `from` that predicate's node; `InOneWay` with `kind` `unique-path`. Each node's occurrences have role `claim`, and the I2 reader admits each and returns its form, predicates and `over` node. No checked claim is refused as not emitted. | Test (TC-1140) |
| FR-410-AC-2 | `possible P` and `always possible P` over one `P` emit different node ids and different `package_id`s; `CanStillWin` without `from` and with `from (true)` emit one node id; `possible Anything using v { true }` emits `over` as the empty aggregate. | Test (TC-1140) |
| FR-410-AC-3 | The I2 reader returns for `ReachesTwo` the triple `[node, "claim", ordinal]` with `node` the emitted NodeId, and resolving it in the recompiled package finds the claim; a triple with an unknown NodeId, with a `temporal`/`temporal_clause` node's NodeId, or with another ordinal returns `clause-root-unresolved` at that triple. | Test (TC-1140) |
| FR-410-AC-4 | Fed to QSL's I2 reader, AC-1's `ReachesTwo` node with `kind` `eventually-possible` refuses `invalid_package`/`invalid-value`, and with `target` a reference to `c`'s parameter node refuses `ill_typed`/`operator-ineligible`, each at FR-475's locus. | Test (TC-1140) |

## Dependencies

- ADR-011 §6.1 layer 4 `package` and §2.1 E4; ADR-022 SG-1 to SG-4.
- [FR-165](FR-165-check-state-graph-claims-at-s3.md) (the checked claim and
  its form), [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md)
  (lowering a predicate), FR-092 (node keys),
  [FR-337](FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md)
  (the identity triple and `clause-root-unresolved`),
  [FR-169](FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md)
  and [FR-170](FR-170-replay-state-graph-evidence.md) (the settlement and
  replay that read the node).

## References

- QSpec FR-475 (checked-package state-graph claim body and obligation
  identity), FR-322 (the artifact and its reader order).
- Owning ticket: Linear STD-267; code: QSL-987.
