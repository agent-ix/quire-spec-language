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
   limits. Use the unchanged TC-830 step 4 Tree and QSL-503's original
   recursive sum, with exactly the same source/value through
   direct checked-call and public replay APIs. Independently enumerate the
   semantic charge sequence, retaining actual charge points and amounts.
   Set all pre-call occurrence, converted-node and helper bounds to permit
   evaluator entry; then run evaluation `work_units` 39,996 and 39,997.
   Establish the actual semantic total from the trace rather than assuming
   the source-predicted 39,997 is an executed result.
5. Run QSpec TC-265's cases (tag `QSpec-TC-265`).

Tag each test `#[trace("TC-831", "<AC id>")]`.

## Expected Results

- Step 1: 0, 27, 6, with the arm and bindings named in FR-322-AC-1; the
  inner arm's value for the nested `case`.
- Step 2: `Undefined::SumOutOfDomain` both times; no arm body is evaluated;
  `f` is never called.
- Step 3: `Err(InternalFault)` naming S6a; no refusal.
- Step 4: default completion with the original fixture's independently
  derived sum and no host stack overflow.
  At 39,996 the evaluator is entered and denies the final actual arithmetic
  charge unspent; at 39,997 it completes. Direct/replay semantic charge
  sequences are equal, including the successful prefix and denied amount.
  The incomplete names evaluation `work_units`, its configured ceiling,
  successful spend, denied amount and actual charge point. Admission or
  conversion stopping first fails this evaluator-boundary control.
  Selection/binding/helper work adds no semantic charge. No fabricated
  FunctionCall may stand for a pre-call event.
- Step 5: TC-265's outcomes.

## Status

Planned. QSL-681 phase controls are proposed, not executed.
