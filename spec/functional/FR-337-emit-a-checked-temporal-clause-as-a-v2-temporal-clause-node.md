---
id: FR-337
title: "Emit a checked temporal clause as a v2 temporal clause node"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-015
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
---
# FR-337: Emit a checked temporal clause as a v2 temporal clause node

## Description

The S4 package emitter (ADR-011 layer 4 `package`, edge E4) SHALL emit each
checked temporal clause as one `quire.checked-semantic-graph/v2` node with
`node_tag: "temporal"` and `semantic_form: "temporal_clause"`, whose body is
the `quire.op.temporal.clause` application QSpec FR-370 states, and QSL's
I2 reader SHALL read that node back. This is the compiler side of QSpec
FR-370 (ADR-018 QS-11): QSpec owns the body, the operation identities, the
member kinds and the reader's refusals; QSL emits them from its checked
clause and reads them through its I2 reader.

## Use case

A verification operator compiles a unit with a liveness claim under weak
fairness. The emitted package carries the clause as a temporal node that
any FR-370 reader admits, so the claim's obligation identity, its fairness
set and its intervals are the same for QSL, the model checker and every
backend that reads the package.

## Semantic authority and boundary

QSpec FR-370 owns the node's body, the `quire.op.temporal.*` catalog
entries, the `temporal_interval` and `fairness` member kinds, the placement
and profile-fit rules and the reader order (References). This requirement
specifies what QSL's emitter writes from its checked clause and what its I2
reader returns.

## Inputs

- A checked temporal clause (FR-123): its profile selection, `over`
  parameter, clock role, activation, captures, resolved fairness set,
  property form and checked formula.
- The package's lock evidence (`profile_selections`) and the node keys
  FR-092 mints.

## Outputs

- One v2 node per distinct checked clause, its body as FR-370 states, with
  `semantic_type` the package's `scalar_type`/`boolean` node and every
  occurrence of role `claim`.
- One identity triple `[node, role, ordinal]` per `claim` occurrence of the
  clause node: `node` is the emitted node's FR-322 NodeId, `role` is
  `claim`, and `ordinal` is that occurrence's ordinal (O-07). Node ids are
  content-addressed, so two structurally identical clause declarations share
  one node and differ only by occurrence ordinal (FR-088-AC-3); each
  declaration has its own triple. The triple is the clause root of FR-338's
  automaton-state context.
- The clause's `over` node: the NodeId of the parameter node the first
  argument of the `quire.op.temporal.clause` application references. A
  clause always has an `over` node: QSpec FR-370 admits no temporal clause
  without an `over` parameter (FR-370-AC-13), and S1 refuses one with
  `invalid_syntax`/`unexpected-token` at the token that stands where `over` is
  required.
- From the I2 reader: the checked clause's v2 view, or FR-370's refusal
  with its locus.

## Behavior

- The emitter SHALL write the body as one `quire.op.temporal.clause`
  application with one `temporal_profile` law, equal to the lock's
  `profile_selections` row of that role, and the six arguments over, clock,
  activation, captures, fairness and formula, in that order (QSpec FR-370).
- The emitter SHALL write each temporal operator of the formula as an
  application with its own `quire.op.temporal.*` identity, and each `holds`
  atom's Boolean operand as FR-093 lowers a Boolean expression.
- The emitter SHALL write an interval operator's member as
  `temporal_interval` with `{lower, upper}` as decimal strings, an `[a,*]`
  operator's interval as `{lower, upper: null}`, and an unbounded
  operator's interval as `null` (QSpec FR-370).
- The emitter SHALL write one `quire.op.temporal.fair` application per
  member of the resolved fairness set, in fairness-set order, each with the
  resolved `fairness_kind` and `granularity`, so an unmarked constraint and
  its written `weak whole` form emit identical bytes (ADR-018 FA-6).
- The emitter SHALL write strong previous as `quire.op.temporal.once` with
  interval `{lower: "1", upper: "1"}`.
- The emitter SHALL emit, for every clause, a node that carries the
  clause's `over` reference and one `claim` occurrence per declaration, so
  the identity triple and `over` node are derivable from the package alone.
- The emitter SHALL write the `over` argument as a reference to the
  parameter node, never as a value.
- The concrete binding of an `over` parameter comes from the model subject
  a check explores (FR-125), not from the package.
- A consumer resolves a clause root against the package (FR-338 states the
  checker's resolution) and takes the `over` node only from the package;
  it takes neither a clause the package does not hold nor an `over` node
  from its caller.
- The I2 reader SHALL return, for each `claim` occurrence, its identity
  triple and its `over` node.
- If a looked-up clause root names a NodeId the package does not hold, a
  node that is not a `temporal`/`temporal_clause` node, or an occurrence
  whose role or ordinal differs, then the I2 reader SHALL return the lookup
  refusal `clause-root-unresolved` at the looked-up triple. This is a
  QSL lookup result, not an FR-370 package refusal, because the package
  itself is well-formed. Whether it rejects a certificate or refuses a
  request is decided by the caller: FR-338 refuses the request when the
  request's own clause root is unresolved and rejects the certificate when
  a key's context root is.
- The emitter SHALL key the node and its dependencies by FR-322's
  application-node rule, so equal checked clauses emit equal node keys and
  a different interval, fairness set or profile emits a different key.
- The I2 reader SHALL admit an emitted temporal node.
- The I2 reader SHALL refuse a node that breaks an FR-370 rule with
  FR-370's code at FR-370's locus, in FR-370's reader order.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-337-AC-1 | Compiling ADR-018 §6's example unit emits the clause `ReachesTwo` as a `temporal`/`temporal_clause` node whose body is a `quire.op.temporal.clause` application with one `temporal_profile` law naming `quire.temporal.infinite-trace/v1`, a `null` member and the arguments over (a reference to `c`'s parameter node), clock `"model-steps"`, activation `origin`, an empty captures aggregate, one `quire.op.temporal.fair` application (`weak`, `each`, `attemptUpdate`) and the formula `always(eventually(holds(...)))` with `null` intervals; the package's I2 reader admits it. | Test (TC-524) |
| FR-337-AC-2 | The clause written with `fair weak attemptUpdate` and with `fair weak whole attemptUpdate` emits byte-identical fairness applications and equal node keys; changing `each` to `whole`, or the formula's `eventually` to `eventually[0,5]`, changes the node key and the `package_id`. | Test (TC-524) |
| FR-337-AC-3 | `eventually[0,5] holds(c.versionNumber = 2)` under event-position false-extension emits `quire.op.temporal.eventually` with member `temporal_interval` `{lower: "0", upper: "5"}` and an empty fairness argument; strong previous emits `quire.op.temporal.once` with `{lower: "1", upper: "1"}`; under infinite-trace `eventually[3,*] holds(c.versionNumber = 2)` emits `{lower: "3", upper: null}`. | Test (TC-524) |
| FR-337-AC-4 | Fed to QSL's I2 reader, AC-1's node with its arguments reordered fails the schema; with a `null` interval under a bounded profile it refuses `invalid_package`/`operation-member-mismatch`; with a fairness member naming an absent operation it refuses `missing_declaration`/`missing-name`; with a `quire.op.temporal.fair` application inside the formula it refuses `ill_typed`/`operator-ineligible`, each at FR-370's locus. | Test (TC-524) |
| FR-337-AC-5 | Compiling ADR-018 §6's example unit, the I2 reader returns for `ReachesTwo` the triple `[node, "claim", ordinal]` with `node` equal to the emitted node's NodeId and the `over` node equal to `c`'s parameter node; resolving that triple in the recompiled package finds the clause. Two structurally identical clause declarations share one node and the reader returns two triples with distinct ordinals, each resolving to its own declaration. A temporal clause written with no `over` parameter refuses `invalid_syntax`/`unexpected-token` at the token that stands where `over` is required, with no node. A triple with an unknown NodeId, a non-temporal node's NodeId, role `invariant` or another ordinal returns `clause-root-unresolved` at that triple. | Test (TC-1079) |

## Dependencies

- ADR-011 §6.1 layer 4 `package` and §2.1 E4; ADR-018 QS-11, FA-6, IV-1.
- [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (the checked clause), [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md)
  (lowering a Boolean operand), FR-092 (node keys),
  [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  (the `over` binding and the instances of a clause),
  [FR-338](FR-338-check-an-en-1-closure-certificate.md) (the clause root
  that consumes the triple), ADR-013 O-07 (occurrence ordinals), QSpec
  FR-322 (NodeId).

## References

- QSpec FR-370 (checked-package temporal clause body and temporal operation
  identities), FR-322 (the artifact and its reader order): Linear STD-99 and
  STD-131.
- Owning ticket: Linear QSL-366.
