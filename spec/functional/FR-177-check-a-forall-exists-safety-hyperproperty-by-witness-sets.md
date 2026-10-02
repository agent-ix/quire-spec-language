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
universal cycle exists is a violation. The construction is one `WitnessSet`
component of `qsl-analyze`, which ADR-020's hidden-field refinement also
uses. It needs no Büchi complementation.

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
  an empty `X`, the first in that order SHALL be returned.
- If `μ_U` or `μ_E` evaluates refused or incomplete at a reachable joint
  step, and no refuting evidence comes first, then the engine SHALL return
  `Undecided(MatchUndetermined)` (ADR-023 HV-4).

### Violation

- `X` empty SHALL be absorbing.
- The engine SHALL run FR-126's SCC phase over the product states with `X`
  empty, with the fairness filter scoped to universal components as FR-176
  scopes it. A reachable state with `X` empty from which a cycle exists that
  is fair for every universal variable SHALL return `Violated`. With no
  universal variable, a reachable empty `X` SHALL return `Violated`.
- The counterexample SHALL be the canonical universal lasso through the
  first such state, with `position` the first joint position at which `X`
  is empty.
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

## Dependencies

- ADR-023 §4 HC-2, HC-4, HC-9, HC-11; §8.2; §5 PH-6; ADR-014 A-4 (safety
  fragment); ADR-018 §3 EN-1.
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (SCC phase), [FR-176](FR-176-check-a-universal-hyperproperty-by-self-composition.md)
  (pre-check, scoped fairness), [FR-173](FR-173-classify-hyper-clauses-into-forms.md),
  [FR-184](FR-184-bound-hyper-runs-with-caller-budgets.md),
  [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md).
- QSpec owns the forms and HP-3's evidence (ADR-023 QS-3, QS-6).

## References

- ADR-023. QSpec half: QSpec FR-399 (Linear STD-136; ADR-023 QS-3, QS-6).
