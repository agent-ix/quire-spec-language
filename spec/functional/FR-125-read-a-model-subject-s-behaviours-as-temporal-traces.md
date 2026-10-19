---
id: FR-125
title: "Read a model subject's behaviours as temporal traces"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-015
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-361
    type: depends_on
---
# FR-125: Read a model subject's behaviours as temporal traces

## Description

QSL SHALL read each behaviour of a temporal claim's **model subject** as a
trace for the layer-5 TemporalTrace evaluator, by the QSpec model-subject
semantics (ADR-018 SM-1 to SM-5, QS-1, QS-2, QS-5). The evaluator's value on
one trace is the semantics: the model checker (FR-126), replay (FR-128) and
monitors all use it. The evaluator SHALL read a lasso exactly, past
operators included (ADR-018 SM-8). A model verdict is that value over every
admitted behaviour.

## Use case

A verification operator names the model, its initial snapshots and a key
set for each population, and asks whether a temporal claim holds. They
need one definition of what "a behaviour" and "position `i`" mean, so that
a counterexample the model checker reports evaluates false when they replay
it, and a model that halts reads the same way in every engine.

## Semantic authority and boundary

QSpec owns the model subject, behaviours, positions and their anchors, the
terminal-state readings, the `over` binding and the meaning of interval
operators over an infinite trace (ADR-018 QS-1, QS-2, QS-5, QS-13;
References), in QSpec FR-361. QSL specifies how it builds the subject from
FR-120, how its evaluator reads a lasso, what it charges and what it
reports.

## Inputs

- `ModelSubject` (`qsl_eval::model_subject`, ADR-018 LA-2): the S4 `CheckedPackage`; FR-120's
  package input, snapshot provision and limits; `initial: Vec<DocumentRef>`,
  a list of FR-106 initial snapshots; `universes:
  Vec<PopulationUniverse>`; and any `ProofBound`s (ADR-014 B-4).
- A checked temporal clause (FR-123) or a checked state-graph claim
  (FR-165), each with its `over` parameter, if any.

## Outputs

- `ModelSystem::new` over the subject's members (FR-120), whose successor
  relation is the subject's.
- For a behaviour: a sequence of positions, each an FR-120 state
  observation with an anchor, read by the TemporalTrace evaluator.
- `NoInitialState` (ADR-016 EX-10) for a subject with no initial state,
  which FR-127 settles `inconclusive`.
- `NoInstance` for an item with zero instances (Instancing), which FR-127
  settles `inconclusive`.

## Behavior

### QSpec model-subject rules

- QSL SHALL read the subject, its behaviours, their positions and anchors,
  terminal states, the `over` binding and the model verdict as QSpec FR-361
  states them (ADR-018 SM-2 to SM-5); the subject is part of every
  obligation identity over it (ADR-013 O-09).

### Building the subject

- QSL SHALL build the subject's successor relation as `ModelSystem::new`
  over the subject's package, snapshots, initial states and universes
  (FR-120), and SHALL read a behaviour's positions as FR-120's synthesized
  state observations (ADR-016 ID-10), each with the anchor of the
  transition that produced it.
- The evaluator SHALL evaluate a `holds` atom through the one clause
  evaluator (FR-107) over the observation at its position.
- The evaluator SHALL evaluate a `holds` atom over an object that does not
  exist at a position as FR-107 evaluates a reference to an absent object.
- When the subject's `initial` list is empty, QSL SHALL report
  `NoInitialState` before it explores any state.

### Instancing

- QSL SHALL instantiate a checked temporal clause or state-graph claim at
  request time, from the subject's `universes`: one instance per object of
  the subject's universes whose type conforms to the `over` parameter's type
  `T` (QSpec FR-361), with the parameter bound to that object. An object of
  a subtype of `T` conforms. The next bullets state how QSL realizes this
  over universes of keys. S3 records the `over` binding and does not
  instantiate, because the universes are not an S3 input; one checked claim
  serves every subject.
- Universes are kept per population (FR-120). When several populations hold
  objects conforming to `T`, the instances are the objects of all of them;
  instances are ordered by population declaration identity text, then by
  the order of the population's universe list, which is the canonical order.
  QSL SHALL list instances, including in `ModelLetters::bindings`, in the
  canonical order.
- A universe is a list of keys, and a key has no type until a state holds its
  object (FR-120). A population **admits `T`** when one of its member types is
  `T`, a subtype of `T`, or a supertype of `T`: QSpec FR-153 says a member type
  covers itself and every type that conforms to it, so a key of a population
  of `Animal` may hold a `Dog`. QSL SHALL instantiate each key of each
  population that admits `T` exactly once, whatever number of its member types
  are comparable with `T`. This follows QSpec FR-361 (an instance for each
  object that conforms to `T`) and FR-153; that the instance is the key, not a
  typed object, is QSL-proposed (Remaining work).
- QSL SHALL identify an instance by its population's declaration identity text
  and its key. That pair is the instance in the obligation identity
  (ADR-013 O-09) and the name of a refuted binding (QSpec FR-361); neither
  carries an object type.
- QSL SHALL read the bound parameter at a position as the reference of the
  state's object under the instance's population and key, carrying that
  object's own most-specific type and the population's `UniverseId` (FR-120),
  so FR-107's comparison by (universe, object type, key), `x = r` and
  `contains(allInstances<T>(P), x)` treat the bound object as they treat any
  reference to it. The bound value conforms to `T` at every position where it
  is read.
- Where the population holds no object under the key whose most-specific type
  conforms to `T`, QSL SHALL return, for a read through the bound parameter,
  the `absent-key` undefined value QSpec FR-361 states for an absent object.
  A claim over a subtype of a mixed population (such as `over (x: Dog)` over
  an `Animals` population) is therefore refuted, `undefined-evaluation`, at any
  key holding a non-conforming object unless the property guards the read, and
  QSL has no guard for it yet: population membership is true there and there
  is no type-test operator (Remaining work).
- When a population that admits `T` has no universe, QSL SHALL return
  `NotSimulated::RequiresBound` naming it (FR-120), before instantiating.
- When no population that admits `T` has a key, including an empty universe and
  a type no population's member types are comparable with, the claim has zero instances. QSL SHALL then
  report `NoInstance` for the item before it explores any state, and FR-127
  settles it `inconclusive`, so an item that was never evaluated is never read
  as holding.
- QSL SHALL instantiate a checked hyper clause (FR-172) at request time with
  one instance per key of the universe of the one population each object
  parameter `v: V::T` resolves to, the instances ranging over the product of
  those universes. For each of those populations with no universe, QSL SHALL
  return `NotSimulated::RequiresBound` naming it (FR-120), before
  instantiating.
- When any one of those populations has an empty universe, the hyper clause
  has zero instances. QSL SHALL then report `NoInstance` for the item before
  it explores any state, and FR-127 and FR-182 settle it `inconclusive`.
- QSL SHALL give a clause or claim with no `over` parameter one instance, with
  no bound object.

### Reading a lasso

- For a lasso with a prefix of `n` positions and a loop of `L` positions,
  the evaluator SHALL read state (atoms, the observation and its anchor) at
  a position `p >= n` from position `n + ((p - n) mod L)`.
- The evaluator SHALL compute each subformula's past reach `R` by ADR-018
  SM-8: 0 for an atom; the largest operand reach for a Boolean or future
  operator; `R + 1` for a previous-position operator; `R + b` for a past
  interval operator with upper bound `b`, with the larger operand reach for
  `since[a,b]` and `triggered[a,b]`; the largest operand reach plus `L`
  for an unbounded `once`, `historically`, `since` or `triggered`; and the
  largest operand reach plus `a + L` for a past `[a,*]` operator.
- The evaluator SHALL unroll the loop `m = ceil(R(φ) / L) + 1` times for a
  formula `φ`, evaluate every subformula over the `n + m × L` unrolled
  positions with past operators reading back to position 0, and read a
  subformula at a position past the unrolled positions at the position of
  the last unrolled loop copy congruent to it modulo `L`.
- The evaluator SHALL read every atomic predicate as false before position
  0, QSpec FR-092's complete-history rule.
- Under infinite-trace the evaluator SHALL give an interval operator QSpec
  FR-091's (future) or FR-092's (past) offset meaning, with no
  closed-boundary rule, and an `[a,*]` operator the limit of the closed
  interval `[a,b]` as `b` grows, whose `until` and `since` keep the left
  operand's range `[a,d)` (ADR-018 IV-2).

### Undefined evaluation

- At each position a claim reads, the evaluator SHALL read the claim's
  letter: the value of every atom of the claim at that position, and, for
  the deadlock-freedom item, the value of DL-1's `P` (FR-124's `When(P)`)
  at a terminal state (ADR-018 UE-1).
- A TP-2 `on origin` claim SHALL read positions 0 to its horizon `h`;
  every other claim SHALL read every position of the behaviour. A position
  that a bounded profile's closed-boundary rule supplies past a terminal
  state SHALL evaluate no atom.
- When an atom evaluates `Undefined` (the kernel `Outcome::Undefined` or a
  family's `FamilyResult::Undefined`) at a position the claim reads, the
  evaluator SHALL return the claim undefined at that position, with the
  `UndefinedRecord` of the first such atom in clause-node order as its
  cause.

### Work

- The evaluator SHALL charge one TR-5 work unit for each visit of a node at
  a position, unrolled positions included.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-125-AC-1 | Over the `Counter` subject (universe `{c}`, initial value 0, no `terminal` declaration), the one behaviour is positions `0, 1, 2, 3` with anchors `initialization`, `inc`, `inc`, `inc`. Under infinite-trace it continues with the terminal stutter step repeating position 3, whose transition identity differs from every `inc` identity. | Test (TC-520) |
| FR-125-AC-2 | Over that behaviour, `always[0,5] holds(c.value <= 3)` from position 0 evaluates `false` under event-position false-extension (positions 4 and 5 are past closure) and `true` under infinite-trace (the stutter repeats `value = 3`); `eventually holds(c.value = 3)` under infinite-trace evaluates `true`, and `always eventually holds(c.value = 0)` evaluates `false`. | Test (TC-520) |
| FR-125-AC-3 | On ADR-018 §6's lasso (empty prefix, loop `(0,0) (1,0) (2,0)`), `eventually[0,4] holds(a.versionNumber = 0)` at position 2 reads the states of positions 2 to 6 by wrapping and evaluates `true` (position 3 holds `(0,0)`); `once[0,3] holds(a.versionNumber = 2)` at position 1 reads positions 1 and 0, the rest before position 0 as false, and evaluates `false`; each visit charges one work unit. | Test (TC-520) |
| FR-125-AC-4 | Over ADR-018 §6's subject, a clause with `over (c: Config::ConfigVersion)` yields one instance per object of `{a, b}`. A subject whose `initial` list is empty is `NoInitialState` before any state is explored. | Test (TC-520) |
| FR-125-AC-5 | On the same lasso, `always eventually (holds(a.versionNumber = 0) and once[1,1] holds(a.versionNumber = 2))` has past reach 1 and is evaluated over `m = 2` loop copies, 6 positions: the conjunction is `false` at position 0, which has no previous position, and `true` at position 3, whose previous position holds version 2, and the formula evaluates `true`. The same conjunction read at position 9 by congruence is `true`. | Test (TC-520) |
| FR-125-AC-6 | Over the `Counter` subject, the letter of `always holds(6 / (2 - c.value) >= 0)` under infinite-trace is defined at positions 0 and 1 and undefined at position 2, cause `division-by-zero`. `eventually[0,1] holds(6 / (2 - c.value) = 6)` under event-position false-extension, `on origin`, reads positions 0 and 1 only and evaluates `true`. | Test (TC-537) |
| FR-125-AC-7 | Over ADR-018 §6's subject (universe `{a, b}`), the state-graph claims `ReachesTwo` and `ReachesThree` (`over (c: Config::ConfigVersion)`) each yield two instances, `a` then `b`; `possible Anything using v { true }` with no `over` yields one instance with no bound object. `ReachesTwo`, checked once, yields `a`, `b` over `{a, b}` and `a` over a subject whose universe is `{a}`, without being re-checked. A claim over a subtype-conforming population yields its objects; a universe-less population is `RequiresBound`; an empty universe yields zero instances and the item reports `NoInstance`, settled `inconclusive`, never proved. | Test (TC-520) |
| FR-125-AC-8 | (QSL-proposed rule, STD-196: instance is a key) For a population `P` declaring member types `Lamp` and `Switch`, both subtypes of `Thing`, each key of its universe is one instance of `over (x: Lamp)`, and one instance (not two) of `over (x: Thing)`. A population none of whose member types is `T`, a subtype or a supertype of `T` yields no instance. A population that admits `T` and has no universe is `RequiresBound`; when no population that admits `T` has a key the item gives `NoInstance`. | Test (TC-520) |
| FR-125-AC-9 | (QSL-proposed rule, STD-196: instance is a key) With `Lamp`'s `DeclarationKey` ordering before `Switch`'s, an instance of `over (x: Thing)` reads at a position whose object under the key is a `Switch` as the reference with type `Switch`, and at one whose object is a `Lamp` as the reference with type `Lamp`; for `over (x: Lamp)` the position whose object is a `Switch`, or has no object, reads `absent-key` undefined. The instance is named by its population and key in the obligation identity and in a refuted binding, without a type. | Test (TC-520) |
| FR-125-AC-10 | (QSL-proposed rule, STD-196) With populations `P1` and `P2`, whose declaration identity texts order `P1` before `P2`, holding universes `[d, c]` and `[b, a]` respectively, the instances of a claim admitted by both are ordered `P1/d`, `P1/c`, `P2/b`, `P2/a`, and `ModelLetters::bindings` returns that order, not an ascending reference order. | Test (TC-520) |
| FR-125-AC-11 | (QSL-proposed rule, STD-196) For an instance of `over (x: Thing)` whose object is a `Switch` (not the least member type), `holds(x = r)` where `r` is the state's reference to that object, and `holds(contains(allInstances<Thing>(P), x))`, each evaluate `true`; for a `Lamp` object likewise. | Test (TC-520) |
| FR-125-AC-12 | (QSL-proposed rule, STD-196: instance is a key) A population `Dogs` declaring `Dog` and a population `Animals` declaring `Animal` and holding a `Dog` object under key `k`, with `Dog` a subtype of `Animal`: `over (x: Dog)` instances every key of `Dogs` and every key of `Animals`, so `Animals/k` is checked and read as the `Dog` reference; a position where an `Animals` key holds a `Cat`, or no object, reads `absent-key` undefined. A population declaring only `Animal` with a `Dog` object is not `NoInstance` for `over (x: Dog)`. | Test (TC-520) |

## Remaining work

- What follows QSpec: an instance for each object whose type conforms to `T`
  (FR-361), a member type covering every conforming type (FR-153), so a
  population admits `T` by a member type that is `T`, a subtype or a supertype
  of `T`, and `absent-key` for an absent object (FR-361). What is
  QSL-proposed (AC-8 to AC-12, and the instance order of AC-10): that an
  instance is a (population, key) pair, because the universe is a list of keys
  (QSL FR-120's `keys: Vec<String>`; QSpec FR-331 names `universes` without a
  shape); that the bound value is the state's own reference; and the canonical
  instance order. QSpec follow-up: Linear STD-196, with the open questions of
  the key-versus-object shape and of the membership guard: population
  membership is true where a key holds a non-conforming object, so QSpec
  FR-361's membership guard does not prevent the `absent-key` refutation. The
  consequence stands until QSpec rules: `over (x: Dog)` over a population that
  also holds a `Cat` is refuted at that key whatever the property, since there
  is no type-test operator. QSpec may add a guard operator, exclude such keys
  from the instances, or settle the item `inconclusive` with a named cause. If
  QSpec rules otherwise, its rule replaces these.
- Code follow-up (QSL-886): `ModelLetters::bindings` in qsl-eval still returns
  an `ObjectReference` whose object type this requirement no longer
  specifies, and that value flows into the replay `ContextBinding`; the
  binding should name the population and key only.
- FR-169 (state-graph claims) instances under these rules. FR-182 (hyper
  clauses) instances by the hyper bullet above and takes the reference reading
  (the bound value is the state's own reference) and the `absent-key` rule;
  no checker in the repository exercises either yet, so AC-8 to AC-11 are
  tested through the temporal claim only.

## Dependencies

- ADR-018 §1 UE-1, §2 SM-1 to SM-5 and SM-8, §10 DL-6, §11 IV-2; ADR-014 §3 TR-2 and TR-5, §5 A-4;
  ADR-016 ID-10 (synthesized state observation) and EX-10.
- [FR-104](FR-104-check-state-clauses.md) (member-type conformance and the
  canonical member type).
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md) (successor
  relation), [FR-106](FR-106-admit-snapshots-and-invocations.md) (initial
  snapshots), [FR-107](FR-107-evaluate-state-clauses-at-s6a.md) (clause
  evaluation), [FR-101](FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md),
  [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (checked clause), [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md)
  (terminal states), [FR-165](FR-165-check-state-graph-claims-at-s3.md)
  (checked state-graph claim).

## References

- QSpec FR-361 (model behaviours read as temporal traces) and FR-367
  (interval operators under infinite-trace): the QSpec half of ADR-018 QS-1,
  QS-2, QS-5 and QS-13 (Linear STD-131).
