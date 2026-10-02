---
id: FR-333
title: "Read an optional collection bound and population maximum from source to the kernel type"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-144
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-153
    type: depends_on
---
# FR-333: Read an optional collection bound and population maximum from source to the kernel type

## Description

S1, S2 and the S3 type-form resolver SHALL accept a collection type with or
without a cardinality bound, `K<T>` or `K<T>[a,b]` for `K` in `Sequence`,
`Set`, `Bag` and `OrderedSet`, and a population type with or without a
maximum, `Population<T>` or `Population<T>[N]`. An absent bound SHALL resolve
to the unbounded kernel type: `CollectionType.bound == None` and
`ValueType::Population(None)` (ADR-014 §2, N-3; QSpec FR-144, FR-153-AC-9).

## Use case

An author declares `accounts: Set<Account>` because the system has no fixed
number of accounts. The compiler accepts it as written and types it as an
unbounded set, rather than refusing it or inventing a maximum the author
never chose.

## Inputs

- A unit's source bytes with type references (FR-001).

## Outputs

- S2 `TypeForm`s whose `bounds` list is empty when the source writes no
  bound.
- The resolved kernel `ValueType`.
- A `CheckRefusal` with a span on refusal.

## Behavior

- S1 SHALL parse a collection type reference with an optional `[uint,
  uint]` bound and a population type reference with an optional `[uint]`
  maximum.
- S2 SHALL build the type form with the bounds as written, and with an
  empty bounds list when none is written (FR-091).
- When a collection form has no bound, the resolver SHALL return
  `CollectionType::new(kind, element, None)`. When it has `[a,b]`, it SHALL
  return `Some(CardinalityBound{minimum: a, maximum: b})`, and SHALL refuse
  `ill_typed`/`type-mismatch` at the form's span when `a > b`.
- When a population form has no maximum, the resolver SHALL return
  `ValueType::Population(None)`; with `[N]`, `Population(Some(N))`.
- Checking an unbounded type's values, queries and lowering is FR-097-AC-7
  and FR-097-AC-8; this requirement delivers the type those criteria check.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-333-AC-1 | `function f using v(s: Set<Int>): Integer pure { 0 }` compiles through the spine, and `s`'s resolved type is a `Set` with element `Int` and `bound: None`; with `Set<Int>[0, 8]` the bound is `Some{0, 8}`, and the two parameters' types are unequal. Each of `Sequence`, `Bag` and `OrderedSet` with no bound resolves to `bound: None`. | Test (TC-843) |
| FR-333-AC-2 | A parameter `p: Population<Account>` resolves to `ValueType::Population(None)` and `p: Population<Account>[4]` to `Population(Some(4))`. | Test (TC-843) |
| FR-333-AC-3 | `Set<Int>[3, 2]` refuses `ill_typed`/`type-mismatch` at the type form's span; `Set<Int>[0, x]` fails at S1 with a parse diagnostic at `x`'s span. | Test (TC-843) |

## Dependencies

- ADR-014 §1 B-1 and B-6, §2, §9 N-3, §11.
- [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md)
  (value forms), [FR-097](FR-097-classify-claim-extent-and-write-bounded-requests.md)
  AC-7 and AC-8 (checking and lowering unbounded types).
- QSpec FR-144-AC-9, AC-12, AC-13; FR-153-AC-9.

## References

- Linear QSL-385 (spec ticket); QSL-42 (implementation).
- QSpec half: QSpec FR-144 (merged).
