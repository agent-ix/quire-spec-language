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

- One v2 node per temporal clause, its body as FR-370 states, with
  `semantic_type` the package's `scalar_type`/`boolean` node and every
  occurrence of role `claim`.
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

## Dependencies

- ADR-011 §6.1 layer 4 `package` and §2.1 E4; ADR-018 QS-11, FA-6, IV-1.
- [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (the checked clause), [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md)
  (lowering a Boolean operand), FR-092 (node keys).

## References

- QSpec FR-370 (checked-package temporal clause body and temporal operation
  identities), FR-322 (the artifact and its reader order): Linear STD-99 and
  STD-131.
- Owning ticket: Linear QSL-366.
