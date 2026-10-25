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
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: references
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
- `invalid_runtime_input`/`conflicting-identity` for an item whose
  overlapping populations' initial member records give one key two types
  (Instancing).
- `ModelSystem::instances` and `ModelSystem::excluded_keys` for a claim: its
  instances in the canonical order, and the universe keys no initial record
  admits (Instancing).

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
  `NoInitialState` before it explores any state ("Instancing" states its
  place among the other outcomes).

### Instancing

QSpec FR-361 "Binding of the clause's `over` parameter" owns these rules; the
bullets state how QSL applies them over FR-120's subject.

- QSL SHALL instantiate a checked temporal clause or state-graph claim at
  request time, from the subject's `universes` and initial states. S3 records
  the `over` binding and does not instantiate, because the universes are not
  an S3 input; one checked claim serves every subject.
- A universe is a list of keys, and a key has no type until a member record
  admits it (FR-120). A population **overlaps `T`**, the `over` parameter's
  type, when one of its member types conforms to `T` or `T` conforms to one
  of its member types (QSpec FR-153: a member type covers itself and every
  type that conforms to it, so a key of a population of `Animal` may hold a
  `Dog`). A population whose member types are all unrelated to `T` does not
  overlap it.
- An **admitted key** is a key of the universe of at least one population
  that overlaps `T` and that a member record of some initial state of the
  subject admits. QSL SHALL give the claim one instance per distinct admitted
  key. All populations that overlap `T` lie in `T`'s object universe (ADR-013
  O-05: one `UniverseId` per connected component of the supertype graph), so
  one key text listed by several of them is one object and one instance.
- The claim's **excluded keys** are the universe keys of the populations that
  overlap `T` that no member record of any initial state admits, each listed
  once, in the instance order below. A key that only an operation creates is
  an excluded key, never an instance. A verdict over the item covers its
  instances only: a `proved` item has not checked an object created under an
  excluded key, and a refuted item never names one. `ModelSystem::excluded_keys`
  SHALL return them for a claim; they are a function of the subject, so no
  verdict member carries them.
- QSL SHALL order the instances by first occurrence in the sequence that
  lists the populations overlapping `T` in ascending UTF-8 byte order of their
  declaration identity texts, each universe in its list order. QSL SHALL list
  instances, including in `ModelSystem::instances`, in that order. The
  reported instance of a refuted item is the first refuted instance in that
  order, and its counterexample, basis and cause are that instance's (FR-127).
- QSL SHALL identify an instance by its key alone. The key is the instance
  in the obligation identity (ADR-013 O-09) and in the binding of the
  instance's FR-338 automaton-state key context (`["instance", key]`);
  neither carries a population or an object type. A refutation names the
  instance by its key (QSpec FR-361); the binding it writes out is the
  reference the next bullet states, which carries a type and no population.
- QSL SHALL read the bound parameter at a position as the reference
  (`UniverseId`, most-specific type, key) of the object the state holds under
  the instance's key (FR-120's reference triple), carrying that object's own
  most-specific type, never a population's member type. FR-107's comparison
  by (universe, object type, key), `x = r` and
  `contains(allInstances<T>(P), x)` therefore treat the bound object as they
  treat any reference to it, from whichever overlapping population it is
  reached. A binding written out of the position (a refuted binding, a replay
  `over_binding`) is the reference (`UniverseId`, admitted type, key), whose
  type is that of the member record admitted for its key, which the next
  bullet makes unique; it carries no population.
- A key has one type across the subject. After the `RequiresBound` and
  `NoInitialState` checks below, QSL SHALL compare the member records of every
  overlapping population in every initial state. Two member records with one
  universe and key and different types, in one population or in two, in one
  initial state or in two, SHALL refuse the item
  `invalid_runtime_input`/`conflicting-identity`, naming the key and both
  types, before it forms any instance.
- An instance whose admitted record does not conform to `T` (an `Animal` key
  holding a `Cat` under `over (x: Dog)`) is an instance. Where no overlapping
  population holds under the key an object whose most-specific type conforms
  to `T`, including a position where no object is held, QSL SHALL return, for
  every evaluated read through the bound parameter other than the guard or a
  forwarding read below, the `absent-key` undefined value QSpec FR-361
  states. This applies only when the expression actually reads through the
  bound parameter; an atom that does not read `x` is evaluated normally. The
  claim is then undefined at that position and the item settles `refuted`,
  `undefined-evaluation`, naming the instance, the position and the read's
  locus (FR-127).
- **The guard.** For a parameter `x` bound by `over`, `lookup<T>(p, x)`
  (FR-084) SHALL be present exactly when `p` holds under `x`'s key an object
  whose most-specific type conforms to `T`, and SHALL otherwise give the
  declared absence mode, never a type refusal. A clause guards its reads
  through `x` with the disjunction over every population `p1`, `p2`, ... that
  overlaps `T`: `present(lookup<T>(M::p1, x) absent empty) or
  present(lookup<T>(M::p2, x) absent empty) or ...`. The disjunction is true
  exactly when the key holds an object of a type conforming to `T` in at
  least one listed population. A guard naming only some of the overlapping
  populations is false at the keys only the others list, and holds the
  guarded property vacuously there. QSpec FR-361 adds no type-test operator
  and no population-free membership form, so an unguarded read at a key whose
  object does not conform to `T` refutes the item whatever the property.
- **Guard provenance.** QSL SHALL attach `x`'s provenance to an immutable
  `let` binding initialized with the same reference value. When a function
  call binds a formal parameter to an actual argument carrying
  `x`'s provenance, QSL SHALL attach that provenance to the formal parameter.
  When a function returns the same reference value from a provenance-bearing
  input, QSL SHALL propagate that provenance to its result. The provenance
  denotes the binding origin of the reference, not equality with an
  independently supplied reference. QSL SHALL treat a `lookup<T>(p, r)`
  reference operand `r` carrying `x`'s provenance as the guard operand,
  including when the lookup is inside a called function or `r` is returned
  unchanged through a helper function. QSL SHALL allow a reference to be
  forwarded through a `let` binding or function call without returning
  `absent-key`; this exception permits only passing the reference onward. QSL
  SHALL return `absent-key` for every evaluated field read or other operation
  through `x` or its alias outside a `lookup` reference operand when no
  conforming object is held. QSL SHALL NOT treat a helper result that lacks
  `x`'s provenance as a guard operand. When the lookup reports absence, QSL
  SHALL evaluate its `present` test as false. QSL SHALL NOT evaluate an
  implication consequent while its guard is false.
  Under FR-361, when the unit selects the model profile, a helper function
  reachable by function calls from a model-context call site SHALL derive and
  carry the StateModel family needed by its body, even when its `using` alias
  selects `quire.value.complete/v1`. Reachability is transitive through
  helper calls. A function not reachable from a model-context call site,
  including one with a model-shaped parameter and a bare field read, remains
  under its declared family. Each helper's declaration and `decreases`
  measure remain under their declared selection. Every helper remains subject
  to ordinary S3 resolution and assembly refusals.
- S3 SHALL admit an `over (x: T)` clause when more than one population
  overlaps `T`. Instancing and reads use all such populations: their universes
  contribute keys in the order specified above, equal keys form one
  instance, and the guard SHALL test each overlapping population. S3 SHALL
  NOT refuse the clause as ambiguous solely because multiple populations
  overlap `T`.
- QSL SHALL decide the item's instancing outcomes in this order, each before
  it explores any state:
  1. When a population that overlaps `T` has no universe, QSL SHALL return
     `NotSimulated::RequiresBound` naming it (FR-120).
  2. When the subject's `initial` list is empty, QSL SHALL report
     `NoInitialState`, whatever the universes hold.
  3. The `conflicting-identity` check above.
  4. When the claim has no admitted key, because no population overlaps `T`,
     every overlapping population has an empty universe, or no member record
     of an initial state admits a key of their universes, QSL SHALL report
     `NoInstance` for the item, and FR-127 settles it `inconclusive`, so an
     item that was never evaluated is never read as holding.
- QSL SHALL instantiate a checked hyper clause (FR-172) at request time with
  one instance per key of the universe of the one population each object
  parameter `v: V::T` resolves to, the instances ranging over the product of
  those universes. For each of those populations with no universe, QSL SHALL
  return `NotSimulated::RequiresBound` naming it (FR-120), before
  instantiating.
- When any one of those populations has an empty universe, the hyper clause
  has zero instances. QSL SHALL then report `NoInstance` for the item before
  it explores any state, and FR-127 and FR-182 settle it `inconclusive`.
- A hyper clause's parameter bound to a key SHALL read as an `over`
  parameter does: as the reference of the object the state holds under the
  key, with that object's own most-specific type, and as the `absent-key`
  undefined value where the parameter's population holds under the key no
  object whose most-specific type conforms to the parameter's type.
- QSL SHALL give a state-graph or probabilistic clause or claim with no `over`
  parameter one instance, with no bound object. A `temporal` clause always
  names an `over` parameter (QSpec FR-370-AC-13): S1 SHALL refuse one written
  without it, `invalid_syntax`/`unexpected-token` at the token that stands
  where `over` is required, and QSL produces no node.

### The `on each` activation guard

The `when` guard of a clause's `on each` activation (FR-337) is the
activation guard. It is not "The guard" above, which is a `lookup`
disjunction a clause writes inside its formula: FR-125-AC-12's `G` is such a
formula guard and is read as an ordinary `holds` condition.

- QSL's production letter seam (FR-107 `ModelLetters`, built over a checked
  clause and FR-120's `ModelSystem`) SHALL read the activation guard of a
  clause for an instance at a state as it reads a `holds` atom there: through
  the clause evaluator over the state's observation, with the `over`
  parameter bound as "Instancing" binds it and the trigger binder (FR-337
  slot 1) bound to the same reference, the `absent-key` rule and the
  `lookup` guard operand applying to a read through either. The reading is true where the position
  activates the clause and false where it does not; an `absent-key` or
  kernel undefined value is the undefined reading, which FR-126 turns into
  `kind: UndefinedEvaluation` at the activation, never a false reading.
- A guard evaluation that gives no reading (a refused or faulted
  evaluation, a state the system cannot observe, an exhausted meter) SHALL
  stop the check exactly as a `holds` atom's evaluation that gives no
  reading stops it.
- The guard's footprint (FR-156 "Atom footprints") SHALL be the locations
  the guard reads, with the `over` parameter as the receiver of a read
  through `over` or through the trigger binder, computed as an atom's
  footprint is; FR-126's partial-order run joins it into the atoms'.
- The engine (FR-126) SHALL read the activation guard only for a clause
  whose activation has one. For a clause with no guard the seam SHALL return
  an empty guard footprint.
- FR-128's replay and FR-338's and FR-339's checkers SHALL read the guard
  through the same seam, from the recompiled package.
- A clause's captures are read at the activation through the same seam, in
  source order and only where the guard is true, as FR-337 "Capture
  observation" states; an undefined capture is `kind: UndefinedEvaluation`
  located at the capture, and a capture is never read as a default.

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
| FR-125-AC-6 | Over the `Counter` subject, the letter of `always holds(6 mod (2 - c.value) >= 0)` under infinite-trace is defined at positions 0 and 1 and undefined at position 2, cause `division-by-zero`. `eventually[0,1] holds(6 mod (2 - c.value) = 0 and c.value = 1)` under event-position false-extension, `on origin`, reads positions 0 and 1 only and evaluates `true`. | Test (TC-537) |
| FR-125-AC-7 | Over ADR-018 §6's subject (universe `{a, b}`), the state-graph claims `ReachesTwo` and `ReachesThree` (`over (c: Config::ConfigVersion)`) each yield two instances, `a` then `b`; `possible Anything using v { true }` with no `over` yields one instance with no bound object. `ReachesTwo`, checked once, yields `a`, `b` over `{a, b}` and `a` over a subject whose universe is `{a}`, without being re-checked. A claim over a subtype-conforming population yields its objects; a universe-less population is `RequiresBound`; an empty universe yields zero instances and the item reports `NoInstance`, settled `inconclusive`, never proved. | Test (TC-520) |
| FR-125-AC-8 | For a population `P` declaring member types `Lamp` and `Switch`, both subtypes of `Thing`, whose universe keys `l` and `s` are admitted by initial records of a `Lamp` and a `Switch`, `over (x: Lamp)` and `over (x: Thing)` each have exactly the instances `l` and `s`, one per key. A population none of whose member types conforms to `Lamp` or is conformed to by it yields no instance. A key of `P`'s universe that no initial record admits is an excluded key, not an instance, and `ModelSystem::excluded_keys` returns it. | Test (TC-520) |
| FR-125-AC-9 | An instance of `over (x: Thing)` reads at a position whose object under its key is a `Switch` as the reference with type `Switch`, and at one whose object is a `Lamp` as the reference with type `Lamp`; for `over (x: Lamp)` a read at a position whose object is a `Switch`, or has no object, is `absent-key` undefined. The instance is identified by its key alone in the obligation identity and the FR-338 key context, with no population and no type; a refuted binding of `over (x: Thing)` at the key `s`, whose admitted record is a `Switch`, is the reference (`UniverseId`, `Switch`, `s`), with no population. | Test (TC-520) |
| FR-125-AC-10 | With populations `P1` and `P2`, whose declaration identity texts order `P1` before `P2`, holding universes `[d, c]` and `[b, a]`, every key admitted, the instances of a claim both overlap are `d`, `c`, `b`, `a`, and `ModelSystem::instances` returns that order, not an ascending key order. With `P2`'s universe `[c, a]` instead, the instances are `d`, `c`, `a`: the key `c` listed by both is one instance, at its first occurrence. With `P1`'s universe `[d, u]` and `P2`'s `[u, a]`, where no initial record admits `u`, the instances are `d`, `a` and the excluded keys are `[u]`, listed once. | Test (TC-520) |
| FR-125-AC-11 | For an instance of `over (x: Thing)` whose object is a `Switch` (not the population's first member type), `holds(x = r)` where `r` is the state's reference to that object, and `holds(contains(allInstances<Thing>(P), x))`, each evaluate `true`; for a `Lamp` object likewise. With the key `c` listed by `P1` and `P2` of `Thing`, `holds(contains(allInstances<Thing>(P2), x))` for the instance `c` evaluates `true`. | Test (TC-520) |
| FR-125-AC-12 | A population `dogs` declaring `Dog` with universe `[d]`, and a population `animals` declaring `Animal` with universe `[k, t]`, where an initial state holds a `Dog` under `k` and a `Cat` under `t` in `animals` and a `Dog` under `d` in `dogs` (`Dog` and `Cat` subtypes of `Animal`), with `Animal` declaring `age: Int[0, 9]`, give `over (x: Dog)` the instances `k`, `t`, `d` (`animals` orders before `dogs`). S3 admits the clause with both overlapping populations, with no ambiguous-population refusal. `k` reads as the `Dog` reference. The unguarded `always holds(x.age >= 0)` refutes, `undefined-evaluation`, naming `t`; `always holds(true)` evaluates normally and is true at `t`, where its atom does not read `x`. With `G` the guard `present(lookup<Dog>(M::animals, x) absent empty) or present(lookup<Dog>(M::dogs, x) absent empty)`, `always holds(G implies x.age >= 0)` evaluates at every instance and position, `G` being false at `t` and true at `k` and `d`, and settles `proved`. `present(lookup<Dog>(M::animals, x) absent empty)` alone is false at `d`, so a property guarded by it holds vacuously there. A subject whose only population is `animals` holding only `Cat` objects still has instances for `over (x: Dog)` and is not `NoInstance`. | Test (TC-520) |
| FR-125-AC-13 | Outcome order. A subject in which an overlapping population has no universe returns `RequiresBound` naming it, also when its `initial` list is empty. A subject with an empty `initial` list and bounded universes reports `NoInitialState`, never `NoInstance`. Two member records under the key `c`, one an `Animal` in `animals` and one a `Dog` in `dogs`, in one initial state or in two initial states, refuse the item `invalid_runtime_input`/`conflicting-identity` naming `c` and both types, before any instance is formed; agreeing types are admitted. A subject with an initial state, bounded universes and no admitted key reports `NoInstance`. Each outcome arrives before any state is explored. | Test (TC-520) |
| FR-125-AC-14 | Over `P1` of `Counter` with universe `[c, d, z]`, initial records under `c` and `d` only, value 0 each, `inc` disabled, and an operation `make` that creates a `Counter` with value 1 under `z`, `over (x: Counter)` has the instances `c` and `d` and the excluded keys `[z]`, and `always holds(x.value = 0)` settles `proved`, although `z`'s object has value 1 at a reachable position. | Test (TC-520) |
| FR-125-AC-15 | A state-graph claim `over (x: Thing)` (FR-169) over AC-10's populations has the instances, order and excluded keys a temporal claim has. A hyper clause (FR-172) whose parameter `v: V::Thing` resolves to `P` of AC-9 reads `v`, at each position, as the reference with the held object's most-specific type, and a parameter typed `V::Lamp` reads `absent-key` undefined where the key holds a `Switch`. | Test (TC-520) |
| FR-125-AC-16 | With a population `all` of member type `Entity` (a supertype of `Counter`) whose initial state holds a `Counter` under `c` (value 0, with `inc` enabled) and an `Entity` that is not a `Counter` under `e`, the unguarded claim `always holds(x.value <= 0)` over `Counter` refutes at both instances, `c` by a false evaluation after `inc` and `e` by an `absent-key` read. With `all = [c, e]` the reported instance is `c`, with no undefined cause; with `all = [e, c]` it is `e`, with cause `undefined-evaluation`. The item is `refuted` in both orders, and the two subjects give different request identities. | Test (TC-520) |
| FR-125-AC-17 | On AC-12's `dogs` and `animals` subject, let `y` be an immutable `let` alias of `x`; let a helper `same(r)` return its reference argument unchanged, `isDog(r)` test the disjunction of `lookup<Dog>` in `animals` and `dogs` using `same(r)`, and `ageOK(r)` read `r.age >= 0`. The unit selects the model profile and each helper's `using` alias selects `quire.value.complete/v1`; because `same`, `isDog` and `ageOK` are reachable from the model-context temporal formula, S3 derives the StateModel family for their bodies per FR-361. An uncalled function with a bare field read on a `Reference` remains in its declared family and refuses as TC-1031 N16 specifies. `always holds(isDog(y) implies ageOK(y))` settles `proved`: at `t`, the guard is false and `ageOK` is not called; at `k` and `d`, the guard and property are true. At `t`, the reference is forwarded through `y`, `same` and `isDog` without `absent-key`; the unguarded `always holds(ageOK(y))` still refutes with `undefined-evaluation`/`absent-key` at `t`. | Test (TC-520), Test (TC-1031) |
| FR-125-AC-18 | Compiled from source (FR-337-AC-6) and read through QSL's production letter seam, with no test letters: over the `Counter` subject under event-position false-extension, `eventually[0,1] holds(c.value = 3)` `on each (q: M::Counter) when (c.value = 1)` returns `Violated` with the prefix `0 -inc-> 1 -inc-> 2`, and `when (c.value >= 2)` returns `Holds{Exhaustive}`; under infinite-trace, `always holds(c.value <= 1)` `on each (q: M::Counter) when (c.value >= 3)` returns `Violated` with the prefix `0 -inc-> 1 -inc-> 2 -inc-> 3`, which FR-128 replays to `reproduced-with-evaluated-witness`, and `always holds(true)` `on each (q: M::Counter) when (6 mod (2 - c.value) = 0)` returns `Violated` with `kind: UndefinedEvaluation` at position 2, cause `division-by-zero`, its site naming the activation. On AC-12's `dogs` and `animals` subject, `always holds(true)` `on each (y: M::Dog) when (x.age >= 1)` refutes `undefined-evaluation` naming `t`, the guard's read through `x` being `absent-key` there, and, with AC-12's `G` as the activation guard (`when (G)`), the seam's guard reading at the initial state is false for `t` and true for `k` and `d`, and `always holds(false)` `on each (y: M::Dog) when (G)` refutes naming `k`, the first instance where the clause activates, at position 0. The guard footprint of the `c.value >= 3` clause is the read of `Counter`'s `value` through `c`, so `inc`, which writes it, is visible to a partial-order run of that clause. | Test (TC-520) |

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
