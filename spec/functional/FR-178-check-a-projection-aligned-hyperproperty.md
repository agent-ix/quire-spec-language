---
id: FR-178
title: "Check a projection-aligned universal hyperproperty (EN-1)"
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
# FR-178: Check a projection-aligned universal hyperproperty (EN-1)

## Description

QSL's layer-5 `model_check` SHALL decide an HP-6 item, a universal clause
with a safety body and `align skip { O, … }`, over its projected product
(ADR-023 PA-2 to PA-6): components take skipped steps one at a time, take
visible steps together when `μ_U` holds, and the body's safety monitor reads
only joint visible positions. A reachable rejecting monitor state is a
violation, found in EN-1's first phase. The layer-5 evaluator SHALL evaluate
a body three-valued over a joint projected prefix.

## Use case

A verification operator checks noninterference on a vault where a secret of
1 adds an internal `mix` step after each `step`. Lockstep pairs one run's
`step` with the other's `mix` and reports a leak no observer can see. With
`align skip { V::Vault::mix }`, the runs align on their `step`s and the
clause is proved.

## Inputs

- A `ModelCheckRequest` (FR-126) whose item is an HP-6 clause (FR-173), with
  one subject per alias and `ModelCheckLimits` (FR-184), after FR-176's
  pre-check.

## Outputs

- FR-126's `ModelCheckOutcome`, with `Violated` carrying a
  `HyperCounterexample` of kind `Projected` (FR-183).
- From the evaluator: the three-valued value of a body over a joint
  projected prefix.

## Behavior

### Projection

- A step SHALL be **skipped** when its operation is in the clause's `align
  skip` list and **visible** otherwise.
- The projection of a finite path SHALL be its position 0 followed by each
  position a visible step enters; projected position `j` is the `j`-th of
  them. At a projected position an atom SHALL read the state the visible
  step entered and `μ` that step's label. Interval operators SHALL count
  projected positions.

### Projected product

- The product state SHALL be the tuple of component states with the body's
  deterministic safety monitor state (ADR-018 SM-6), keyed as FR-176 keys a
  state. The initial states SHALL be FR-176's, the monitor reading joint
  position 0.
- A **skip move** SHALL let one component take an enabled skipped step while
  every other component and the monitor stay.
- A **visible move** SHALL let every component take an enabled visible step
  together when `μ_U` holds on the joint step, the monitor reading the joint
  post-position.
- The engine SHALL explore the product with FR-101's canonical breadth-first
  engine. A reachable rejecting monitor state SHALL return `Violated`, the
  counterexample being the first path in breadth-first order to it, split
  into its components' steps with each step marked skipped or visible.
- Exploration that completes with no rejecting state SHALL return
  `Holds{Exhaustive}`, or `Holds{Reduced{…}}` under FR-174.
- `max_depth` SHALL count moves of either kind; `max_states` product states;
  `max_transitions` moves.

### Three-valued prefix evaluation

- The evaluator SHALL evaluate a safety body over a tuple of finite
  projected traces of equal projected length by ADR-014 A-4's three-valued
  reading, returning `false` exactly when the joint projected prefix is a
  bad prefix of the body.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-178-AC-1 | ADR-023 §13's vault (`busy`, `mix`, `step` setting `l = i` and `busy` to `h = 1`), with `μ` matching the arguments of two `step` steps: the lockstep clause (HP-2) returns `Violated`; with `align skip { V::Vault::mix }` (HP-6) it returns `Holds{Exhaustive}`. | Test (TC-603) |
| FR-178-AC-2 | §13's vault with the leaky post clause `l = (i + h) mod 2` and `align skip { V::Vault::mix }` returns `Violated` with a `Projected` counterexample whose traces have equal visible step counts, unequal total lengths, no loop entry, and each step marked skipped or visible. | Test (TC-603) |
| FR-178-AC-3 | Three-valued evaluation of `always holds(v.l @ a = v.l @ b)` over a joint projected prefix whose last projected position has unequal `l` returns `false`; over one with equal `l` throughout it returns no `false`. | Test (TC-603) |
| FR-178-AC-4 | AC-2's request with `max_depth` 1 returns `BoundReached{depth: 1}`, counting a skip move as one move. Running AC-2's request twice gives byte-equal counterexamples. | Test (TC-603) |

## Dependencies

- ADR-023 §13 PA-2 to PA-6, §12 RU-1, §5 PH-4; ADR-014 A-4; ADR-018 SM-6.
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md),
  [FR-176](FR-176-check-a-universal-hyperproperty-by-self-composition.md)
  (pre-check, product keys), [FR-173](FR-173-classify-hyper-clauses-into-forms.md),
  [FR-184](FR-184-bound-hyper-runs-with-caller-budgets.md),
  [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md).
- QSpec owns projection alignment and its prefix reading (ADR-023 QS-2) and
  the `Projected` counterexample (QS-6).

## References

- ADR-023. QSpec half: Linear STD-136 (ADR-023 QS-2, QS-6).
