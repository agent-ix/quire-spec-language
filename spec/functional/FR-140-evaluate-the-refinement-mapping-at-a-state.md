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
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
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

`map_state` SHALL check that `concrete_system` is over the refinement's
concrete model and `abstract_system` over its abstract side, and SHALL
return `MappingFailure::WrongSystem` with the refusal
`invalid_runtime_input`/`wrong-role-mapping` when either is not (for example
the two passed swapped). That failure is not a verdict: `check_step` and the product pass the
refinement's own systems, so one arising there is an internal fault of the
calling code (FR-141). How a `ModelSystem` is built over a model, and the refusal of a
model the unit does not select, are FR-120's.

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
    WrongSystem(RefusalRecord),       // wrong systems passed; see Inputs
    Fault(CatalogCode),               // an internal fault; not a row outcome
}

pub struct MappingUndetermined {
    pub row: MappingRowRef,           // field row, argument or history update
    pub object: Option<ObjectKey>,
    pub cause: EvalFailure,
}

pub enum EvalFailure {
    Refused(RefusalRecord),           // the evaluator refused the row
    Incomplete(IncompleteCause),      // the meter ran out, or the budget admits no charge
    OutOfType,                        // a history update's value leaves its declared type
    UnmappedType(DeclarationKey),     // no object row names this concrete most-specific type
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
- A concrete reference value of top-level type `Reference<T_C>` SHALL map to
  the abstract reference with the same key. A whole-population
  reference row (`Set<Reference<T_C>>`, FR-139) SHALL map element by
  element: each concrete `Reference<T_C>` element becomes the abstract
  `Reference<T_A>` with the same key, in the set's canonical element order.
  A reference nested inside an option, record, union or collection value
  that is not such a population reference is not lifted: FR-135 refuses a
  row whose type nests one, so `map_state` never receives one.
- If a reference points at an object whose most-specific type no object row
  names, then `map_state` SHALL return `MappingFailure::Undetermined` naming
  the row being evaluated and the object that holds the reference, with
  cause `EvalFailure::UnmappedType`.
- If a concrete object's most-specific type is one that no object row
  names, then `map_state` SHALL return `MappingFailure::Undetermined`
  naming `MappingRowRef::Object` for that concrete type and the object, with
  cause `EvalFailure::UnmappedType`.
- `map_state` SHALL NOT drop an object it cannot map. S3 requires an object
  row for each abstract type that a population admits (FR-135), not for
  each concrete subtype the source population may hold.
- Where an abstract field is declared optional (`T?`, a field that may be
  absent), its row has type `Option<T>`, and `map_state` SHALL leave the
  field unset when the row evaluates to `none` and SHALL set it to `v` when
  the row evaluates to `Some(v)`. A hidden field stays in `hidden`; an unset
  optional field is not hidden. A field declared `Option<T>` (a value that
  is an option) takes the row's option value unchanged, so `none` stays an
  explicit `none` and the field is set (QSpec FR-377). No well-typed row
  evaluates to `null` (QSpec FR-143), so `map_state` has no `null` case.
- `MappedState.visible` SHALL list no relationship extent. No refinement row
  maps a relationship (ADR-020 RM-2 to RM-4), so FR-135 refuses a refinement
  whose abstract model declares a relationship (`unsupported_construct`/
  `declaration-form`), and a checked refinement's abstract model has no link
  for a mapped state to hold.
- `map_state` SHALL read the rows in a fixed order that does not depend on
  the order they were written in: the populations in ascending node-key digest order of
  the row's first named declaration, the abstract population (QSpec FR-375;
  FR-146 uses the same wording for `steps`), within each population its objects in ascending key order, and per
  object its type's object row and then that type's field rows in ascending
  field name as UTF-8 bytes (QSpec FR-377 "Failure precedence").
  `map_state` SHALL return the first `MappingFailure::Undefined` in that
  order if any row is undefined; otherwise the first
  `MappingFailure::Undetermined` in that order. An undefined row therefore
  refutes the refinement at the state even when an earlier row is
  undetermined, and the order fixes the row, object and cause that a verdict
  and its replay report.
- If a row's evaluation is `Undefined`, then `map_state` SHALL return
  `MappingFailure::Undefined` naming the row, the object and the
  evaluator's `UndefinedRecord`, and no partial state (ADR-020 RE-5).
- If a row's evaluation is refused, then `map_state` SHALL return
  `MappingFailure::Undetermined(MappingUndetermined)` naming the row and the
  object, with `EvalFailure::Refused`, and no partial state.
- If a row's evaluation is incomplete, then `map_state` SHALL return
  `MappingFailure::Undetermined(MappingUndetermined)` naming the row and the
  object, with `EvalFailure::Incomplete`, and no partial state.
- If a history update's value leaves its declared type, then
  `map_state` SHALL return `MappingFailure::Undetermined` with
  `EvalFailure::OutOfType`.
  S3 proves each field row conforms, so only a history update (FR-138) can
  reach it.
- If the evaluator hits an internal fault such as a broken invariant, then
  `map_state` SHALL return `MappingFailure::Fault` with its catalog code at
  once. The fault is not an outcome of any row and takes no part in the
  order above: the item settles `failed` (QSpec FR-383) and the failure is
  never `Undefined`, `Undetermined` or inconclusive.
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
| FR-140-AC-6 | Over a variant of `RingIsQueue` whose concrete `Ring` also has `next: Reference<Ring>` and whose abstract `Queue` has `next: Reference<Queue>` with the row `next = self.next`, and whose rings population also admits a subtype `SubRing` of `Ring` that no object row names: with ring `a` (type `Ring`) holding `next` = `q`, and `q` of type `SubRing`, `map_state` returns `MappingFailure::Undetermined` naming the `next` row and object `a`, with cause `EvalFailure::UnmappedType(SubRing)`. With `q` of type `Ring` it maps `a.next` to the `Queue` reference with key `q`. | Test (TC-1086) |
| FR-140-AC-7 | Over `CasRefinesCounter` with the concrete package also declaring a subtype `SubCounter` of `Counter` that `counters` admits and no object row names, a state whose object `c` has type `SubCounter` returns `MappingFailure::Undetermined` naming `MappingRowRef::Object` for `SubCounter` and the object `c`, with cause `EvalFailure::UnmappedType(SubCounter)`, and no mapped state that omits `c`. | Test (TC-1086) |
| FR-140-AC-8 | With the two systems of AC-1, `map_state` reads the rows through the concrete system and builds the result in the abstract one, so the mapped state's key is the abstract system's key function's. With the two systems swapped, or with a system over a third model, it returns `MappingFailure::WrongSystem` with `invalid_runtime_input`/`wrong-role-mapping` and reads no row. | Test (TC-1086) |
| FR-140-AC-9 | `RingIsQueue` with the `only` condition of AC-3: the failure's `UndefinedRecord` has reason `only-not-one`. The same variant at a state that also holds a `SubRing` object whose key sorts before `r` returns `MappingFailure::Undefined` (the undefined row outranks the unmapped type); with `RingIsQueue`'s original `items` row, which is defined at `head = 1`, `size = 2`, it returns `MappingFailure::Undetermined` for `SubRing`. The `SubRing` state is `head = 1`, `size = 2`. | Test (TC-1086) |
| FR-140-AC-10 | The mapped state of AC-2 lists no relationship extent. | Test (TC-1086) |
| FR-140-AC-11 | With the abstract `Queue` declaring `head: Int[0, 1]?` and the row `head = if self.size >= 1 then Some(self.head) else none`: at `head = 1`, `size = 2` the mapped `head` is 1; at `size = 0` the mapped `head` is unset, is not in `hidden`, and the key equals the key of the abstract state with `head` unset. With the field declared `head: Option<Int[0, 1]>` and the same row, `size = 0` maps `head` to an explicit `none` and the field is set. | Test (TC-1086) |
| FR-140-AC-12 | Over `RingIsQueue` with the abstract `Queue` also declaring `cells: Set<Reference<Cell>>[0, 2]`, an object row `Cell` from `Slot`, `population Q::cells from R::slots` and the row `cells = R::slots`, the state of AC-2 maps `cells` to the two `Cell` references with keys `s0` and `s1`, in key order, each of type `Cell`, not `Slot`. | Test (TC-1086) |
| FR-140-AC-13 | Fixture `TwoRings`: `RingIsQueue` with the `rings` universe `{a, b}` and the `slots` universe `{a0, a1, b0, b1}` (`a0`, `a1` in ring `a` with indices 0 and 1; `b0`, `b1` in ring `b` with indices 0 and 1) and the `AC-3` `only` condition (`s.ring = self`), so `items` is undefined on both rings at `size >= 1`, and `Queue` also declaring `x: Int[0, 1]` and `y: Int[0, 1]` with rows `x = g(self.head)` and `y = g(self.head)`, `g` being FR-141-AC-7's function with parameter `Int[0, 1]`, so both are undefined at `head = 0`. At the state with `head = 0` and `size = 1` on both rings (two slots satisfy `s.ring = self` per ring), `map_state` reports the undefined `items` row of object `a`, not `b`; with `items` made defined (the original `items` row) at `head = 0`, `size = 1` it reports object `a`'s field `x` before `y` (UTF-8 byte order); writing the population, object and field rows in any other source order changes neither the order nor the reported row, object or cause. | Test (TC-1086) |
| FR-140-AC-14 | A broken evaluator invariant while reading a row returns `MappingFailure::Fault` with its catalog code at once, even when an earlier row was undefined, and never `Undefined` or `Undetermined`. | Test (TC-1086) |
| FR-140-AC-15 | Fixture `TwoPopulations`: a refinement with two populations `P` and `Q` whose first named declarations (the abstract populations) have node-key digest order opposite to their `DeclarationKey` byte order, each with a row that evaluates undefined at the state. `map_state` returns the `MappingFailure::Undefined` naming the row and object of the population first in digest order, and a state with only the other population undefined returns that one. | Test (TC-545) |

## Dependencies

- ADR-020 §1 RM-2, RM-3, RM-7 and §5 RE-4, RE-5; ADR-016 FE-3 and ID-10;
  ADR-013 O-16 and ADR-018 UE-2 (the undefined cause record).
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md)
  (state key), [FR-107](FR-107-evaluate-state-clauses-at-s6a.md),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md),
  [FR-135](FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md),
  [FR-138](FR-138-check-and-compute-history-fields.md),
  [FR-139](FR-139-type-and-evaluate-population-valued-expressions.md).

## References

- The QSpec half (the population map and reference lifting): QSpec FR-377 (Linear STD-133).
