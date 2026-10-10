---
id: TC-815
title: "S1 and S2 build union declaration and case forms"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-313
    type: verifies
---
# TC-815: S1 and S2 build union declaration and case forms

## Description

Scope: FR-313-AC-1, FR-313-AC-2.

## Test Procedure

1. Parse and build forms for `union Shape { Empty, Circle(Integer), Rect(Integer, Integer), }`.
2. Parse and build forms for a function whose body is
   `case s { Circle(r): r; Shape::Rect(w, h): w * h; Empty: 0; }`.
3. Build forms for the expressions `Shape::Circle(3)` and `Shape::Empty`.

Tag each test `#[trace("TC-815", "<AC id>")]`.

## Expected Results

- Step 1: one `UnionForm` `Shape`; members `Empty` (no payload), `Circle`
  (`Integer`), `Rect` (`Integer`, `Integer`) in order, each with its span.
- Step 2: one `CaseForm`, scrutinee the name `s`, arms in source order with
  member spellings `Circle`, `Shape::Rect`, `Empty`, binders `[r]`,
  `[w, h]`, `[]`, and their bodies and spans.
- Step 3: an `Expression::Call` and an `Expression::Name`; no construction
  form exists.
