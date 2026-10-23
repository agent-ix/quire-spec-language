---
id: TC-819
title: "S3 checks union construction and refuses FR-143's four construction errors"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-317
    type: verifies
---
# TC-819: S3 checks union construction and refuses FR-143's four construction errors

## Description

Scope: FR-317-AC-1, FR-317-AC-2, FR-317-AC-4, FR-317-AC-5.

## Test Procedure

Over `union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }`:

1. Check `Shape::Empty`, `Shape::Rect(2, 3)` and `Shape::Rect(2, true)`.
2. Check `Shape::Square(1)`, `Shape::Empty(1)`, bare `Shape::Circle` and
   `Shape::Rect(1)`.
3. Check `Shape::Rect(2, 3)` and `Shape::Rect(1)` as part of a state clause
   body and of a `decreases` measure.
4. Run QSpec TC-262's construction cases (tag `QSpec-TC-262`).
5. Over ADR-022 §7.2's `Game` unit (enum `G::Phase`), check the bare and the
   qualified `CanStillWin` with `from`, `Lost != x.phase`, `x.phase = Draw`,
   `Lost = Won`, `x.phase < Mid` and `let Lost = 1 in x.phase != Lost`; compare the node ids of the bare and qualified claims.

Tag each test `#[trace("TC-819", "<AC id>")]`.

## Expected Results

- Step 1: two union constructions of type `Shape` (members `Empty`,
  `Rect`); `ill_typed`/`type-mismatch` at `true`.
- Step 2: `ill_typed`/`type-mismatch` at each construction expression, one
  refusal each.
- Step 3: the same verdicts as in a function body, under each clause kind.
- Step 4: TC-262's verdicts.
- Step 5: the bare and qualified claims check equal and emit one node id;
  `Lost != x.phase` checks as `x.phase != Lost`; `missing_declaration`/
  `missing-name` at `Draw`, at `Lost` in `Lost = Won`, and at `Mid`;
  `ambiguous_declaration`/`ambiguous-name` at the compared `Lost`, naming the
  `let` binder and `G::Phase::Lost`.
