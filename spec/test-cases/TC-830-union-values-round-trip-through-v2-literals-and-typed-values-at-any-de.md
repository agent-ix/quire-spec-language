---
id: TC-830
title: "Union values round-trip through v2 union_value nodes at any depth"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-321
    type: verifies
---
# TC-830: Union values round-trip through v2 union_value nodes at any depth

## Description

Scope: FR-321-AC-3, FR-321-AC-4.

## Test Procedure

1. Convert `Shape::Empty`, `Shape::Circle(4)` and a depth-3 `Tree` to their
   v2 `union_value` spelling and back.
2. Supply a `Tree` 10,000 levels deep; admit, convert and compare it with
   itself under default run limits.
3. Admit it with the value-occurrence limit one below its occurrence
   count.

Tag each test `#[trace("TC-830", "<AC id>")]`.

## Expected Results

- Step 1: each round trip gives an equal kernel value.
- Step 2: admitted, converted, equal to itself; no host stack overflow.
- Step 3: the value-occurrence limit, named, with its configured value.
