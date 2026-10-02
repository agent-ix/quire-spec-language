---
id: TC-833
title: "Union values as collection elements, in queries and in predicates"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-323
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-046
    type: verifies
---
# TC-833: Union values as collection elements, in queries and in predicates

## Description

Scope: FR-323-AC-1, FR-323-AC-2, FR-323-AC-3, FR-046-AC-9.

## Test Procedure

1. Check `Set<Shape>`, `Bag<Shape>`, `OrderedSet<Shape>` and
   `Sequence<Shape>`, then build each from `Shape::Rect(2, 3)`,
   `Shape::Empty`, `Shape::Rect(2, 3)`, `Shape::Circle(1)`, and again from
   those values reversed.
2. Check `Set<F>` and `Sequence<F>` for `union F { Measured(Float64) }`.
3. Evaluate predicate `isRect(s: Shape)` (a `case` over `s`) on
   `Shape::Rect(2, 3)` and `Shape::Empty`; over a `Sequence<Shape>` of
   `Shape::Empty, Shape::Rect(2, 3), Shape::Rect(2, 3)` evaluate `contains`
   of `Shape::Rect(2, 3)`, `filter` by `isRect` and `count` by `isRect`.

Tag each test `#[trace("TC-833", "<AC id>")]`.

## Expected Results

- Step 1: all admitted; the set holds three elements and the bag four, each
  in QSpec's union-key order, the same for both input orders; the ordered
  set holds `Rect(2, 3)`, `Empty`, `Circle(1)` and, from the reversed input,
  `Circle(1)`, `Rect(2, 3)`, `Empty` (first-occurrence order, not key
  order); the sequence keeps all four in input order.
- Step 2: `Set<F>` refuses `ill_typed`/`operator-ineligible` at its type
  reference; `Sequence<F>` is admitted.
- Step 3: `true`, `false`; `contains` is `true`; `filter` keeps both `Rect`
  occurrences in order; `count` is 2.

## Status

Planned.
