---
id: FR-140
title: "Evaluate the refinement mapping at a concrete state"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-018
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-138
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-139
    type: depends_on
---
# FR-140: Evaluate the refinement mapping at a concrete state

## Description

QSL's layer-5 `model_check` module SHALL compute `map(s, h)`, the abstract
state that a concrete state `s` with history values `h` maps to (ADR-020
RM-7). It builds each abstract population as the key-preserving image of
its source population (RM-2), gives each image object the abstract type
mapped from its concrete most-specific type (RM-3), and evaluates each
visible field row through the one clause evaluator. A row that does not
evaluate makes the mapping undetermined at that state.

## Use case

The model checker, the step check and replay all need the abstract state a
concrete state stands for. They compute it the same way, from the same
rows, so a verdict and its replay never disagree about what the abstract
model saw.

## Inputs

- A `CheckedRefinement` (FR-135).
- A concrete FR-120 model state and its `HistoryValues` (FR-138).
- The abstract subject's `ModelSystem` (FR-120), for the abstract state
  model the result is built in.

## Outputs

```rust
pub struct MappedState {
    pub visible: ModelState,          // abstract state with hidden fields unset
    pub hidden: Vec<HiddenSlot>,      // (object key, field) left open
}

pub fn map_state(
    refinement: &CheckedRefinement,
    abstract_system: &ModelSystem<'_>,
    state: &ModelState,
    history: &HistoryValues,
) -> Result<MappedState, MappingFailure>;

pub enum MappingFailure {
    Undefined { row: MappingRowRef, object: Option<ObjectKey>, cause: UndefinedRecord },
    Undetermined(MappingUndetermined),
}

pub struct MappingUndetermined {
    pub row: MappingRowRef,           // field row, argument or history update
    pub object: Option<ObjectKey>,
    pub cause: EvalFailure,           // refused, incomplete, out of type
}
```

## Behavior

- For each population row, `map_state` SHALL create, for each object with
  key `k` in the concrete source population, one abstract object with key
  `k`, and no other abstract object of that population.
- Each abstract object's most-specific type SHALL be the abstract type whose
  object row names the concrete object's most-specific type.
- Each visible field SHALL be the value of its row's expression, evaluated
  through the one clause evaluator (ADR-016 FE-3, FR-107) as a value-typed
  body over `s`'s synthesized observation (ADR-016 ID-10) with `self` bound
  to the concrete object, history fields read from `h`, and a fresh meter
  per evaluation.
- A concrete reference value SHALL map to the abstract reference with the
  same key.
- If a row's evaluation is `Undefined`, then `map_state` SHALL return
  `MappingFailure::Undefined` naming the row, the object and the
  evaluator's undefined cause, and no partial state (ADR-020 RE-5).
- If a row's evaluation is refused or incomplete, or its value lies outside
  the abstract field's declared type, then `map_state` SHALL return
  `MappingFailure::Undetermined(MappingUndetermined)` naming the row and the
  object, and no partial state.
- When the refinement has no hidden field, `map_state` SHALL return a
  complete abstract state in `MappedState.visible`.
- `model_check` SHALL compare mapped states by the FR-101 state key that the
  abstract `ModelSystem`'s key function computes, so "equal mapped states"
  means equal keys.
- When it has hidden fields, `MappedState.hidden` SHALL list them; FR-142
  completes them from its set of consistent abstract states.
- `map_state` SHALL depend on its inputs only, reading no path, clock or
  environment variable.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-140-AC-1 | Over ADR-020 §8's `CasRefinesCounter`, the concrete state `(1, 0, f, 0, t)` maps to the abstract state with `c.value = 1`, whose key equals the key of that state built directly in `Spec`'s `ModelSystem`. `(0, 0, t, 0, f)` and `(0, 0, f, 0, f)` map to equal keys. | Test (TC-545) |
| FR-140-AC-2 | Over `RingIsQueue` (FR-139's fixture) at `head = 1`, `size = 2`, `s0.value = 0`, `s1.value = 1`, the mapped state holds one `Queue` with key `r` and `items = [1, 0]`, and no object for `s0` or `s1`. | Test (TC-545) |
| FR-140-AC-3 | `RingIsQueue` with the `items` row's `only` condition changed to `s.ring = self` returns `MappingFailure::Undefined` naming the `items` row, object `r` and the evaluator's undefined cause, at every state with `size >= 1`. | Test (TC-545) |
| FR-140-AC-4 | Over `RegisterHistory` (FR-138's fixture), the concrete state `value = 1` with `last = 0` maps to `value = 1`, `prev = 0`. With the `prev` row removed, the same state maps to `visible` with `value = 1` and `hidden = [(r, prev)]`. | Test (TC-545) |

## Dependencies

- ADR-020 §1 RM-2, RM-3, RM-7 and §5 RE-4, RE-5; ADR-016 FE-3 and ID-10.
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md)
  (state key), [FR-107](FR-107-evaluate-state-clauses-at-s6a.md),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md),
  [FR-135](FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md),
  [FR-138](FR-138-check-and-compute-history-fields.md),
  [FR-139](FR-139-type-and-evaluate-population-valued-expressions.md).

## References

- The QSpec half (the population map and reference lifting): Linear
  STD-133.
