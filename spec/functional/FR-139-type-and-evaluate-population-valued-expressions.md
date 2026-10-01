---
id: FR-139
title: "Type and evaluate population-valued expressions in a refinement"
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
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: depends_on
---
# FR-139: Type and evaluate population-valued expressions in a refinement

## Description

Inside a refinement declaration (field rows, step-row receivers and
arguments, history updates), QSL SHALL admit a concrete population name as
the set of that population's objects in the current state, read through the
forms `count`, `sum`, `exists`, `all`, `only` and `seq` (ADR-020 RM-8). S3
types each form; the clause evaluator evaluates it. Their meaning and
typing rules are QSpec's (ADR-020 QS-10); QSL implements them as QSpec
states them. State-clause bodies keep ADR-016 FE-4: the forms live in
refinement declarations only.

## Use case

A specification author refines a queue by a ring buffer whose contents are
spread over a population of slot objects. They build the abstract `items`
sequence from the whole slots population, ordered from the ring's head, in
one mapping row.

## Inputs

- Checked expressions inside a refinement declaration that name a concrete
  population or use one of the six forms.
- The concrete model's populations, their member types and the universe
  sizes the subject supplies.

## Outputs

- A checked population reference of type `Set<Reference<T>>[0, n]`, with `T`
  the population's member type and `n` its universe size.
- Checked forms: `count(x in P where c)`, `sum(x in P where c: e)`,
  `exists(x in P where c)`, `all(x in P where c)`, `only(x in P where c:
  e)` and `seq(i in lo .. hi: e)`, each with the type QSpec gives it.
- At evaluation, a value, or undefined for an `only` that finds no object or
  several.

## Behavior

### S3

- A concrete population name inside a refinement declaration SHALL check as
  a population reference of type `Set<Reference<T>>[0, n]`.
- A population name or one of the six forms in any other clause body SHALL
  refuse `unsupported_construct`/`expression-form` at its span.
- Each form SHALL bind its variable to `Reference<T>` (for `x in P`) or to
  an integer within the bounds of `lo` and `hi` (for `seq`), and SHALL check
  `c` as `Boolean` and `e` by its own type, refusing `ill_typed`/
  `type-mismatch` otherwise.
- `seq`'s declared maximum length SHALL be computed from the static bounds
  of `lo` and `hi`, as QSpec states it.
- The form's result SHALL be checked against its use (an abstract field, a
  parameter, a history field) as FR-135 checks a field row.

### Evaluation

- A population reference SHALL evaluate to the objects of that population
  in the observed state, in ascending key order.
- Each form SHALL be evaluated through the one clause evaluator (ADR-016
  FE-3, FR-107) over the state's observation, charging one work unit per
  object visited and per `seq` element.
- `only` SHALL evaluate to `e` for the one object satisfying `c`; when no
  object or more than one satisfies it, the form SHALL be undefined, and the
  mapping that contains it is undetermined (FR-140).
- `seq(i in lo .. hi: e)` SHALL evaluate to the sequence of `e` for each
  integer from `lo` to `hi` in ascending order, empty when `hi < lo`.

## Acceptance Criteria

Fixture: ADR-020 §8's `RingIsQueue` with universes `rings = {r}` and `slots
= {s0, s1}` (indices 0 and 1).

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-139-AC-1 | `RingIsQueue`'s `items` row checks, with `R::slots` typed `Set<Reference<R::Slot>>[0, 2]` and the `seq` form's declared maximum length 2, conforming to `Sequence<Int[0, 1]>[0, 2]`. `count(s in R::slots where s.ring = self)` checks as an integer and `exists(s in R::slots where s.value = 1)` as `Boolean`. | Test (TC-544) |
| FR-139-AC-2 | An invariant of `R::Ring` reading `count(s in R::slots where s.ring = self)` refuses `unsupported_construct`/`expression-form`. `only(s in R::slots where s.index: s.value)` refuses `ill_typed`/`type-mismatch` at `s.index`. | Test (TC-544) |
| FR-139-AC-3 | At the state `head = 1`, `size = 2`, `s0.value = 0`, `s1.value = 1`, `items` evaluates to `[1, 0]`; at `head = 0`, `size = 0` it evaluates to `[]`; `count(s in R::slots where s.ring = self)` evaluates to 2 and `all(s in R::slots where s.value = 1)` to `false`. | Test (TC-544) |
| FR-139-AC-4 | `only(s in R::slots where s.ring = self: s.value)` is undefined at every state of the fixture (two slots satisfy it), and `only(s in R::slots where s.ring = self and s.index = self.head + 2: s.value)` is undefined (no slot satisfies it). | Test (TC-544) |

## Dependencies

- ADR-020 §1 RM-8, §8 data refinement and §11 RU-5; ADR-016 FE-3 and FE-4.
- [FR-135](FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md),
  [FR-107](FR-107-evaluate-state-clauses-at-s6a.md).
- FR-140 reads an undefined form as an undefined mapping row, which refutes
  the refinement (ADR-020 RE-5).

## References

- The QSpec half (the forms, their typing and `seq`'s maximum length): QSpec FR-376 (Linear STD-133).
