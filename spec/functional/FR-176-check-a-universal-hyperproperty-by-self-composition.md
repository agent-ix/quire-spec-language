---
id: FR-176
title: "Check a universal hyperproperty by self-composition (EN-1)"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-021
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-173
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-175
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-184
    type: depends_on
---
# FR-176: Check a universal hyperproperty by self-composition (EN-1)

## Description

QSL's layer-A `qsl-analyze` engine EN-1 (ADR-029 CB-3) SHALL decide an HP-2 item by exploring a
`HyperProduct` (ADR-023 HC-1): the tuple of component model states, one per
universal variable, with the state of the generalized Büchi automaton for
the negated body, stepping only on joint steps where `μ_U` holds. It reuses
FR-126's first phase, SCC phase, fairness filter and lasso, with every
fairness constraint scoped to its own component (ADR-023 HC-4). It runs at
S6c over E10. The engine also performs the pre-check shared by every hyper
form (ADR-023 HC-8) and detects a vacuous match (ADR-023 HM-7).

## Use case

A verification operator requests noninterference over a vault model. The
engine builds the pair product of the model with itself, steps both copies
only on equal inputs, and either proves the clause over every matched pair
of fair behaviours or returns the canonical pair of lassos on which the
public outputs differ.

## Inputs

- A `ModelCheckRequest` (FR-126) whose item is an HP-2 clause (FR-173), with
  one model subject per alias and `ModelCheckLimits` (FR-184).

## Outputs

- FR-126's `ModelCheckOutcome`, with `Violated` carrying a
  `HyperCounterexample` of kind `Lockstep` (FR-183), and with
  `Undecided(MatchUndetermined)` and `Undecided(VacuousMatch)` added
  (FR-182).

## Behavior

### Pre-check (every hyper form)

- Before building any product, the engine SHALL classify every root of
  every subject as FR-126's pre-check does, and explore each subject's
  reachable graph once under the request's limits. A subject that alone
  reaches a limit SHALL stop the item V-7 before the product is built,
  naming the limit.

### Product

- The product state SHALL be the tuple of component states in quantifier
  order with an automaton state, keyed by the component FR-101 state keys
  in quantifier order and the automaton state index.
- The initial states SHALL be every tuple of initial states, each component
  from its own subject's initial list, with the automaton reading joint
  position 0. The same subject twice SHALL give every ordered pair, a
  state's pair with itself included.
- The successors SHALL be every tuple of component steps, each an FR-120
  transition enabled at its component or the stutter step at a terminal
  component, on which `μ_U` holds, each with every joint post-state and
  automaton successor. A joint step's identity is the tuple of component
  identities.
- `μ_U` SHALL be evaluated through the one clause evaluator (FR-107) over
  the joint step's labels. If it evaluates `Undefined` at a reachable
  joint step, then the engine SHALL return `Violated` with QSpec FR-400's
  `Undefined` counterexample ending at that joint step, one path per trace
  variable (ADR-023 HV-8). If it evaluates refused or incomplete there, then the engine SHALL
  return `Undecided(MatchUndetermined)`.

### Phases

- The engine SHALL explore the product with FR-101's canonical
  breadth-first engine, retaining every edge, and then run FR-126's SCC
  phase for every body: for a safety body the automaton's rejecting monitor
  state is an accepting sink with a self-loop, so a violation is a fair
  accepting cycle through it.
- The fairness filter SHALL read a constraint of variable `k` as enabled and
  taken on component `k` of the joint state and joint step, and is
  otherwise FR-126's.
- A passing candidate SHALL return `Violated` with the canonical lasso split
  into its components (FR-126's canonical stem and loop over the product).
- No passing candidate SHALL return `Holds{Exhaustive}`, or
  `Holds{Reduced{…}}` under FR-174.

### Vacuity

- When `μ_U` is non-empty and no tuple of fair universal behaviours matches
  it at every joint step, the engine SHALL return `Undecided(VacuousMatch)`
  and never `Holds` (ADR-023 HM-7): the explored product has no fair cycle
  under the scoped fairness filter, the stutter loop at terminal components
  included.

### Limits

- `max_states` SHALL count product states, `max_transitions` joint steps
  and `max_automaton_states` automaton states (FR-184).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-176-AC-1 | ADR-023 §8.1, no reduction: the leaky product has 14 reachable states and two accepting SCCs, and returns `Violated` with a `Lockstep` counterexample of the §8.1 table's shape, `a` from `(0, 0)`, `b` from `(1, 0)`, joint steps `(step(0), step(0))`, loop entry 1. The secure product has 8 states and returns `Holds{Exhaustive}`. | Test (TC-601) |
| FR-176-AC-2 | `NonInterference` with its `match` block removed returns `Violated` over the secure model. `match { a.step.op = V::Vault::step and b.step.op = stutter }` over the vault, which has no terminal state, returns `Undecided(VacuousMatch)`. | Test (TC-601) |
| FR-176-AC-3 | The secure vault with an added `reset` operation (no precondition, postcondition `self.l = 0`, frame `[l]`) and the one-variable clause `forall trace a of V { always eventually holds(v.l @ a = 0) }`: under `fair { weak V::Vault::reset }` on `a` it returns `Holds`, and with the empty fairness set `Violated` with a loop of `step(1)` steps. In a two-variable clause over a subject where `a` can reach a terminal state, `μ` `a.step.op = b.step.op` pairs `a`'s stutter step only with `b`'s stutter step. | Test (TC-601) |
| FR-176-AC-4 | A subject that alone exceeds `max_states` stops the item `Stopped(ResourceExhausted, MaxStates)` before the product is built. `NonInterference` over the secure vault with the added `match` conjunct `(if present(a.step.Vault::step.i) then 1 / value(a.step.Vault::step.i) else 1) = 1`, which has no value on a joint step whose `a` step is `step(0)`, returns `Violated` with an `Undefined` counterexample ending at the first such joint step in canonical order, with a path for `a` and for `b` and cause `division-by-zero`. Running AC-1's leaky request twice gives byte-equal counterexamples. | Test (TC-601) |

## Dependencies

- ADR-023 §4 HC-1, HC-4, HC-5, HC-8, HC-9; §2 HM-3, HM-5, HM-7; §4.2
  (MCHyper's method); ADR-018 §3 EN-1, FA-4, CX-5; ADR-011 §1 (S6c, E10).
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (EN-1's phases, fairness filter and canonical lasso),
  [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md),
  [FR-173](FR-173-classify-hyper-clauses-into-forms.md),
  [FR-175](FR-175-evaluate-a-body-over-a-tuple-of-behaviours.md),
  [FR-184](FR-184-bound-hyper-runs-with-caller-budgets.md).
- EN-2 refutation and EN-3 induction of HP-2 are the SMT backend's on the
  IR side (ADR-023 HC-6, HC-7, DS-3); FR-182 settles their outcomes.
- QSpec owns the `μ_U`/`μ_E` relativisation and vacuity (ADR-023 QS-2).

## References

- ADR-023. QSpec half: QSpec FR-395 and FR-399 (Linear STD-136; ADR-023 QS-2, QS-9).
