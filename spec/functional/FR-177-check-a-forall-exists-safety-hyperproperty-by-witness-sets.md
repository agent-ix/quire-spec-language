---
id: FR-177
title: "Check a forall-exists safety hyperproperty by witness sets (EN-1)"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-021
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-173
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-176
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-184
    type: depends_on
---
# FR-177: Check a forall-exists safety hyperproperty by witness sets (EN-1)

## Description

QSL's layer-A `qsl-analyze` engine EN-1 (ADR-029 CB-3) SHALL decide an HP-3 item by exploring a
witness-set product (ADR-023 HC-2): each state is a tuple of universal
component states with the set `X` of (existential tuple, safety automaton
state) pairs still consistent with it. An empty `X` from which a fair
universal cycle exists is a violation. The `WitnessSet` step is one construction
in `qsl-eval`, which ADR-020's hidden-field refinement and the core
certificate checker (FR-163) also use. It needs no Büchi complementation.

## Use case

A verification operator states that for every run of a vault there is a
run with the other secret and the same inputs whose public outputs are the
same. On the leaky vault, after one step no such run is left, and the
engine returns the universal run and the position at which every candidate
partner has failed.

## Inputs

- A `ModelCheckRequest` (FR-126) whose item is an HP-3 clause (FR-173), with
  one subject per alias and `ModelCheckLimits` (FR-184), after FR-176's
  pre-check.

## Outputs

- FR-126's `ModelCheckOutcome`, with `Violated` carrying a
  `HyperCounterexample` of kind `WitnessExhausted{position}` (FR-183).

## Behavior

### Safety automaton

- The engine SHALL build the Büchi automaton for the body, remove every
  state whose language is empty, and treat every remaining state as
  accepting.

### Product

- The product state SHALL be (universal tuple, `X`), `X` a set of
  (existential tuple of model states, safety automaton state). `X` SHALL be
  stored as the sorted sequence of its members' keys, so equal sets
  coalesce.
- The initial states SHALL be, for each initial universal tuple, `X_0`
  holding every (initial existential tuple, automaton state reached by
  reading joint position 0).
- A successor SHALL exist for each universal joint step on which `μ_U`
  holds; its `X'` SHALL hold every (existential post-tuple, automaton
  successor) reached from a member of `X` by an existential joint step on
  which `μ_E` holds against that universal step, the automaton reading the
  joint post-position.
- With no universal variable, the one universal tuple is the empty tuple
  and every joint step is an existential one.
- When an `X` would hold more than `max_witness_set` members, the engine
  SHALL stop and return `Stopped(ResourceExhausted, MaxWitnessSet)`
  (FR-184).

### Undefined and undetermined evaluation

- The engine SHALL evaluate `μ_U`, `μ_E` and the safety automaton's letters
  through the one clause evaluator (FR-107) at every joint step and joint
  position it builds, for universal and existential components alike.
- If one evaluates `Undefined`, then the engine SHALL return `Violated`
  with QSpec FR-400's `Undefined` counterexample ending at that joint
  position, one path per universal and existential variable, the first such
  position in canonical breadth-first order (ADR-023 HV-8). Between it and
  an empty `X`, the first in that order SHALL be returned (see "Ranking
  findings").
- If `μ_U` or `μ_E` evaluates refused or incomplete at a reachable joint
  step, and no refuting evidence comes first, then the engine SHALL return
  `Undecided(MatchUndetermined)` (ADR-023 HV-4).
- If a universal component's contract conjunction is refused or incomplete
  at a product state, or an existential contract conjunction is refused,
  incomplete or undefined at a member of that state's `X`, so that the
  member's existential steps are not exactly known, and no refuting evidence
  comes first, then the engine SHALL return `Undecided(UndecidedSuccessor)`
  and never `Holds`. A universal component's contract conjunction that is
  undefined is an undefined evaluation of the universal side.
- When the evaluation is on the universal side at a product state whose `X`
  is not empty, the existential variables' paths SHALL be the path of the
  first member of `X` in `X`'s sorted order, which ends at that member's
  tuple. For an undefined `μ_U` on a joint step out of the state, the
  universal paths include that step and the existential paths end at the
  state it leaves, one step short, and `trace_position` is the position the
  step enters. For an undefined `μ_E` or a body letter, the evaluation is
  of one member, and the existential paths are that member's, including the
  existential step that was evaluated.
- An `Undefined` counterexample whose locus is a body letter or a `match`
  conjunct SHALL name every trace variable, universal and existential, in
  quantifier order, each with its state key at the position, so that
  `variables` and `state_keys` hold as many entries as there are traces,
  however many universal variables the clause has. One whose locus is a
  component's contract conjunction names that component alone, as QSpec
  FR-400 states for a guard.

### Ranking findings

- A finding SHALL have the joint position at which it is made: an undefined
  letter, the position the letter reads; an undefined or refused `μ_U` or
  `μ_E`, the position its joint step enters, one past the state it leaves;
  a contract conjunction, the position of its product state; an empty `X`
  with a fair universal cycle, the first joint position at which `X` is
  empty.
- The engine SHALL return the finding at the least joint position. At one
  position an undefined evaluation SHALL come before a refused or incomplete
  `μ`, or a contract conjunction that leaves a member's steps unknown, which
  SHALL come before an empty `X`, since a refused `μ_E` may have dropped
  members from the `X` that then empties. At
  one position and rank the first in FR-101's breadth-first discovery order
  of product states SHALL win, within a state the first finding its
  expansion recorded, and across instances the earlier in FR-125's canonical
  order.
- A later finding SHALL NOT replace an earlier one: an undefined evaluation
  at a position after an empty `X` with a fair universal cycle returns the
  empty `X`'s `WitnessExhausted` counterexample.

### Instances

- The engine SHALL take the clause's instances in FR-125's canonical order
  and rank their findings as FR-126 "Bindings and ranking" does: a later
  instance wins only with a strictly earlier finding by the order above.
- An empty `X` is found only once a fair universal cycle past it is known,
  so a finding at a joint position after 0 SHALL NOT bound a later instance:
  each later instance is explored in full, and a later instance that stops
  on a limit before it is explored returns its `Stopped`.
- A finding at joint position 0 that is an undefined evaluation SHALL end the
  search, since nothing can precede it. A finding at position 0 of a later
  rank SHALL bound each later instance to the first phase through the
  expansion of its initial states (horizon 1), which is enough to find the
  position-0 undefined evaluation that would replace it.

### Violation

- `X` empty SHALL be absorbing.
- The engine SHALL run FR-126's SCC phase over the product states with `X`
  empty, with the fairness filter scoped to universal components as FR-176
  scopes it. A reachable state with `X` empty from which a cycle exists that
  is fair for every universal variable SHALL return `Violated`. With no
  universal variable, a reachable empty `X` SHALL return `Violated`.
- The counterexample SHALL be the canonical universal lasso through the
  first such state, with `position` the first joint position at which `X`
  is empty. The first such state is the first in FR-101's discovery order
  among the empty-`X` states from which a fair component of the empty part
  is reachable. The lasso SHALL be the tree path to it, then the shortest
  path on, by the retained edges in their stored order, to the nearest
  state of a fair component, then that component's canonical loop (ADR-018
  CX-5).
- The loop entry SHALL be canonical: it SHALL rotate back one step at a time
  while the lasso is unchanged, that is while the lasso's last step is the
  step before the entry and both leave universal states with equal
  state keys. The lasso with the least loop entry is the one reported.
- No such state SHALL return `Holds{Exhaustive}`, or `Holds{Reduced{…}}`
  under FR-174's diagonal symmetry, `X` renamed member by member and
  re-sorted.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-177-AC-1 | ADR-023 §8.2's `Opaque` over the leaky vault: from `a = (0, 0)`, `X_0` is `{((1, 0), g)}` and `X_1` is empty; the product has 6 reachable states; the item returns `Violated` with `WitnessExhausted{position: 1}` whose one trace is `a`'s lasso `(0, 0) -step(0)-> (0, 0)` with loop entry 0. | Test (TC-602) |
| FR-177-AC-2 | `Opaque` over the secure vault: every reachable universal state `(h, l)` has `X = {((1 - h, l), g)}`, the product has 4 states, and the item returns `Holds{Exhaustive}`. | Test (TC-602) |
| FR-177-AC-3 | The `exists trace b of V exists trace c of V` clause (`n = 0`) with body `holds(v.h @ b != v.h @ c) and always holds(v.l @ b = v.l @ c)` and `match { b.step.Vault::step.i = c.step.Vault::step.i }` returns `Holds` over the secure vault and `Violated` over the leaky vault. | Test (TC-602) |
| FR-177-AC-4 | `Opaque` over the leaky vault with `max_witness_set` 0 returns `Stopped(ResourceExhausted, MaxWitnessSet)` naming the limit and its value; with the default it returns AC-1's outcome. | Test (TC-602) |
| FR-177-AC-5 | `Opaque` over the secure vault with the added `match` conjunct `(if present(b.step.Vault::step.i) then 1 / value(b.step.Vault::step.i) else 1) = 1` on the existential variable `b`, which has no value on a joint step whose `b` step is `step(0)`: the item returns `Violated` with an `Undefined` counterexample whose traces hold a path for `a` and for `b`, ending at the first such joint step in canonical order, with cause `division-by-zero`; it never returns `Holds`. | Test (TC-602) |
| FR-177-AC-6 | `Opaque` with a `match` conjunct that calls a function whose evaluation exhausts its meter at the first joint step (incomplete) returns `Undecided(MatchUndetermined)` and never `Holds`. | Test (TC-602) |
| FR-177-AC-7 | `Opaque` over the leaky vault whose `step` has the precondition `1 div (1 - self.l + self.h) >= 0`, which has no value in the state `(h, l) = (0, 1)`, with the added `match` conjuncts `(if present(a.step.Vault::step.i) then value(a.step.Vault::step.i) else 0) = 0 and b.step.op != stutter`, so that `a` never reaches `(0, 1)` and an existential member does: the item returns `Undecided(UndecidedSuccessor)` and never `Holds`. With a `μ_U` evaluation refused on a joint step that enters position 2, it returns the same cause, the contract at position 1 coming first. | Test (TC-602) |
| FR-177-AC-8 | Over the leaky vault, a `μ_U` conjunct with no value on `step(1)` from joint position 1 on, a joint step that enters position 2, does not replace the empty `X` at position 1: the item returns `Violated` with `WitnessExhausted{position: 1}`. AC-6's refused `μ_E` returns `Undecided(MatchUndetermined)` although the members it drops would empty `X` at the same position. | Test (TC-602) |
| FR-177-AC-9 | AC-1's lasso is the canonical one: the stem is `step(0)` into the empty `X`, the loop is `step(0)` from that state, and since the loop's step repeats the stem's step from an equal universal state the loop entry rotates back to 0. Over the leaky vault with `step` requiring `l = 0`, an added `reset` requiring `l = 0` with postcondition `l = 1`, and `fair { weak V::Vault::reset }` on `a`, the empty state after `step(0)` has only the unfair loop `step(0)`: the lasso goes on by one connecting step to `l = 1`, where nothing is enabled, and loops on the terminal stutter, with `position: 1` and loop entry 2. | Test (TC-602) |
| FR-177-AC-10 | `Opaque` over the secure vault with the added `match` conjunct `(if present(a.step.Vault::step.i) then 1 / value(a.step.Vault::step.i) else 1) = 1` on the universal variable `a`: the item returns `Violated` with an `Undefined` counterexample at position 1 with cause `division-by-zero`, whose trace for `a` takes `step(0)` and whose trace for `b` is the first member's path, from the initial state `(1, 0)` with no step, one step short of `a`'s; `variables` and `state_keys` hold two entries. With a second universal variable `c` before `b` they hold three, in the order `a`, `c`, `b`. | Test (TC-602) |
| FR-177-AC-11 | A clause with two instances `v` and `w` that both end at the undefined `μ_E` of AC-5 at position 1 returns the first instance's counterexample. With one initial state, instance `v` is violated at position 0 by an empty `X`; instance `w` is explored through the expansion of its initial states, which needs 7 product states, so `max_states` 7 still returns `Violated` and `max_states` 6 returns `Stopped`. If instance `w` has an atom with no value at position 0, the run returns `w`'s `Undefined` counterexample at position 0, which ranks before `v`'s empty `X` at the same position. | Test (TC-602) |

## Dependencies

- ADR-023 §4 HC-2, HC-4, HC-9, HC-11; §6 HV-4, HV-8; §8.2; §5 PH-6; ADR-014 A-4
  (safety fragment); ADR-018 §3 EN-1 and CX-5.
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (SCC phase), [FR-176](FR-176-check-a-universal-hyperproperty-by-self-composition.md)
  (pre-check, scoped fairness), [FR-173](FR-173-classify-hyper-clauses-into-forms.md),
  [FR-184](FR-184-bound-hyper-runs-with-caller-budgets.md),
  [FR-101](FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md).
- [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md) (canonical
  instance order).
- QSpec owns the forms and HP-3's evidence (ADR-023 QS-3, QS-6). Its
  `undefined` kind (QSpec FR-400) ends each path at the state the evaluation
  reads: for an undefined `μ_U` on a joint step, the universal paths' state
  after the step and the existential paths' state before it, so paths of one
  `Undefined` counterexample may differ in length by that step.

## References

- ADR-023. QSpec half: QSpec FR-399 (Linear STD-136; ADR-023 QS-3, QS-6).
