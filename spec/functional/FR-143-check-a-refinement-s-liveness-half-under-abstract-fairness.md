---
id: FR-143
title: "Check a refinement's liveness half under abstract fairness"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-018
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-137
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-141
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-142
    type: depends_on
---
# FR-143: Check a refinement's liveness half under abstract fairness

## Description

When the abstract fairness set `F_A` is non-empty and the safety half
holds, QSL's layer-5 `model_check` module SHALL decide the liveness half of
a refinement in EN-1's second phase (ADR-020 RS-7, RS-10, RS-13, CO-2,
RE-1): every behaviour of the concrete subject that is fair under `F_C`
satisfies `F_A` on its mapped behaviour. It decomposes FR-142's retained
product graph into SCCs and looks for a fair cycle on which some `F_A`
constraint is enabled at every mapped state and taken by no step. A
refinement with hidden fields and a non-empty `F_A` settles the liveness
half `unsupported` with its cause named (ADR-020 AX-5).

## Use case

A specification author states that the abstract increment is weakly fair.
A concrete model that can loop forever on read-only steps, or halt, while
the abstract counter can still move, is refuted with a lasso that names the
abstract operation it never delivers. Without the `ensure` row the same
model is a valid safety refinement.

## Inputs

- FR-142's retained product graph, with each edge's `check_step` result
  (`Taken`) and a safety half that returned `Holds{Exhaustive}`.
- `F_C` and `F_A` from FR-137, including the scheduler constraints over a
  concrete protocol subject.
- The abstract subject's `ModelSystem`, for enabledness at mapped states.

## Outputs

```rust
pub enum LivenessHalf {
    Checked(ModelCheckOutcome),          // FR-126's outcome type
    Unsupported(UnsupportedCause),       // AX-5: liveness through hidden abstract fields
}
```

A `Violated` liveness outcome carries a lasso `TemporalCounterexample` over
the concrete subject with failure `Divergence{constraint}` or
`AbstractUnfair{constraint}`.

## Behavior

### When the half runs

- With `F_A` empty, `RefinementOutcome.liveness` SHALL be `None`, whatever
  the concrete model does (ADR-020 RS-7).
- With `F_A` non-empty and a hidden field, it SHALL be
  `Unsupported(unsupported-requested-capability)` naming liveness through
  hidden abstract fields as the cause, and no second phase SHALL run.
- With `F_A` non-empty, no hidden field and a safety half other than
  `Holds`, the second phase SHALL NOT run and `liveness` SHALL be `None`.

### Enabled and taken through the mapping (CO-2)

- An `F_A` class SHALL be one abstract operation for a `whole` constraint
  and one abstract transition identity for each class of an `each`
  constraint, as ADR-018 FA-1 states.
- A class SHALL be **enabled** at a product state when the abstract
  subject's `ModelSystem` gives a successor with an identity of the class
  from the state's mapped abstract state.
- A class SHALL be **taken** by a product edge when that edge's `check_step`
  result is `Taken::Abstract` listing an identity of the class. A
  `Taken::Stutter` edge, including the concrete terminal stutter edge
  (ADR-020 RS-6), SHALL take no class.

### Second phase

- For each `F_A` class `k`, in `F_A`'s source order and then canonical
  identity order, the checker SHALL decompose, in discovery order, the
  subgraph of the retained graph whose states enable `k` and whose edges do
  not take `k`.
- A non-trivial SCC of that subgraph SHALL violate `k` when it passes the
  `F_C` filter: for every `F_C` constraint it holds an edge taking the
  constraint or a state where the constraint is not enabled, by FR-126's
  filter function over the concrete subject (over a protocol subject,
  ADR-027 PA-1's classes and PA-2's enabledness, scheduler constraints
  included).
- The first violating SCC SHALL return `Violated` with FR-126's canonical
  lasso into it. Its failure SHALL be `Divergence{constraint}` when every
  loop edge is `Taken::Stutter`, and `AbstractUnfair{constraint}`
  otherwise, naming the `F_A` constraint of `k`.
- No violating SCC for any class SHALL return `Holds{basis: Exhaustive}`.
- A concrete terminal state SHALL be read through its terminal stutter
  edge, so it violates `k` exactly when `k` is enabled at its mapped state
  (ADR-020 RS-10). Neither model's `terminal` member SHALL change this
  reading.
- The phase SHALL respect FR-126's limits and poll and return `Stopped`
  with the limit reached.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-143-AC-1 | ADR-020 §8's `CasRefinesCounter` returns liveness `Checked(Holds{Exhaustive})`. The divergence variant (`assume` rows only for `commitA` and `commitB`, each `each`) returns `Checked(Violated)` with an empty stem, a loop of one `peek` step at `(0, 0, f, 0, f)` and failure `Divergence{constraint: inc}`; the same variant with its `ensure` row removed returns `liveness: None` and safety `Holds{Exhaustive}`. | Test (TC-548) |
| FR-143-AC-2 | Fixture `Halting`: abstract `Spec::Counter` of ADR-020 §8 with `ensure fair weak Spec::Counter::inc`; concrete `Impl::Counter` with `value: Int[0, 2]` and `inc()` with precondition `self.value < 1`, `value = self.value`, `step Impl::Counter::inc -> Spec::Counter::inc(self)`, `assume fair weak Impl::Counter::inc`. It returns `Checked(Violated)` with stem `inc`, a loop that is the terminal stutter at `value` 1 with the stutter marker set, and `Divergence{constraint: inc}`. With the `ensure` row removed, `liveness` is `None`. With the concrete precondition `self.value < 2`, it returns `Checked(Holds{Exhaustive})`. Each of these three verdicts is the same under every combination of `terminal` members on the two models, each model taking none, `terminal when self.value = 2` or `terminal any`. | Test (TC-548) |
| FR-143-AC-3 | Fixture `Pair`: abstract and concrete `Pair` with `a: Int[0, 1]`, `b: Int[0, 1]`, operations `flipA()` and `flipB()` with no precondition (each negates its field), both mapped by name, `assume fair weak each Impl::Pair::flipA`, `ensure fair weak each Spec::Pair::flipB`. It returns `Checked(Violated)` with an empty stem, a loop of two `flipA` steps and `AbstractUnfair{constraint: flipB}`. Adding `assume fair weak each Impl::Pair::flipB` makes it `Checked(Holds{Exhaustive})`. | Test (TC-548) |
| FR-143-AC-4 | `RegisterHistory` (FR-138) with the `prev` row removed and `ensure fair weak Spec::Register::write` returns safety `Holds{Exhaustive}` and liveness `Unsupported(unsupported-requested-capability)` naming liveness through hidden abstract fields; without the `ensure` row, `liveness` is `None`. | Test (TC-548) |
| FR-143-AC-5 | Over a concrete protocol subject, an SCC in which branch `B`'s step class is enabled at every state and taken by no edge fails the `F_C` filter through `B`'s `Scheduler` constraint (FR-137-AC-3) and so violates no `F_A` class; with `scheduling adversarial` on the protocol the same SCC passes the filter and is returned as the violation. | Test (TC-548) |

## Dependencies

- ADR-020 §2 RS-7, RS-10, RS-12, RS-13, §3 CO-2, §4 AX-5, §5 RE-1, §6 RC-1
  and §11 RU-2, RU-4; ADR-018 §4 FA-1 to FA-4; ADR-027 PA-1 to PA-3.
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (the fairness filter, canonical lasso and limits),
  [FR-137](FR-137-check-a-refinement-s-fairness-rows.md),
  [FR-141](FR-141-decide-one-concrete-step-against-the-abstract-model.md),
  [FR-142](FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md).

## References

- The QSpec half (abstract progress through `F_A`, RS-7): QSpec FR-377 (Linear STD-133).
- Liveness refinement with hidden abstract fields, explored later: Linear
  QSL-380.
