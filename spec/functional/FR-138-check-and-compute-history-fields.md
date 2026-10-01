---
id: FR-138
title: "Check and compute a refinement's history fields"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-018
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: depends_on
---
# FR-138: Check and compute a refinement's history fields

## Description

QSL SHALL check a refinement's `history` rows at S3 and compute history
values along a concrete behaviour at layer 5 (ADR-020 AX-1, AX-3). A history
field belongs to the refinement only: it never enters the concrete package,
its clauses or its successor relation, and an update never removes a step.

## Use case

A specification author's abstract model keeps a value the concrete model
has overwritten, such as the previous value of a register. They add a
history field to the refinement, updated on each concrete write, and map
the abstract field from it. The concrete model and its other verdicts stay
exactly as they were.

## Inputs

- The parsed `history` rows: `history <C>::<Type>.<field>: <bounded type> =
  <literal> { on <C>::<Type>::<op>: <expression>; … }`, with spans.
- The concrete model's object types, fields and operations.
- At layer 5: a concrete state, the history values at it, and a concrete
  step (its transition identity and post-state).

## Outputs

- `CheckedRefinement.history: Vec<HistoryRow>` with
  `HistoryRow{object_type, field, declared: ValueType, initial: Literal,
  updates: Vec<(DeclarationKey, CheckedExpr)>}`.
- `HistoryValues`: for each object of each type with a history row, the
  value of each history field, sorted by object key and field.
- `fn initial_history(&CheckedRefinement, &ModelState) -> HistoryValues` and
  `fn step_history(&CheckedRefinement, &HistoryValues, pre: &ModelState,
  transition: &ModelTransition, post: &ModelState) -> Result<HistoryValues,
  MappingUndetermined>`.

## Behavior

### S3

- The object type SHALL resolve to a concrete object type, and each `on`
  operation to an operation of that type; a name that does not resolve
  SHALL refuse `missing_declaration`/`missing-name`.
- The declared type SHALL be a bounded value type (finite domain); an
  unbounded type SHALL refuse `ill_typed`/`type-mismatch` at the type.
- A history field whose name equals a declared field of the concrete type,
  or another history field of the same type, SHALL refuse
  `invalid_model_binding`/`conflicting-binding`.
- The initial literal SHALL conform to the declared type, refusing
  `ill_typed`/`type-mismatch` otherwise.
- An operation with two updates for one history field SHALL refuse
  `invalid_model_binding`/`conflicting-binding`.
- Each update SHALL be checked with `self` bound to the step's receiver
  (post-state fields), `pre(...)` of pre-state fields and of the history
  field itself, and the step's arguments by parameter name, and its type
  SHALL conform to the declared type by `TypeEnvironment::conforms`
  (ADR-016 SC-2), refusing `ill_typed`/`type-mismatch` otherwise.
- A history field SHALL be readable only inside the declaring refinement.
  A reference to it from any other clause SHALL refuse
  `missing_declaration`/`missing-name`.

### Layer 5

- `initial_history` SHALL give every object of the initial state the
  initial literal for each history field of its type.
- `step_history` SHALL, for the step's receiver, evaluate the update for the
  step's operation through the one clause evaluator (ADR-016 FE-3, FR-107)
  over the pre-state and post-state observations with a fresh meter, and
  write the value to the receiver's history field only. Every other history
  value SHALL be kept. A step whose operation has no update SHALL keep every
  history value.
- An object the step creates SHALL get the initial literal; an object the
  step deletes SHALL lose its history values.
- If an update evaluates undefined, refused or incomplete, or its value
  lies outside the declared type, then `step_history` SHALL return
  `MappingUndetermined` naming the history field, the object and the step;
  the step SHALL remain a step of the behaviour (ADR-020 AX-3).
- History values SHALL be a function of the concrete prefix, and SHALL be
  read by the mapping (FR-140) and the product key (FR-142) only.

## Acceptance Criteria

Fixture `Register` (universe `regs = {r}`). Abstract `Spec::Register`:
`value: Int[0, 1]`, `prev: Int[0, 1]`, operation `write(x: Int[0, 1])` with
frame `modifies [value, prev]` and postcondition `self.value = x and
self.prev = pre(self.value)`; initial `value` 0, `prev` 0. Concrete
`Impl::Register`: `value: Int[0, 1]`, operation `write(x: Int[0, 1])` with
frame `modifies [value]` and postcondition `self.value = x`; initial `value`
0. Refinement `RegisterHistory`: `prev = self.last`, `value = self.value`,
`history Impl::Register.last: Int[0, 1] = 0 { on Impl::Register::write:
pre(self.value); }`, `step Impl::Register::write ->
Spec::Register::write(self, x)`.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-138-AC-1 | `RegisterHistory` checks with one `HistoryRow` (`last`, `Int[0, 1]`, initial 0, one update on `write`). The concrete model's state nodes, the node identities of its clauses and the successors `ModelSystem` gives from its initial state are equal whether or not the refinement declares the history row. | Test (TC-543) |
| FR-138-AC-2 | Refusals: `history Impl::Register.value: …` (`invalid_model_binding`/`conflicting-binding`); `last: Int` with no bounds (`ill_typed`/`type-mismatch`); initial literal `true` (`ill_typed`/`type-mismatch`); two `on write` updates (`invalid_model_binding`/`conflicting-binding`); `on Impl::Register::erase` (`missing_declaration`/`missing-name`); an invariant of `Impl::Register` reading `self.last` (`missing_declaration`/`missing-name`). | Test (TC-543) |
| FR-138-AC-3 | Along `write(r, 1)`, `write(r, 0)`, `write(r, 1)` from the initial state, `step_history` gives `last` = 0, 1, 0 after each step. | Test (TC-543) |
| FR-138-AC-4 | With a second history field `writes: Int[0, 1] = 0 { on Impl::Register::write: pre(self.writes) + 1; }` added, `step_history` along `write(r, 0)`, `write(r, 0)` gives `writes` 1 after the first step and returns `MappingUndetermined` naming `writes`, `r` and the second step, whose post-state is still the concrete successor `ModelSystem` gives. | Test (TC-543) |

## Dependencies

- ADR-020 §4 AX-1 and AX-3; ADR-016 SC-2 and FE-3.
- [FR-135](FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md),
  [FR-107](FR-107-evaluate-state-clauses-at-s6a.md) (the clause evaluator),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (`ModelSystem` states and transitions).

## References

- The QSpec half (history field semantics): Linear STD-133.
