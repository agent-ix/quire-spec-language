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
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-381
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: references
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
clause and reads them through its I2 reader. It also states the S3 check
of an `on each` activation's `when` guard, which S4 lowers into the node.

## Use case

A verification operator compiles a unit with a liveness claim under weak
fairness. The emitted package carries the clause as a temporal node that
any FR-370 reader admits, so the claim's obligation identity, its fairness
set and its intervals are the same for QSL, the model checker and every
backend that reads the package. They also write a response claim that
starts at every state where a counter reaches 2, `on each (q: M::Counter)
when (c.value >= 2)`: the package carries the activation's trigger and
guard, so the checker reads the guard from the package, not from the
request.

## Semantic authority and boundary

QSpec FR-370 owns the node's body, the `quire.op.temporal.*` catalog
entries, the `temporal_interval` and `fairness` member kinds, the placement
and profile-fit rules and the reader order (References). This requirement
specifies what QSL's emitter writes from its checked clause and what its I2
reader returns.

## Inputs

- A checked temporal clause (FR-123): its profile selection, `over`
  parameter, clock role, activation, captures, resolved fairness set,
  property form and checked formula. For an `on each (q: T) [when (g)]`
  activation, the parameter `q` with its resolved type `T` and, when `g` is
  written, the typed guard (§ "`on each` activation at S3").
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

### `on each` activation at S3

An `on each (q: T) [when (g)]` activation, written over an `over (c: U)`
parameter, names the clause's trigger binder `q` and, optionally, its guard
`g`. Over a model subject every position the clause reads is a candidate
activation and `c` already names the instance's object at that position, so
QSL gives `q` no value of its own: `q` is the trigger binder FR-370's
`trigger` member references, and the guard reads the activation position
through `c`.

- S3 SHALL resolve `T` as it resolves every parameter type of the clause
  (`TemporalCause::ParameterType`, with the type-form fault's code and tag:
  `missing_declaration`/`missing-name` for a name that resolves to nothing).
  S3 adds no rule relating `T` to `U`.
- S3 SHALL type `g` as it types a `holds` condition of the clause: with the
  `over` parameter bound at slot 0 and nothing else of the clause in scope,
  no `self`, each field read through `c` a model read of the activation
  position's current state, and the guard a population position (FR-391),
  so the populations it names join the clause's named populations. Its
  nodes are located at the guard's own expression tree.
- The trigger binder is in the scope of no guard and no `holds` condition:
  S3 SHALL refuse a read of `q` in `g` with `missing_declaration`/
  `missing-name` at the read, the cause an unresolved name has.
- S3 SHALL refuse a guard whose type is not `Boolean` with `ill_typed`/
  `non-boolean-root` at the guard, as it refuses such a `holds` condition
  and such a protocol activation guard (FR-381).
- S3 SHALL check the guard's definedness as a `holds` condition's (FR-104
  definedness over the current-state observation) and SHALL refuse an
  undischarged obligation with `undefined_expression` and FR-104's tag
  (`unproved-range`, `unproved-nonzero`, `unproved-presence`) at the
  operation that carries it. Two undefined readings remain possible at run
  time, and neither is an S3 obligation: a read through `c` at a key whose
  object does not conform to `U`, or at which no object is held, which is
  FR-125's `absent-key` undefined value; and a kernel undefined value of an
  operation S3 types with no definedness law, such as an integer `mod` by a
  divisor that may be zero (`division-by-zero`, as for a `holds` condition,
  FR-125-AC-6). Either ends the run with `kind: UndefinedEvaluation` located
  at the activation (FR-126), never a false guard.
- S3 SHALL type the guard before the clause's `holds` conditions, the
  activation preceding the body in source, so a clause with a defect in both
  refuses at the guard.
- An absent guard is the guard that is always true, and S3 SHALL record it
  as absent: `on each (q: T)` checks to `Activation::Each { guarded: false
  }` with no guard, and `on each (q: T) when (true)` to `Activation::Each {
  guarded: true }` with the guard `true`. They are different checked
  clauses, as the protocol activation forms are (FR-381 "Activation"):
  FR-338 keys the first with activation `"each"` and the second with
  `"each-when"`.
- The checked clause SHALL keep the typed guard, with its evaluation slot
  count, beside its `holds` conditions, so the letter seam (FR-125) and the
  certificate checkers (FR-338, FR-339) read the guard from the checked
  package.
- These are existing causes; this requirement adds no refusal cause and no
  wire-visible field.

### `on each` activation at S4

- The emitter SHALL write the activation argument of an `on origin` clause
  as the aggregate that binds `kind` to the text `origin` alone.
- The emitter SHALL write the activation argument of an `on each (q: T)`
  clause as the aggregate that binds `kind` to the text `each` and
  `trigger` to a reference to the `value`/`parameter` node of `q` (name
  `q`, level 1, `semantic_type` `T`'s type node), as FR-447 writes a hybrid
  claim's trigger and FR-383 a protocol's. The `over` parameter stays at
  level 0.
- When the clause writes `when (g)`, the emitter SHALL also bind `guard` to
  `g`'s term, lowered as FR-093 lowers a Boolean expression and as a `holds`
  condition is lowered: a read of `c` is a reference to the `over`
  parameter's node. With no `when`, the aggregate binds no `guard`
  (QSpec FR-370: absent otherwise).
- The guard and the trigger node enter the clause node's content identity
  by FR-322's application-node rule, so `on each (q: T)` with no guard,
  `on each (q: T) when (true)` and `on origin` emit three different clause
  keys and `package_id`s, a different guard or a renamed trigger binder
  emits a different clause key, and reformatting emits the same bytes, as
  FR-383-AC-2 and FR-383-AC-14 state for a protocol.
- The emitted clause's identity triple and `over` node are those of
  "Outputs"; the trigger node is not the `over` node.
- The I2 reader SHALL admit an emitted `each` clause node. QSpec FR-370
  states no reader refusal for the activation argument, so this requirement
  adds none.

### Clauses S4 does not emit

- A checked clause that declares a capture, whose fairness constraint
  resolves to a protocol target (FR-212), or that is checked under the timed
  profile (FR-234) SHALL be refused `unsupported_construct`/
  `not-yet-implemented` (`TemporalCause::NotEmitted`, or
  `ProtocolFairnessNotEmitted` at the protocol-target constraint) and is not
  compiled into a package, since S4 writes no capture, no protocol fairness
  member and no timed interval. An `on each` activation, guarded or not, is
  not a reason to refuse a clause `NotEmitted`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-337-AC-1 | Compiling ADR-018 §6's example unit emits the clause `ReachesTwo` as a `temporal`/`temporal_clause` node whose body is a `quire.op.temporal.clause` application with one `temporal_profile` law naming `quire.temporal.infinite-trace/v1`, a `null` member and the arguments over (a reference to `c`'s parameter node), clock `"model-steps"`, activation `origin`, an empty captures aggregate, one `quire.op.temporal.fair` application (`weak`, `each`, `attemptUpdate`) and the formula `always(eventually(holds(...)))` with `null` intervals; the package's I2 reader admits it. | Test (TC-524) |
| FR-337-AC-2 | The clause written with `fair weak attemptUpdate` and with `fair weak whole attemptUpdate` emits byte-identical fairness applications and equal node keys; changing `each` to `whole`, or the formula's `eventually` to `eventually[0,5]`, changes the node key and the `package_id`. | Test (TC-524) |
| FR-337-AC-3 | `eventually[0,5] holds(c.versionNumber = 2)` under event-position false-extension emits `quire.op.temporal.eventually` with member `temporal_interval` `{lower: "0", upper: "5"}` and an empty fairness argument; strong previous emits `quire.op.temporal.once` with `{lower: "1", upper: "1"}`; under infinite-trace `eventually[3,*] holds(c.versionNumber = 2)` emits `{lower: "3", upper: null}`. | Test (TC-524) |
| FR-337-AC-4 | Fed to QSL's I2 reader, AC-1's node with its arguments reordered fails the schema; with a `null` interval under a bounded profile it refuses `invalid_package`/`operation-member-mismatch`; with a fairness member naming an absent operation it refuses `missing_declaration`/`missing-name`; with a `quire.op.temporal.fair` application inside the formula it refuses `ill_typed`/`operator-ineligible`, each at FR-370's locus. | Test (TC-524) |
| FR-337-AC-5 | Compiling ADR-018 §6's example unit, the I2 reader returns for `ReachesTwo` the triple `[node, "claim", ordinal]` with `node` equal to the emitted node's NodeId and the `over` node equal to `c`'s parameter node; resolving that triple in the recompiled package finds the clause. Two structurally identical clause declarations share one node and the reader returns two triples with distinct ordinals, each resolving to its own declaration. A temporal clause written with no `over` parameter refuses `invalid_syntax`/`unexpected-token` at the token that stands where `over` is required, with no node. A triple with an unknown NodeId, a non-temporal node's NodeId, role `invariant` or another ordinal returns `clause-root-unresolved` at that triple. | Test (TC-1079) |
| FR-337-AC-6 | Over FR-124-AC-1's `Counter` unit (`over (c: M::Counter)`), under infinite-trace, `always holds(c.value <= 1)` `on each (q: M::Counter) when (c.value >= 3)` compiles with no `NotEmitted` refusal and emits a `temporal`/`temporal_clause` node whose activation argument binds `kind` `each`, `trigger` a reference to the `value`/`parameter` node named `q` at level 1 typed by `Counter`'s type node, and `guard` the term of `c.value >= 3` whose field read references `c`'s parameter node at level 0; the same clause `on each (q: M::Counter)` binds `kind` and `trigger` and no `guard`, `when (true)` binds `guard` the lowered term of `true`, and `on origin` binds `kind` `origin` alone. The four give four different clause keys and `package_id`s; `when (c.value >= 2)` and the binder renamed `r` each change the clause key; reformatting changes neither. QSL's I2 reader admits each package and returns each clause's triple and `over` node. | Test (TC-1265) |
| FR-337-AC-7 | Over the same unit, a refused clause gives no package: `when (c.value)` refuses `ill_typed`/`non-boolean-root` at the guard; `when (q.value = 1)` refuses `missing_declaration`/`missing-name` at the read of `q`; `when (c.value = c.value + 1)` refuses `undefined_expression`/`unproved-range` at `c.value + 1`, as FR-104-AC-12 refuses the unguarded sum; `when (6 mod (2 - c.value) = 0)` checks, its divisor having no S3 law; `on each (q: M::Absent)` refuses `missing_declaration`/`missing-name` at `Absent`; a clause whose guard is `c.value` and whose `holds` condition is non-Boolean refuses once, at the guard. An `on each (q: M::Counter)` clause whose body begins `capture n: Int[0, 3] = c.value;` still refuses `unsupported_construct`/`not-yet-implemented` (`NotEmitted`) at the clause. | Test (TC-1265) |

## Dependencies

- ADR-011 §6.1 layer 4 `package` and §2.1 E4; ADR-018 QS-11, FA-6, IV-1.
- [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (the checked clause), [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md)
  (lowering a Boolean operand), FR-092 (node keys),
  [FR-104](FR-104-check-state-clauses.md) (the guard's definedness),
  [FR-381](FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md)
  and [FR-383](FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md) (the
  protocol activation's check and emission, the analogue of the `on each`
  rules), FR-447 (a hybrid claim's trigger node at level 1),
  [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  (the `over` binding and the instances of a clause),
  [FR-338](FR-338-check-an-en-1-closure-certificate.md) (the clause root
  that consumes the triple), ADR-013 O-07 (occurrence ordinals), QSpec
  FR-322 (NodeId).

## References

- QSpec FR-370 (checked-package temporal clause body and temporal operation
  identities), FR-322 (the artifact and its reader order): Linear STD-99 and
  STD-131.
- QSpec FR-370-AC-13 owns the rule that a temporal clause always names an
  `over` parameter, which FR-337-AC-5 and TC-1079 step 3 apply.
- Owning ticket: Linear QSL-366. The `on each` activation rules: Linear
  QSL-926; the letter seam's guard read: QSL-1037.
