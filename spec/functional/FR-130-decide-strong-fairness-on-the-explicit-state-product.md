---
id: FR-130
title: "Decide strong fairness on the explicit-state product by SCC refinement"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-016
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-019
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-129
    type: depends_on
---
# FR-130: Decide strong fairness on the explicit-state product by SCC refinement

## Description

QSL's layer-5 `model_check` SHALL decide a liveness (TP-4) item whose
fairness set holds strong constraints by recursive SCC refinement of the
retained product graph (ADR-019 SR-1 to SR-7). It extends FR-126's second
phase: the fairness filter becomes one recursive function over a set of
product states and the fairness set, with a rule per `FairnessKind`. With
weak constraints only it is FR-126's single pass. The first phase, the
outcome type and the counterexample type are FR-126's.

## Use case

A verification operator checks that process 1 eventually holds a contended
lock, under strong fairness of `acquire` for each process. The engine
examines every reachable product state, removes from consideration the
states where a starved acquisition is enabled, and either proves the claim
over the subject or returns the canonical lasso that is fair under every
constraint.

## Inputs

- FR-126's `ModelCheckRequest` for a `Liveness` clause whose fairness set
  (FR-129) holds one or more `FairnessKind::Strong` constraints.
- The retained product graph of FR-126's first phase.

## Outputs

- FR-126's `ModelCheckOutcome`: `Holds{basis: Exhaustive}`, `Violated` with
  the canonical lasso, `BoundReached`, `Undecided` or `Stopped`.

## Behavior

### Enabled sets

- When the first phase expands a model state, the engine SHALL record the
  set of transition identities enabled there, which are the labels of the
  state's FR-120 transitions, at no extra contract evaluation (ADR-019
  SR-1).
- The engine SHALL read whether a constraint is enabled at a product state
  from its model state's enabled set, never from the product's edges, for
  weak and strong constraints alike, so an identity the automaton gives no
  move is still enabled (ADR-019 SF-2, AM-2).
- A strong constraint SHALL be taken by an edge whose transition identity
  belongs to it; a `Whole` constraint is enabled when any of its identities
  is; the terminal stutter edge belongs to no constraint and enables none.

### The filter

- The engine SHALL compute `fair(C)` over a set of product states `C` and
  the fairness set: decompose the subgraph induced by `C` into SCCs in
  discovery order, and test each non-trivial SCC `S` (one with an edge) for
  (a) a state of every acceptance set of the automaton, (b) for every weak
  constraint, an edge in `S` that takes it or a state of `S` where it is
  disabled, and (c) for every strong constraint, an edge in `S` that takes
  it, or no state of `S` where it is enabled (ADR-019 SR-2).
- When `S` meets (a), (b) and (c), the engine SHALL pass it.
- When `S` fails (a) or (b), the engine SHALL reject it.
- When `S` meets (a) and (b) and fails (c) for a set of strong constraints
  `B`, the engine SHALL remove from `S` the states `R` at which some member
  of `B` is enabled and recurse with `fair(S \ R)`.
- The engine SHALL run the second phase as `fair` over every retained
  product state, and SHALL take as the passing component the first passing
  SCC in discovery order, depth-first through the refinement (ADR-019
  SR-6).
- The engine SHALL consult the poll before each SCC decomposition of the
  second phase, and SHALL return `Stopped{cause: Cancelled, limit: None}`
  when it returns `true`.

### The counterexample

- The canonical stem SHALL be the first path in FR-101 canonical
  breadth-first order from an initial product state to the passing
  component.
- The canonical loop SHALL be ADR-018 CX-5's greedy walk over the passing
  component `P` from its entry state, in which a strong constraint enabled
  at some state of `P` has an obligation discharged only by an edge of `P`
  that takes it, and a strong constraint enabled at no state of `P` has
  none (ADR-019 SR-7).
- The engine SHALL return `Violated` with that lasso when a component
  passes, and FR-126's `Holds{basis: Exhaustive}` or `BoundReached` when
  none does.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-130-AC-1 | ADR-019 §6's mutex (universe `{m}`, owner 0), `always eventually holds(m.owner = 1)`: under `weak each` on `acquire` it returns `Violated` with an empty stem and the loop `0 -acq(2)-> 2 -rel-> 0`; under `strong each` it returns `Holds{Exhaustive}` over 5 product states, although the product has no `acq(1)` edge out of `(0, q1)`; under `strong` with no granularity it returns `Violated` with the same loop as under `weak each`. | Test (TC-531) |
| FR-130-AC-2 | The `Handoff` mutex (owner `Int[0, 3]`, `acquire(p: Int[1, 3])` with precondition `self.owner = 0`, `release` with precondition `self.owner != 0`, and `pass` with precondition `self.owner = 2 or self.owner = 3` setting `owner` to `5 - pre(self.owner)`), same claim under `strong each` on `acquire`: the accepting SCC `{(0, q1), (2, q1), (3, q1)}` fails (c) for `acq(1)`, its refinement removes `(0, q1)`, and the remainder `{(2, q1), (3, q1)}` passes, so the engine returns `Violated` with the stem `0 -acq(2)-> 2` and the loop `2 -pass-> 3 -pass-> 2`. | Test (TC-531) |
| FR-130-AC-3 | Over the mutex under `strong each` on `acquire`, a poll that returns `true` from the first call the second phase makes returns `Stopped{Cancelled, None}`. The same claim with the fairness set `weak each` on `acquire` and `strong each` on `acquire` returns the `strong each` verdict, `Holds{Exhaustive}`. Each AC-1 and AC-2 request run twice gives equal outcomes and byte-equal counterexamples. | Test (TC-531) |

## Dependencies

- ADR-019 §2 SF-2 to SF-4, §3 SR-1 to SR-7, AM-1 and AM-2; ADR-018 §3 EN-1,
  §4 FA-4, FA-5, §5 CX-5.
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (the engine it extends), [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (transitions and enabledness), [FR-129](FR-129-check-strong-fairness-constraints.md)
  (the fairness set).

## References

- QSpec FR-362 (strong fairness semantics) and FR-365 (the canonical
  counterexample): the QSpec half of ADR-019 (Linear STD-132).
- E. A. Emerson and C.-L. Lei, 1987, and T. Latvala and K. Heljanko, 2000,
  as ADR-019 cites them.
