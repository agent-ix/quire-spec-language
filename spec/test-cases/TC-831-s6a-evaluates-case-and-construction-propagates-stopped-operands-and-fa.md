---
id: TC-831
title: "S6a evaluates case and construction, propagates stopped operands and faults on a broken invariant"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-322
    type: verifies
---
# TC-831: S6a evaluates case and construction, propagates stopped operands and faults on a broken invariant

## Description

Scope: FR-322-AC-1, FR-322-AC-2, FR-322-AC-4.

## Test Procedure

1. Evaluate `area` (FR-318-AC-1) on `Shape::Empty`, `Shape::Circle(3)` and
   `Shape::Rect(2, 3)`; evaluate a `case` with a nested `case` arm.
2. Under `CheckMode::Kernel`, with `type Small = Int[0, 3]` and `q` holding
   `2, 2`, evaluate a `case` over `Shape::Circle(sum<Small>(x in q: x))` and
   the construction `Shape::Rect(sum<Small>(x in q: x), f())` with `f`
   instrumented.
3. Inject a scrutinee whose `VariantId` matches no arm.
4. Evaluate a recursive sum over a `Tree` 10,000 levels deep under default
   limits, then with `work_units` one below its total.
5. Run QSpec TC-265's cases (tag `QSpec-TC-265`).

Tag each test `#[trace("TC-831", "<AC id>")]`.

## Expected Results

- Step 1: 0, 27, 6, with the arm and bindings named in FR-322-AC-1; the
  inner arm's value for the nested `case`.
- Step 2: `Undefined::SumOutOfDomain` both times; no arm body is evaluated;
  `f` is never called.
- Step 3: `Err(InternalFault)` naming S6a; no refusal.
- Step 4: completes; then `incomplete` naming `work_units` and its configured
  value; no host stack overflow.
- Step 5: TC-265's outcomes.
