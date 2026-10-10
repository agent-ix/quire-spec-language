---
id: TC-821
title: "S3 checks a case, types its binders per arm and decides its result type by QSpec FR-146"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-318
    type: verifies
---
# TC-821: S3 checks a case, types its binders per arm and decides its result type by QSpec FR-146

## Description

Scope: FR-318-AC-1, FR-318-AC-2.

## Test Procedure

1. Check `area` (FR-318-AC-1) over `union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }`.
2. Check a `case` whose `Rect` arm body is a further `case` over a second
   `Shape` parameter.
3. With no enclosing expectation, check arms `Circle(r): r; Rect(w, h): true;
   Empty: 0;`.

Tag each test `#[trace("TC-821", "<AC id>")]`.

## Expected Results

- Step 1: one `Case` node over `Shape`, arms for `Empty`, `Circle`, `Rect`,
  binders `r`, `w`, `h` of type `Integer`, each unbound outside its arm;
  result `Integer`.
- Step 2: checks.
- Step 3: refuses `ill_typed`/`type-mismatch` at `true`.
