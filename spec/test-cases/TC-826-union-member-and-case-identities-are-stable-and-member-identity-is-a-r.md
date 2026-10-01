---
id: TC-826
title: "Union, member and case identities are stable and member identity is a retyped VariantId"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-319
    type: verifies
---
# TC-826: Union, member and case identities are stable and member identity is a retyped VariantId

## Description

Scope: FR-319-AC-1, FR-319-AC-2, FR-319-AC-3.

## Test Procedure

1. Compile a unit with `union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }` and `area` (FR-318-AC-1) twice, then with whitespace and
   comments changed and `area`'s arms reordered.
2. Rename member `Empty`; separately add a member; separately change
   `Circle`'s payload type.
3. For each `Shape` member compare the check-time `VariantId`, the
   admission-time `VariantId` and the member's node key bytes. Compute the
   `VariantId`s of `X` in `union A { X }`, `union B { X }` and
   `enum E { X }`.
4. Evaluate `Shape::Rect(2, 3) = Shape::Rect(2, 3)`,
   `Shape::Rect(2, 3) = Shape::Rect(3, 2)` and
   `Shape::Empty = Shape::Circle(0)`; check `A::X(1) = B::X(1)` over
   `union A { X(Integer) }` and `union B { X(Integer) }`.
5. Run QSpec TC-262's equality cases (tag `QSpec-TC-262`).

Tag each test `#[trace("TC-826", "<AC id>")]`.

## Expected Results

- Step 1: equal union node id, member `VariantId`s, `case` node id and
  `package_id` across all three compiles.
- Step 2: each change gives a new union node id and new ids for every node
  naming `Shape`.
- Step 3: all three equal per member; the three `X` `VariantId`s differ.
- Step 4: `true`, `false`, `false`; the cross-union comparison is refused by
  type checking.
- Step 5: TC-262's verdicts.

## Status

Planned.
