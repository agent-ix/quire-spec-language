---
id: FR-179
title: "Check a step relation over a model's reachable transitions (EN-1)"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-021
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-173
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-176
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-184
    type: depends_on
---
# FR-179: Check a step relation over a model's reachable transitions (EN-1)

## Description

QSL's layer-5 `model_check` SHALL decide the model claim of an HP-1 step
relation (ADR-023 HM-9, HC-3) by exploring each subject's reachable graph
once, retaining edges, and evaluating the relation's body over every tuple
of reachable transitions, one per execution variable, each labelled with
that variable's operation. The first tuple, in canonical order, on which
the body is false is the counterexample. `max_relation_tuples` bounds the
number of tuples a run evaluates (FR-184).

## Use case

A verification operator states that a vault's `step` is deterministic: two
executions from equal pre-states with equal inputs give equal post-states.
The engine evaluates the relation on every pair of reachable `step`
transitions and either proves it or returns the first pair that differs,
each with a path from an initial state to its pre-state.

## Inputs

- A `ModelCheckRequest` (FR-126) whose item is an HP-1 clause (FR-173), with
  one subject per alias and `ModelCheckLimits` (FR-184), after FR-176's
  pre-check.

## Outputs

- FR-126's `ModelCheckOutcome`, with `Violated` carrying a
  `HyperCounterexample` of kind `StepTuple` (FR-183), and with
  `Undefined { tuple, cause }` added: the first undefined tuple in canonical
  order, as a `StepTuple` of its executions, and the undefined cause the
  evaluator gave for it (ADR-013 O-16 category undefined).

## Behavior

- The engine SHALL explore each subject's reachable graph once by FR-101's
  canonical breadth-first engine and retain every edge, with its
  pre-state, transition identity (operation, receiver, arguments) and
  post-state, and its result.
- For each tuple of retained edges whose `i`-th edge is labelled with
  execution variable `i`'s operation, taken in the product of the canonical
  edge orders (FR-101) in variable order, the engine SHALL evaluate the body
  through the one clause evaluator (FR-107) over the executions'
  observations: arguments, the receiver's pre-state, post-state and result.
- The first tuple on which the body evaluates `false` SHALL return
  `Violated`, each execution carrying the canonical breadth-first path from
  an initial state to its pre-state followed by its transition.
- If the body evaluates to no value on a tuple, then the engine SHALL record
  that tuple as undefined, with the evaluator's undefined cause, and
  continue the enumeration. An undefined tuple SHALL count neither as a pass
  nor as a refutation.
- A refuting tuple SHALL return `Violated` whether or not other tuples are
  undefined.
- When the enumeration ends with no refuting tuple and at least one
  undefined tuple, the engine SHALL return `Undefined` naming the first
  undefined tuple in canonical order and its cause.
- When every tuple evaluated `true`, the engine SHALL return
  `Holds{Exhaustive}`.
- When the next tuple would pass `max_relation_tuples`, the engine SHALL
  stop and return `Stopped(ResourceExhausted, MaxRelationTuples)`.
- The engine SHALL enumerate only transitions that leave states at depth
  below `max_depth`.
- When a run reaches `max_depth` with no `false` tuple, the engine SHALL
  return `BoundReached{depth: max_depth}` (ADR-023 HV-3).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-179-AC-1 | FR-171-AC-3's `Det` over ADR-023 §8's secure vault returns `Holds{Exhaustive}` after evaluating every pair of the 8 reachable `step` edges (64 tuples). | Test (TC-604) |
| FR-179-AC-2 | `Det` over a vault variant whose `step` postcondition is `self.l = i or self.l = 1 - i` (a nondeterministic post-state) returns `Violated` with a `StepTuple` counterexample: two executions of `step` with equal pre-states and inputs and unequal post-states, each with a path from an initial state to its pre-state. | Test (TC-604) |
| FR-179-AC-3 | AC-1's request with `max_relation_tuples` 10 returns `Stopped(ResourceExhausted, MaxRelationTuples)` after 10 tuples, naming the limit and its value; with `max_depth` 1 it enumerates only the 4 transitions from the two initial states (16 tuples) and returns `BoundReached{depth: 1}`. Running AC-2's request twice gives byte-equal counterexamples. | Test (TC-604) |
| FR-179-AC-4 | Over a `Cell` model (field `d: Int[0, 1]`, operation `set(k: Int[0, 1])` with postcondition `self.d = k`, initial `d = 0`), `relation R using v over (x: C::Cell::set, y: C::Cell::set) { 1 / x.k >= y.k }` evaluates every tuple with `x.k = 1` `true` and every tuple with `x.k = 0` undefined, and returns `Undefined` naming the first tuple in canonical order with `x.k = 0` and the evaluator's division-by-zero cause; it never returns `Holds`. | Test (TC-610) |
| FR-179-AC-5 | Over the same `Cell` model, `relation R2 using v over (x: C::Cell::set, y: C::Cell::set) { 1 / x.k > y.k }`, whose tuples with `x.k = 0` are undefined and whose tuple with `x.k = 1` and `y.k = 1` is false, returns `Violated` with a `StepTuple` counterexample on that refuting tuple. | Test (TC-611) |

## Dependencies

- ADR-023 §2 HM-9, §4 HC-3, HC-9, HC-10; HV-3; §12 RU-2.
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md),
  [FR-107](FR-107-evaluate-state-clauses-at-s6a.md),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md),
  [FR-173](FR-173-classify-hyper-clauses-into-forms.md),
  [FR-176](FR-176-check-a-universal-hyperproperty-by-self-composition.md)
  (pre-check), [FR-184](FR-184-bound-hyper-runs-with-caller-budgets.md).
- FR-180 hands over the code claim of the same relation.
- QSpec owns the step relation's meaning (ADR-023 QS-2) and the `StepTuple`
  counterexample (QS-6).

## References

- ADR-023. QSpec half: Linear STD-136 (ADR-023 QS-2, QS-6).
