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

QSL's layer-6 crate `qsl-replay` SHALL compute `map(s, h)`, the abstract
state that a concrete state `s` with history values `h` maps to (ADR-020
RM-7). It builds each abstract population as the key-preserving image of
its source population (RM-2), gives each image object the abstract type
mapped from its concrete most-specific type (RM-3), and evaluates each
visible field row through the one clause evaluator. A row that evaluates
undefined refutes the refinement at that state; a row that is refused or
incomplete makes the mapping undetermined there (QSpec FR-377).

## Use case

The model checker, the step check and replay all need the abstract state a
concrete state stands for. They compute it the same way, from the same
rows, so a verdict and its replay never disagree about what the abstract
model saw.

## Inputs

- A `CheckedRefinement` (FR-135).
- A concrete FR-120 model state and its `HistoryValues` (FR-138).
- The concrete subject's `ModelSystem` (FR-120), which holds the concrete
  package's clause evaluator and builds the state's observation (ADR-016
  ID-10); the rows are evaluated there.
- The abstract subject's `ModelSystem` (FR-120), for the abstract state
  model the result is built in.

Each `ModelSystem` is built over one model of the unit by
`ModelSystem::over_model`, which refuses a model the unit does not select
with `invalid_runtime_input`/`wrong-role-mapping`; `map_state` takes the two
systems as given and checks no more than that.

## Outputs

```rust
pub struct MappedState {
    pub visible: ModelState,          // abstract state with hidden fields unset
    pub hidden: Vec<HiddenSlot>,      // (object key, field) left open
}

pub fn map_state(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    state: &ModelState,
    history: &HistoryValues,
) -> Result<MappedState, MappingFailure>;

pub enum MappingFailure {
    Undefined { row: MappingRowRef, object: Option<ObjectKey>, cause: UndefinedRecord },  // ADR-013 O-16
    Undetermined(MappingUndetermined),
}

pub struct MappingUndetermined {
    pub row: MappingRowRef,           // field row, argument or history update
    pub object: Option<ObjectKey>,
    pub cause: EvalFailure,
}

pub enum EvalFailure {
    Refused(RefusalRecord),           // the evaluator refused the row
    Incomplete(IncompleteCause),      // the meter ran out, or the budget admits no charge
    OutOfType,                        // the value leaves the declared type of its field
    UnmappedType(DeclarationKey),     // no object row names this concrete most-specific type
    Stopped(CatalogCode),             // the evaluator stopped on a broken invariant
}
```

`MappingFailure::Undefined` is the one arm for an undefined evaluation, and
its `cause` is the evaluator's `UndefinedRecord` (ADR-013 O-16; ADR-018
UE-2); `Undefined` is therefore not a member of `EvalFailure`, which holds
exactly the causes of `Undetermined`.

`MappingRowRef` names either a field row (the abstract object type and
field), the object row of a concrete most-specific type, an argument of a
step row, or a history update.

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
  same key. A reference nested inside an option, record, union or
  collection value SHALL be lifted by the same rule, element by element, and
  the nested value keeps its shape (the option's presence, the record's
  slots, the union's case, the collection's order).
- If a reference points at an object whose most-specific type no object row
  names, then `map_state` shall make the mapping undetermined with
  `EvalFailure::UnmappedType`.
- If a concrete object's most-specific type is one that no object row
  names, then `map_state` shall return `MappingFailure::Undetermined`
  naming `MappingRowRef::Object` for that concrete type and the object,
  with cause `EvalFailure::UnmappedType`, and shall not drop the object. S3
  requires an object row for each abstract type that a population admits
  (FR-135), not for each concrete subtype the source population may hold.
- Where an abstract field is declared optional (`?`) its row has type
  `Option<T>`: `map_state` SHALL leave the field unset when the row
  evaluates to `None`, and SHALL set it to the (lifted) value `v` when the
  row evaluates to `Some(v)`. A hidden field stays unset by `hidden`; an
  unset optional field is not hidden.
- `MappedState.visible` SHALL list no relationship extent: no refinement row
  maps a relationship (ADR-020 RM-2 to RM-4), so the mapped state carries no
  link, and a link of the abstract model never holds in a mapped state.
- If a row's evaluation is `Undefined`, then `map_state` SHALL return
  `MappingFailure::Undefined` naming the row, the object and the
  evaluator's `UndefinedRecord`, and no partial state (ADR-020 RE-5).
- If a row's evaluation is refused or incomplete, then `map_state` SHALL
  return
  `MappingFailure::Undetermined(MappingUndetermined)` naming the row and the
  object, with `EvalFailure::Refused` or `EvalFailure::Incomplete`, and no
  partial state. If a value leaves the declared type of its target (S3 has proved each
  field row conforms, so only a history update, FR-138, can) it SHALL
  return `EvalFailure::OutOfType`, and if the evaluator stops on a broken
  invariant `EvalFailure::Stopped` with its catalog code.
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
| FR-140-AC-5 | `RingIsQueue` (FR-139's fixture) at `head = 1`, `size = 2`, evaluated with a per-evaluation meter budget of zero: the `items` row evaluates `Incomplete`, and `map_state` returns `MappingFailure::Undetermined(MappingUndetermined)` naming the `items` row and object `r`, with no partial state. | Test (TC-555) |
| FR-140-AC-6 | Over a variant of `RingIsQueue` whose abstract `Queue` also declares `owners: Sequence<Reference<Queue>>[0, 2]` and `parent: Reference<Queue>?`, mapped by rows of type `Sequence<Reference<Ring>>[0, 2]` (`seq(i in 0 .. self.size - 1: self)`) and `Option<Reference<Ring>>` (`Some(self)`), the state of AC-2 maps to `owners = [r, r]` and `parent` set to the reference with key `r`, each of type `Queue`, not `Ring`; the same rows over a `Reference<Ring>` whose type no object row names return `MappingFailure::Undetermined` with cause `EvalFailure::UnmappedType`. | Test (TC-1083) |
| FR-140-AC-7 | A concrete state holding an object of a type that no object row names (a `Stray` in the source population) returns `MappingFailure::Undetermined` naming `MappingRowRef::Object` for `Stray` and that object, with cause `EvalFailure::UnmappedType(Stray)`, and no mapped state that omits it. | Test (TC-1083) |
| FR-140-AC-8 | `map_state` called with a system built by `ModelSystem::over_model` over a model the unit does not select refuses `invalid_runtime_input`/`wrong-role-mapping` at construction; with the two systems of AC-1 it reads the rows through the concrete system and builds the result in the abstract one, so the mapped state's key is the key function of the abstract system. | Test (TC-1083) |
| FR-140-AC-9 | A row evaluated with a meter budget of zero gives `EvalFailure::Incomplete`; a row whose evaluation exceeds an evaluator limit gives `EvalFailure::Refused` with the refusal record; each as `MappingFailure::Undetermined`, with no partial state. An `only` that finds two objects gives `MappingFailure::Undefined` whose `cause` is an `UndefinedRecord` with reason `only-not-one`. | Test (TC-1083) |
| FR-140-AC-10 | Over the abstract package of ADR-020 §8's `RingIsQueue` extended with a relationship between `Queue` objects, the mapped state of AC-2 lists no relationship extent, and its key equals the key of the abstract state with no link built directly in the abstract system. | Test (TC-1083) |
| FR-140-AC-11 | With the abstract `Queue` declaring `head: Int[0, 1]?` and a row of type `Option<Int[0, 1]>` that is `Some(self.head)` for `size >= 1` and `None` for `size = 0`: at `head = 1`, `size = 2` the mapped `head` is 1; at `size = 0` the mapped `head` is unset, is not in `hidden`, and the key equals the key of the abstract state with `head` unset. | Test (TC-1083) |

## Dependencies

- ADR-020 §1 RM-2, RM-3, RM-7 and §5 RE-4, RE-5; ADR-016 FE-3 and ID-10.
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md)
  (state key), [FR-107](FR-107-evaluate-state-clauses-at-s6a.md),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md),
  [FR-135](FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md),
  [FR-138](FR-138-check-and-compute-history-fields.md),
  [FR-139](FR-139-type-and-evaluate-population-valued-expressions.md).

## References

- The QSpec half (the population map and reference lifting): QSpec FR-377 (Linear STD-133).
