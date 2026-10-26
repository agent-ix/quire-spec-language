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
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-218
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
of an `on each` activation's `when` guard and of the clause's captures, which
S4 lowers into the node, and how the engine observes a capture.

## Use case

A verification operator compiles a unit with a liveness claim under weak
fairness. The emitted package carries the clause as a temporal node that
any FR-370 reader admits, so the claim's obligation identity, its fairness
set and its intervals are the same for QSL, the model checker and every
backend that reads the package. They also write a response claim that
starts at every state where a counter reaches 2, `on each (q: M::Counter)
when (c.value >= 2)`: the package carries the activation's trigger and
guard, so the checker reads the guard from the package, not from the
request. They also fix the counter's value at the activation, `capture n:
Int[0, 3] = c.value;`, so a later reading cannot replace it: the package
carries the capture's initializer and the checker reads it from the package
too.

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
activation, and `q` names the activation's object: the instance's object at
the activation position, the object `c` names there. `q` is an alias of
that object that the guard may read, as a hybrid claim's guard reads its
`on each` parameter (FR-447).

- S3 SHALL resolve `T` as it resolves every parameter type of the clause
  (`TemporalCause::ParameterType`, with the type-form fault's code and tag:
  `missing_declaration`/`missing-name` for a name that resolves to nothing).
- S3 SHALL refuse an `on each` parameter whose resolved type `T` is not
  `U`, the `over` parameter's resolved type, with `ill_typed`/
  `type-mismatch` at `T`'s type form, as FR-447 refuses a hybrid claim's
  `on each` parameter of another model (FR-447-AC-9).
- S3 SHALL refuse an `on each` parameter named as the `over` parameter is
  with `conflicting_declaration`/`duplicate-declaration` at the binder's
  name, naming both declarations, as FR-447 refuses a repeated capture name.
- S3 SHALL type `g` as it types a `holds` condition of the clause, with the
  `over` parameter bound at slot 0 and the trigger binder `q` at slot 1,
  both of type `U`, and nothing else of the clause in scope: no `self`,
  each field read through `c` or `q` a model read of the activation
  position's current state, and the guard a population position (FR-391),
  so the populations it names join the clause's named populations. Its
  nodes are located at the guard's own expression tree.
- The trigger binder SHALL be in the scope of the guard alone: S3 SHALL
  refuse a read of `q` in a `holds` condition with `missing_declaration`/
  `missing-name` at the read, the cause an unresolved name has.
- S3 SHALL refuse a guard whose type is not `Boolean` with `ill_typed`/
  `non-boolean-root` at the guard, as it refuses such a `holds` condition
  and such a protocol activation guard (FR-381).
- S3 SHALL check the guard's definedness as a `holds` condition's (FR-104
  definedness over the current-state observation) and SHALL refuse an
  undischarged obligation with `undefined_expression` and FR-104's tag
  (`unproved-range`, `unproved-nonzero`, `unproved-presence`) at the
  operation that carries it. Two undefined readings remain possible at run
  time, and neither is an S3 obligation: a read through `c` or `q` at a key
  whose object does not conform to `U`, or at which no object is held,
  which is FR-125's `absent-key` undefined value; and a kernel undefined
  value of an operation S3 types with no definedness law, such as an
  integer `mod` by a divisor that may be zero (`division-by-zero`, as for a
  `holds` condition, FR-125-AC-6). Either ends the run with
  `kind: UndefinedEvaluation` located at the activation (FR-126), never a
  false guard.
- S3 SHALL check the activation, the binder's type and name and then the
  guard, before the clause's `holds` conditions, the activation preceding
  the body in source, so a clause with a defect in both refuses at the
  activation.
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
- Every refusal above uses an existing code and tag; this requirement adds
  no refusal cause and no wire-visible field. The `T`-is-not-`U` and
  repeated-name refusals are new sites of the existing `ill_typed`/
  `type-mismatch` and `conflicting_declaration`/`duplicate-declaration`
  causes.

### `on each` activation at S4

- The emitter SHALL write the activation argument of an `on origin` clause
  as the aggregate that binds `kind` to the text `origin` alone.
- The emitter SHALL write the activation argument of an `on each (q: T)`
  clause as the aggregate that binds `kind` to the text `each` and
  `trigger` to a reference to the `value`/`parameter` node of `q` (name
  `q`, level 1, `semantic_type` `T`'s type node, which is `U`'s), as FR-447
  writes a hybrid claim's trigger and FR-383 a protocol's. The `over`
  parameter stays at level 0.
- The emitter SHALL record the trigger node's one `expression` occurrence,
  and a `type` occurrence of its type node, at the binder's name, as it
  records the `over` parameter node's at the `over` parameter's name.
- When the clause writes `when (g)`, the emitter SHALL also bind `guard` to
  `g`'s term, lowered as FR-093 lowers a Boolean expression and as a `holds`
  condition is lowered: a read of `c` is a reference to the `over`
  parameter's node, and a read of `q` a reference to the level-1 trigger
  node, as FR-447 lowers `ActivationVariable`. With no `when`, the aggregate
  binds no `guard` (QSpec FR-370: absent otherwise).
- The emitter SHALL key the clause node over the guard and the trigger node
  by FR-322's application-node rule, so `on each (q: T)` with no guard,
  `on each (q: T) when (true)` and `on origin` emit three different clause
  keys and `package_id`s, a different guard or a renamed trigger binder
  emits a different clause key, and reformatting emits the same bytes, as
  FR-383-AC-2 and FR-383-AC-14 state for a protocol.
- The I2 reader SHALL return for an emitted `each` clause the identity
  triple and `over` node of "Outputs", never the trigger node as the `over`
  node.
- The I2 reader SHALL admit an emitted `each` clause node. QSpec FR-370
  states no reader refusal for the activation argument, so this requirement
  adds none.

### Captures at S3

A capture `capture n: T = e;` fixes the value of `e` at the position where an
instance of the clause activates and keeps it for that activation (QSpec
FR-093: immutable, evaluated once at the activation anchor). S3 checks it as
FR-381 checks a protocol capture and keeps it.

- S3 SHALL resolve `T` as it resolves every parameter type of the clause
  (`TemporalCause::ParameterType`: `missing_declaration`/`missing-name` at
  the type form for a name that resolves to nothing).
- S3 SHALL type `e` as it types a `holds` condition of the clause (the
  current-state observation at the activation position; each field read
  through `c`, `q` or an earlier capture of object type a model read; the
  initializer a population position, FR-391, so its populations join the
  clause's named populations), with the `over` parameter `c` at slot 0, the
  trigger binder `q` at slot 1 when the activation is `on each`, and the
  captures written earlier at the following slots in source order (the first
  capture is slot 2 under `on each` and slot 1 under `on origin`, where there
  is no `q`), and nothing else in scope: no `self`. Its nodes are located at the
  initializer's own expression tree.
- S3 SHALL refuse an initializer whose type is not the declared type `T`
  with `ill_typed`/`type-mismatch` at the initializer, retaining `capture`,
  `expected` and `actual`, as FR-381 refuses a protocol capture.
- S3 SHALL check the initializer's definedness as FR-381 checks a protocol
  capture's (FR-104 definedness over the current-state observation) and SHALL
  refuse an undischarged obligation with `undefined_expression` and FR-104's
  tag at the operation that carries it. A kernel undefined value of an
  operation S3 types with no definedness law, and a read at a key that holds
  no object of the required type (`absent-key`), remain run-time undefined
  readings (§ "Capture observation").
- A capture is in the scope of the captures written after it and of the
  `holds` conditions of the formula. It is not in the scope of the
  activation's `when` guard, which decides whether the activation exists
  before any capture is fixed, and not in the scope of an earlier capture's
  initializer or its own. S3 SHALL refuse a read of a capture where it is not
  in scope (a later capture, itself, or the guard) with `missing_declaration`/
  `missing-name` at the read, the cause an unresolved name has.
- S3 SHALL refuse a capture whose name is the `over` parameter's, the
  trigger binder's or an earlier capture's with `conflicting_declaration`/
  `duplicate-declaration` at the capture's name, naming both declarations,
  as it refuses a repeated trigger binder.
- S3 SHALL check the activation (binder, type and name, then guard) first,
  then the captures in source order, then the `holds` conditions, so a clause
  with a defect in more than one refuses at the earliest in that order.
- The checked clause SHALL keep its captures in source order, each as name,
  declared type, typed initializer and evaluation slot count (the number of
  binders in scope for the initializer, `c`, `q` when `on each` and the
  earlier captures, that is `1 + e + j` for the zero-based `j`th capture,
  `e` being 1 for `on each` and 0 for `on origin`; a `holds` condition's
  count is `1 + e + k` for the `k` captures), beside the
  guard and the `holds` conditions, so S4, the letter seam (FR-125) and the
  certificate checkers (FR-338, FR-339) read them from the checked package.
  Each `holds` condition's slot names SHALL list the captures after `c` and
  `q`.
- The clause's extent already ranges over each capture's type position
  (`temporal-satisfaction`); this requirement does not change that record.
- Every refusal above uses an existing code and tag; this requirement adds no
  refusal cause and no wire-visible field.

### Captures at S4

- The emitter SHALL write the captures argument as the aggregate with one
  binding per capture, in source order, each named by the capture and holding
  the initializer's term, lowered as FR-093 lowers a Boolean or value
  expression and as a `holds` condition is lowered: a read of `c` is a
  reference to the `over` parameter's node, a read of `q` a reference to the
  level-1 trigger node and a read of an earlier capture a reference to that
  capture's node. A clause with no capture keeps the empty aggregate.
- The emitter SHALL mint, for each capture, the capture's own
  `value`/`parameter` node, named by the capture, with `semantic_type` the
  type node of its declared type `T`, at level `1 + e + j`, where `e` is 1 for
  an `on each` activation and 0 for `on origin`, and `j` is the capture's
  zero-based position in source order (the trigger is level 1 and each
  capture the next, as FR-383 numbers a protocol's, never by source offset).
  The node is keyed by FR-322's parameter-node rule over its name, level and
  type, so the node does not depend on the initializer: the initializer's term
  is in the captures aggregate, and an `holds` read of the capture is a
  reference to this node. The emitter SHALL record the node's one
  `expression` occurrence, and a `type` occurrence of its type node, at the
  capture's name.
- The emitter SHALL lower a `holds` condition's read of a capture as a
  reference to the capture's node.
- The emitter SHALL key the clause node over the captures aggregate by
  FR-322's application-node rule, so a clause with a capture and the same
  clause without it, a renamed capture, a changed initializer, a changed
  declared type, two captures in the other order and an added capture each
  emit a different clause key and `package_id`, and reformatting emits the
  same bytes. The emitted capture nodes enter the clause node's dependencies
  as references of its body. A capture does not change the clause's `claim`
  occurrence, identity triple or `over` node. A `requires temporal Due;`
  requirement (FR-218-AC-16) binds `Due`'s position among the unit's temporal
  clauses and is carried by the emitted clause node of that position (the
  protocol's `requirements` set holds the temporal clause nodes, FR-383), so
  its key is that clause node's key: it is the same for the clause emitted
  twice and different from the key of the same clause without its captures.
- The I2 reader SHALL admit an emitted clause with captures and return its
  identity triple and `over` node of "Outputs", never a trigger or capture
  node as the `over` node. QSpec FR-370 states no reader refusal for the
  captures argument beyond its shape (an aggregate of named bindings), so this
  requirement adds none.

### Capture observation

- For an instance of the clause at an activation position, the engine
  (FR-126) SHALL read the guard first (FR-125 "The `on each` activation
  guard"); only when the guard is true, or the activation is `on origin`
  (position 0) or has no guard, SHALL it evaluate the clause's captures, in
  source order, once, through the clause evaluator over that position's
  observation, with the `over` parameter bound as FR-125 "Instancing" binds
  it, `q` bound to the same reference and each earlier capture bound to its
  value. Where the guard is false no capture is evaluated and no capture can
  be undefined.
- A capture's value SHALL be fixed for that activation: a later position does
  not re-evaluate it, and a capture of one activation never satisfies another
  activation's condition.
- A capture whose evaluation is undefined (a read at a key whose object does
  not conform to the required type or at which no object is held,
  `absent-key`, or a kernel undefined value such as `division-by-zero`) SHALL
  end the run with `kind: UndefinedEvaluation` located at the capture
  (FR-126), never a default value and never a false capture. A capture
  evaluation that gives no reading SHALL stop the check as a guard
  evaluation that gives none does (FR-125), and one that returns `Incomplete`
  naming an exhausted counter stops it with `ResourceExhausted` (FR-126).
- A capture no `holds` condition reads changes no verdict except by the
  undefined reading above: the clause with and without it returns the same
  verdict and counterexample where every capture is defined.
- A clause some `holds` condition of which reads a capture is, until
  QSL-1092, refused by the engine with
  `ModelCheckRefusal::CaptureRead`, naming the clause and the capture, and
  returns no verdict, as a guarded TP-4 clause was refused before QSL-1037
  (FR-338's automaton-state keys carry no capture vector). It still checks at
  S3 and is emitted at S4. QSL-1092 decides such a clause over the
  product of the instance's state, the activation and the capture vector.
- The capture's footprint (FR-156 "Atom footprints") SHALL be the locations
  its initializer reads, with the `over` parameter as the receiver of a read
  through `over`, the trigger binder or an earlier capture, joined into the
  atoms' as the guard's is. FR-128's replay and FR-338's and FR-339's checkers
  SHALL read the captures through the same seam, from the recompiled package.

### Clauses S4 does not emit

- A checked clause whose fairness constraint resolves to a protocol target
  (FR-212), or that is checked under the timed profile (FR-234) SHALL be
  refused `unsupported_construct`/`not-yet-implemented`
  (`TemporalCause::NotEmitted`, or `ProtocolFairnessNotEmitted` at the
  protocol-target constraint) and is not compiled into a package, since S4
  writes no protocol fairness member and no timed interval. An `on each`
  activation, guarded or not, and a capture are not reasons to refuse a
  clause `NotEmitted`.
- FR-218-AC-16 and TC-1056 row 23a require a clause with an `on each`
  activation and a capture to check and S4 to mint its node key; they are
  satisfiable now that S4 emits captures.

### Notes for the implementation

- `qsl-analyze/src/model_check/liveness.rs` still returns
  `ModelCheckRefusal::GuardedActivation` for a guarded TP-4 clause (lines
  176 and 283). FR-126 decides such a clause over FR-338's guarded automaton
  and FR-126-AC-16's TP-4 halves require it, so the coding change removes
  that refusal.
- The trait method `ModelLetters::activation` (`pub trait ModelLetters`,
  `qsl-eval/src/model_subject.rs`, lines 178-183) returns
  `Result<AtomReading, Incomplete>`, and so does its production
  implementation `ClauseLetters::activation` (`impl ModelLetters for
  ClauseLetters`, `qsl-eval/src/simulation/model/letters.rs`, lines
  271-278), which today returns `True` unconditionally. FR-125 requires a
  guard evaluation that gives no reading to stop the check as a `holds`
  atom's does, so the trait method's error type widens to `AtomStop` and
  the implementation evaluates the guard.
- `qsl-semantics/src/check/temporal_clause/tests.rs`'s
  `the_package_check_refuses_a_clause_no_emission_writes` expects
  `NotEmitted` for `on each (d: Counter) when (d.level = 1)`; that clause
  now checks and emits.

For QSL-1081 (captures):

- `qsl-semantics/src/check/temporal_clause.rs`'s `is_emitted` stops testing
  `form.captures.is_empty()`, and the checked clause keeps the captures
  beside the guard (`TypedConditions` in `temporal_clause/condition.rs`
  gains the typed initializers; the slot names of every condition list the
  captures after `c` and `q`).
- `qsl-semantics/src/check/lowering/temporal.rs` writes the captures
  aggregate (it writes `GroupTerm::new(Vec::new())` today) and mints the
  capture parameter nodes, with a `TemporalSite::Capture(index)` site for
  their occurrences.
- `ModelLetters` (`qsl-eval/src/model_subject.rs`) gains a capture reading at
  an activation, and `qsl-analyze/src/model_check/liveness.rs` and the
  bounded monitors refuse a clause whose `holds` reads a capture with
  `ModelCheckRefusal::CaptureRead`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-337-AC-1 | Compiling ADR-018 §6's example unit emits the clause `ReachesTwo` as a `temporal`/`temporal_clause` node whose body is a `quire.op.temporal.clause` application with one `temporal_profile` law naming `quire.temporal.infinite-trace/v1`, a `null` member and the arguments over (a reference to `c`'s parameter node), clock `"model-steps"`, activation `origin`, an empty captures aggregate, one `quire.op.temporal.fair` application (`weak`, `each`, `attemptUpdate`) and the formula `always(eventually(holds(...)))` with `null` intervals; the package's I2 reader admits it. | Test (TC-524) |
| FR-337-AC-2 | The clause written with `fair weak attemptUpdate` and with `fair weak whole attemptUpdate` emits byte-identical fairness applications and equal node keys; changing `each` to `whole`, or the formula's `eventually` to `eventually[0,5]`, changes the node key and the `package_id`. | Test (TC-524) |
| FR-337-AC-3 | `eventually[0,5] holds(c.versionNumber = 2)` under event-position false-extension emits `quire.op.temporal.eventually` with member `temporal_interval` `{lower: "0", upper: "5"}` and an empty fairness argument; strong previous emits `quire.op.temporal.once` with `{lower: "1", upper: "1"}`; under infinite-trace `eventually[3,*] holds(c.versionNumber = 2)` emits `{lower: "3", upper: null}`. | Test (TC-524) |
| FR-337-AC-4 | Fed to QSL's I2 reader, AC-1's node with its arguments reordered fails the schema; with a `null` interval under a bounded profile it refuses `invalid_package`/`operation-member-mismatch`; with a fairness member naming an absent operation it refuses `missing_declaration`/`missing-name`; with a `quire.op.temporal.fair` application inside the formula it refuses `ill_typed`/`operator-ineligible`, each at FR-370's locus. | Test (TC-524) |
| FR-337-AC-5 | Compiling ADR-018 §6's example unit, the I2 reader returns for `ReachesTwo` the triple `[node, "claim", ordinal]` with `node` equal to the emitted node's NodeId and the `over` node equal to `c`'s parameter node; resolving that triple in the recompiled package finds the clause. Two structurally identical clause declarations share one node and the reader returns two triples with distinct ordinals, each resolving to its own declaration. A temporal clause written with no `over` parameter refuses `invalid_syntax`/`unexpected-token` at the token that stands where `over` is required, with no node. A triple with an unknown NodeId, a non-temporal node's NodeId, role `invariant` or another ordinal returns `clause-root-unresolved` at that triple. | Test (TC-1079) |
| FR-337-AC-6 | Over FR-124-AC-1's `Counter` unit (`over (c: M::Counter)`), under infinite-trace, `always holds(c.value <= 1)` `on each (q: M::Counter) when (c.value >= 3)` compiles with no `NotEmitted` refusal and emits a `temporal`/`temporal_clause` node whose activation argument binds `kind` `each`, `trigger` a reference to the `value`/`parameter` node named `q` at level 1 typed by `Counter`'s type node, and `guard` the term of `c.value >= 3` whose field read references `c`'s parameter node at level 0; the same clause `on each (q: M::Counter)` binds `kind` and `trigger` and no `guard`, `when (true)` binds `guard` the lowered term of `true`, and `on origin` binds `kind` `origin` alone. The four give four different clause keys and `package_id`s; `when (c.value >= 2)` and the binder renamed `r` each change the clause key; reformatting changes neither. The trigger node's `expression` occurrence and its type node's `type` occurrence are located at the binder's name `q`. QSL's I2 reader admits each package and returns each clause's triple and `over` node. | Test (TC-1265) |
| FR-337-AC-7 | Over the same unit: `when (q.value = 1)` checks, and its guard term's field read references the level-1 trigger node; a refused clause gives no package: `when (c.value)` refuses `ill_typed`/`non-boolean-root` at the guard; `holds(q.value <= 1)` in the body refuses `missing_declaration`/`missing-name` at the read of `q`; `when (c.value = c.value + 1)` refuses `undefined_expression`/`unproved-range` at `c.value + 1`, as FR-104-AC-12 refuses the unguarded sum; `when (6 mod (2 - c.value) = 0)` checks, its divisor having no S3 law; `on each (q: M::Absent)` refuses `missing_declaration`/`missing-name` at `Absent`; `on each (q: Integer)` refuses `ill_typed`/`type-mismatch` at `Integer`; `on each (c: M::Counter)` refuses `conflicting_declaration`/`duplicate-declaration` at the binder `c`, naming both declarations; a clause whose guard is `c.value` and whose `holds` condition is non-Boolean refuses once, at the guard. | Test (TC-1265) |
| FR-337-AC-8 | Over the same unit, `always holds(c.value <= 1)` `on each (q: M::Counter) when (c.value >= 1)` with the body capture `n: Int[0, 3] = c.value;` then `m: Int[0, 3] = q.value;` checks, and the checked clause keeps `n` then `m`, each with its declared type, typed initializer and slot count (2 and 3 under `on each`, 1 and 2 under `on origin`; a `holds` condition's count is 4 under `on each` and 3 under `on origin`); `holds(c.value >= n)` and `holds(m = n)` check; the same captures under `on origin` check. Each refuses with no package: `b: Boolean = c.value` `ill_typed`/`type-mismatch` at the initializer, `expected` `Boolean`; `x: M::Absent = c.value` `missing_declaration`/`missing-name` at `Absent`; `n: Int[0, 3] = c.value + 1` `undefined_expression`/`unproved-range` at `c.value + 1`; a capture initializer reading a later capture or itself, and the `when` guard reading a capture, each `missing_declaration`/`missing-name` at the read; a capture named `c`, `q` or as an earlier capture `conflicting_declaration`/`duplicate-declaration` at the capture's name, naming both declarations. A clause whose guard is `c.value` and whose capture initializer is ill typed refuses once, at the guard; one whose capture is ill typed and whose `holds` condition is non-Boolean refuses once, at the capture. | Test (TC-1267) |
| FR-337-AC-9 | Over the same unit, `on each (q: M::Counter) when (c.value >= 1)` with the captures of AC-8 emits a clause node whose captures argument binds `n` to the term of `c.value`, whose field read references `c`'s parameter node at level 0, and `m` to the term of `q.value`, whose read references the level-1 trigger node; the `value`/`parameter` nodes `n` and `m` are at levels 2 and 3 typed by `Int[0, 3]`'s type node, and `holds(c.value >= n)` lowers its read of `n` as a reference to `n`'s node; under `on origin` the nodes are at levels 1 and 2; a clause with no capture binds the empty aggregate. Each capture node's `expression` occurrence and its type node's `type` occurrence are located at the capture's name. QSL's I2 reader admits each package and returns each clause's triple and `over` node, never a trigger or capture node as `over`. | Test (TC-1267) |
| FR-337-AC-10 | The clause of AC-9 and the same clause with the capture `n` renamed `p`, with `n`'s initializer `q.value`, with `n` declared `Int[0, 9]`, with `n` and `m` in the other order, with a third capture added and with no capture each emit a different clause key and `package_id`; reformatting changes neither; `requires temporal Due;` over the clause with the captures (FR-218-AC-16) binds to `Due`'s position among the unit's temporal clauses, and the node the protocol's `requirements` set holds for it is the emitted clause node, so its key equals the clause key of AC-9 and differs from the key over the same clause with no capture. | Test (TC-1267) |
| FR-337-AC-11 | Read through QSL's production letter seam over the `Counter` subject under event-position false-extension, `eventually[0,1] holds(c.value = 3)` `on each (q: M::Counter) when (c.value = 1)` with an added unread `capture n: Int[0, 3] = c.value;` returns `Violated` with the prefix `0 -inc-> 1 -inc-> 2`, as AC-18 of FR-125 returns it without the capture, and with `when (c.value >= 2)` returns `Holds{Exhaustive}`; under infinite-trace `always holds(true)` `on each (q: M::Counter) when (true)` with `capture z: Boolean = (6 mod (2 - c.value) = 0);` returns `Violated` with `kind: UndefinedEvaluation` at position 2, cause `division-by-zero`, its site naming the capture `z`, and with `when (false)` (S3 refuses `c.value = 7` over `Int[0, 3]`, an unproved range) returns `Holds{Exhaustive}` because no activation evaluates the capture. | Test (TC-1267) |
| FR-337-AC-12 | The clause `eventually[0,1] holds(n < 3 and c.value = n + 1)` under event-position false-extension, `on each (q: M::Counter) when (c.value = 1)` with `capture n: Int[0, 3] = c.value;` checks and S4 emits it, and the engine returns `ModelCheckRefusal::CaptureRead` naming the clause and `n` and no verdict, exploring no state (no state is charged to the meter); the same refusal results for `always holds(n < 3 and c.value <= n + 1)` under infinite-trace `on each`, for a bounded monitor clause `on origin` with `capture n: Int[0, 3] = c.value;` and `always holds(c.value >= n)`, and for a clause reading its second capture only, naming that capture; a clause whose unit has an unbounded root returns `RequiresBound`, not `CaptureRead`; the same clause with the `holds` condition `c.value = 3` returns AC-11's verdict with the capture unread. | Test (TC-1267) |
| FR-337-AC-13 | Over the `Counter` subject with a capture `n: Int[0, 3] = c.value;` no `holds` reads: infinite-trace `always holds(c.value <= 1)` `on each (q: M::Counter) when (c.value >= 3)` is `Violated`, as FR-125-AC-18 returns it, and FR-128 replays its counterexample to `reproduced-with-evaluated-witness`; the unguarded `on each (q: M::Counter)` clause with the same capture is `Holds{Exhaustive}` (a guarded clause carries no closure certificate) and FR-338's checker accepts the `ClosureCertificate`, reading the captures from the recompiled package. For AC-11's `z` clause, FR-338's checker rejects a certificate for a reached state at which `z` is undefined `BadState` at the reached product state, as it rejects an undefined atom. The replay of an undefined capture (FR-128 reproducing `UndefinedEvaluation` at position 2 located at `z`) is deferred to QSL-1096, which builds the replay of an authored clause's undefined evaluation; FR-339's component checker is not built, so its rejection of the same certificate waits on it. | Test (TC-1267) |
| FR-337-AC-14 | The footprint (FR-156 "Atom footprints") of `always holds(true)` `on each (q: M::Counter) when (true)` with `capture n: Int[0, 3] = c.value;` holds the location `Counter::value` with the `over` parameter as its receiver, which the same clause with no capture does not, and FR-126's partial-order run joins it into the atoms'; a capture initializer reading through `q` has the same receiver. | Test (TC-1267) |

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
- QSpec FR-093 (a capture is immutable and evaluated once at the activation
  anchor) and FR-370 item 4 (the captures aggregate: one binding per capture,
  named by the capture, holding its initializer term). FR-370 states no term
  for a `holds` read of a capture; this requirement uses the `value`/`parameter`
  reference FR-447 uses for a hybrid claim's `Capture(j)`.
- QSpec FR-370-AC-13 owns the rule that a temporal clause always names an
  `over` parameter, which FR-337-AC-5 and TC-1079 step 3 apply.
- Owning ticket: Linear QSL-366. The `on each` activation rules are
  specified under Linear QSL-926; their implementation (the S3 guard check,
  the S4 emission and the letter seam's guard read) is owned by QSL-1037.
  Capture emission: specified by QSL-1081 (this requirement); the code half,
  also under QSL-1081, implements it. The engine's reading of a capture a
  `holds` condition reads (the `CaptureRead` residual) is owned by QSL-1092.
