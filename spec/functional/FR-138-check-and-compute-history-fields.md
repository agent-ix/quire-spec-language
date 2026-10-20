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
values along a concrete behaviour in layer 6, crate `qsl-replay`, where
`check_step` and the certificate checker (FR-149) read them (ADR-020 AX-1,
AX-3). A history
field belongs to the refinement only: it never enters the concrete package,
its clauses or its successor relation, and an update never removes a step.

## Use case

A specification author's abstract model keeps a value the concrete model
has overwritten, such as the previous value of a register. They add a
history field to the refinement, updated on each concrete write, and map
the abstract field from it. The concrete model and its other verdicts stay
exactly as they were.

## Inputs

- The parsed `history` rows, as QSpec FR-375's history-row production
  defines them, with spans.
- The concrete model's object types, fields and operations.
- In `qsl-replay`: a concrete state, the history values at it, and a concrete
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
  MappingFailure>`, with `MappingFailure` FR-140's.

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
- The initial `history-literal` SHALL conform to the declared type under
  QSpec FR-375's history-literal rule (FR-375-AC-9), which covers value-set
  membership including range, `none`, `true`/`false` and enum members. A
  non-member refuses `ill_typed`/`type-mismatch` at the literal, decided
  statically and not at run time.
- An operation with two updates for one history field SHALL refuse
  `invalid_model_binding`/`conflicting-binding`.
- Each update SHALL be checked with `self` bound to the step's receiver
  (post-state fields), `pre(...)` of pre-state fields and of the history
  field itself, and the step's arguments by parameter name, and its type
  SHALL conform to the declared type by `TypeEnvironment::conforms`
  (ADR-016 SC-2), refusing `ill_typed`/`type-mismatch` otherwise.
- A history field SHALL be readable only inside the declaring refinement.
  A reference to it from any other clause SHALL refuse
  `missing_declaration`/`missing-name`. In any clause other than the
  declaring refinement's rows, a read of `x.f` where the receiver's type has
  no field and no relationship end named `f`, and `f` is a history field name
  of any refinement of the unit, refuses `missing-name`; this includes an
  unrelated undeclared field of another type that shares the name. Any other
  unknown field refuses `ill_typed`/`type-mismatch` (FR-363-AC-2).
- S3 SHALL admit an update that reads a history field outside `pre(...)`.

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
- An update SHALL read a history field outside `pre(...)` as the value the
  step started from: the pre-state's history value, or the initial literal
  for an object the step creates.
- If an update evaluates `Undefined`, then `step_history` SHALL return
  `MappingFailure::Undefined` naming the history field, the object, the
  step and the evaluator's undefined cause (ADR-020 AX-3, RE-5).
- If an update evaluates refused or incomplete, or its value lies outside
  the declared type, then `step_history` SHALL return
  `MappingFailure::Undetermined(MappingUndetermined)` naming the history
  field, the object and the step.
- Either way the step SHALL remain a step of the behaviour (ADR-020 AX-3).
- If an update fails, then a post-state row that reads that history field
  SHALL NOT be evaluated at any stand-in value: the read is undetermined
  (FR-141 "Failed history update").
- History values SHALL be a function of the concrete prefix, and SHALL be
  read by the mapping (FR-140) and the product key (FR-142) only.
- The product key SHALL encode the history values in the order of
  `HistoryValues`: by object (its population, then its key), then by field
  name, each with its typed canonical value.

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
Spec::Register::write(self, x)`. For an update expression, which is
computed and not a literal, S3 admits `pre(self.writes) + 1` against a field
declared `Int[0, 1]`: static conformance compares the native type `Integer`
and not its range, so the range is left only at run time.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-138-AC-1 | `RegisterHistory` checks with one `HistoryRow` (`last`, `Int[0, 1]`, initial 0, one update on `write`). The concrete model's state nodes, the node identities of its clauses and the successors `ModelSystem` gives from its initial state are equal whether or not the refinement declares the history row. | Test (TC-543) |
| FR-138-AC-2 | Refusals: `history Impl::Register.value: …` (`invalid_model_binding`/`conflicting-binding`); `last: Int` with no bounds (`ill_typed`/`type-mismatch`); initial literal `true` or `5`, a non-member of `Int[0, 1]` (each `ill_typed`/`type-mismatch` at the literal); two `on write` updates (`invalid_model_binding`/`conflicting-binding`); `on Impl::Register::erase` (`missing_declaration`/`missing-name`); an invariant of `Impl::Register` reading `self.last` (`missing_declaration`/`missing-name`); an invariant of another type `Other` reading its undeclared field `last` (`missing_declaration`/`missing-name`); an invariant reading an undeclared non-history field `zzz` (`ill_typed`/`type-mismatch`). | Test (TC-543) |
| FR-138-AC-3 | Along `write(r, 1)`, `write(r, 0)`, `write(r, 1)` from the initial state, `step_history` gives `last` = 0, 1, 0 after each step. | Test (TC-543) |
| FR-138-AC-4 | With a second history field `writes: Int[0, 1] = 0 { on Impl::Register::write: pre(self.writes) + 1; }` added, `step_history` along `write(r, 0)`, `write(r, 0)` gives `writes` 1 after the first step and returns `MappingFailure::Undetermined(MappingUndetermined)` naming `writes`, `r` and the second step, whose post-state is still the concrete successor `ModelSystem` gives. | Test (TC-543) |
| FR-138-AC-5 | With a second history field `inv: Int[0, 1] = 0 { on Impl::Register::write: (1 mod x) + 1; }` added, `step_history` along `write(r, 1)` gives `inv` 1 (integer `/` has no typing in a refinement row, which selects no division law, so the fixture spells the intent with `mod`), and along `write(r, 0)` returns `MappingFailure::Undefined` naming `inv`, `r`, the step and cause `division-by-zero`; the post-state is still the concrete successor `ModelSystem` gives. | Test (TC-554) |
| FR-138-AC-6 | With `on Impl::Register::write: self.last;` (no `pre`), S3 admits the row and `step_history` along `write(r, 1)`, `write(r, 0)` gives `last` = 0, 0 (the value the step started from); for an object the step creates, the same read gives the initial literal. | Test (TC-543) |
| FR-138-AC-7 | Over a `Register` universe `{r, s}` with the history fields `last` and `writes`, the product key of the history values lists `r.last`, `r.writes`, `s.last`, `s.writes` in that order, and two states that differ only in one history value have different keys. | Test (TC-543) |
| FR-138-AC-8 | With `writes: Int[0, 1] = 0 { on Impl::Register::write: pre(self.writes) + 2; }` (out of type) and `prev = if 1 mod (1 - self.value + self.writes) = 0 then self.last else self.last`, `check_step` on `write(r, 1)` returns `Undetermined(MappingUndetermined)` naming the `writes` update, never `Undefined`: the `prev` row reads the failed field and is not evaluated at any stand-in. | Test (TC-546) |

## Dependencies

- ADR-020 §4 AX-1 and AX-3; ADR-016 SC-2 and FE-3.
- [FR-135](FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md),
  [FR-107](FR-107-evaluate-state-clauses-at-s6a.md) (the clause evaluator),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (`ModelSystem` states and transitions).

## References

- The QSpec half (history field semantics): QSpec FR-378 (Linear STD-133).
  QSpec FR-378 words the product key's order as "history-row order"; the key
  here orders by object, then field name. QSpec should align its wording
  (follow-up for the planner; no ticket yet).
- The history-row production, the `history-literal` spellings and the
  history-literal conformance rule: QSpec FR-375. The covering test for the
  `none`, `true`/`false` and enum-member cases, which FR-138 does not
  restate, is QSpec TC-310 mutation 10 (FR-375-AC-9).
