---
id: FR-323
title: "Key union values in sets, bags and ordered sets by their canonical key"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-032
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-319
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-321
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-143
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-144
    type: depends_on
---
# FR-323: Key union values in sets, bags and ordered sets by their canonical key

## Description

When a set, bag or ordered set has an element type that contains a union, the
kernel SHALL key each union element by the union row of QSpec FR-144's
canonical-key table (References), so that membership, deduplication and
the canonical order of sets and bags follow that key, ordered sets keep
first-occurrence order (QSpec FR-144), and QSL invents no order (QSpec
FR-144-AC-6; ADR-012 §16.4 S3 collection element types row).

## Inputs

- A collection type whose element type contains a union; union values.

## Outputs

- An admitted collection type; collection values keyed by the FR-144 union
  key: sets and bags in key order, ordered sets in first-occurrence order.

## Behavior

- The kernel's key arm for a union value SHALL produce exactly the key
  QSpec FR-144's union row defines, and SHALL compute no other key or
  order.
- Two union values SHALL have equal keys exactly when they are equal under
  QSpec FR-143 (FR-319). A set or ordered set SHALL hold one of two equal
  union values, and a bag SHALL count both.
- A set or bag SHALL visit its union elements in FR-144 key order. An
  ordered set SHALL keep the first occurrence of each element in insertion
  order and SHALL NOT sort by key (QSpec FR-144).
- The checker SHALL admit a set, bag or ordered set whose element type
  contains a union.
- The checker SHALL admit a `Sequence<U>`, which needs no key; the evaluator
  keeps its occurrence order.
- When an operation's catalog entry names `operand:0`, `inner:0` or
  `result_inner` as its leaf source, the checker SHALL derive its leaves by
  [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md)'s
  text-leaf walk, including every union member's positional payload and
  pinned text profile. Leaf order follows type declaration order and
  payload position order; it does not follow a collection's runtime element
  order. The `result_inner` source applies only to set, bag and ordered-set
  results, as FR-093 defines.
- A union whose payload contains a type with no FR-144 key (such as an
  IEEE-bearing type) SHALL make a set, bag or ordered set of it refuse
  `ill_typed`/`operator-ineligible` at S3, on the same path as an
  IEEE-bearing element type.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-323-AC-1 | `Set<Shape>`, `Bag<Shape>` and `OrderedSet<Shape>` are admitted at S3. Building each from `Shape::Rect(2, 3)`, `Shape::Empty`, `Shape::Rect(2, 3)` and `Shape::Circle(1)` gives a set of three elements and a bag of four, each sorted by its FR-144 union key exactly as QSpec's union-key vectors order them and the same whatever the input order, and an ordered set of the three elements `Rect(2, 3)`, `Empty`, `Circle(1)` in first-occurrence order; from the reversed input the ordered set is `Circle(1)`, `Rect(2, 3)`, `Empty`. | Test (TC-833) |
| FR-323-AC-2 | `Sequence<Shape>` of the same four values is admitted and keeps all four in input order. | Test (TC-833) |
| FR-323-AC-3 | With `union F { Measured(Float64) }` (an IEEE-bearing payload), `Set<F>` refuses `ill_typed`/`operator-ineligible` at its type reference, and `Sequence<F>` is admitted. | Test (TC-833) |

## Dependencies

- QSpec FR-144 (canonical keys, FR-144-AC-6), FR-143 (union equality).
- FR-319 (identity and equality), FR-321 (kernel union value).

## References

- Owning ticket: QSL-383. Design: ADR-012 §16.4 collection row, §16.10.
- Union row in FR-144's canonical-key table (SC-G5): QSpec FR-144
  (specification ticket STD-115).
