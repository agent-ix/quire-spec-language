---
id: TC-817
title: "S3 admits union declarations, including an escaping recursive union"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-316
    type: verifies
---
# TC-817: S3 admits union declarations, including an escaping recursive union

## Description

Scope: FR-316-AC-1, FR-316-AC-2.

## Test Procedure

1. Check `union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }` beside `enum Kind { Empty, Circle }`.
2. Check `union Tree { Leaf, Node(Integer, Option<Tree>, Option<Tree>) }`.
3. Check `union L { Nil, Cons(Integer, Option<L>) }`.
4. Run QSpec TC-263's admitted cases (tag `QSpec-TC-263`).

Tag each test `#[trace("TC-817", "<AC id>")]`.

## Expected Results

- Step 1: admitted; the type environment holds one union composite with
  the members and payload types in declared order; `Kind` keeps its enum
  type and is not `Shape`'s type.
- Steps 2 and 3: admitted.
- Step 4: TC-263's verdicts.
