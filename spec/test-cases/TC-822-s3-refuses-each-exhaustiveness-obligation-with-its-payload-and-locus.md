---
id: TC-822
title: "S3 refuses each exhaustiveness obligation with its payload and locus"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-318
    type: verifies
---
# TC-822: S3 refuses each exhaustiveness obligation with its payload and locus

## Description

Scope: FR-318-AC-3.

## Test Procedure

Over `union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }` and `union Other { X }`, check one `case` per row:

1. two `Empty` arms plus `Circle` and `Rect`;
2. an arm `Other::X` plus all `Shape` members;
3. `Circle(a, b)` plus the other members;
4. arms for `Empty` and `Circle` only.
5. Run QSpec TC-264's cases (tag `QSpec-TC-264`).

Tag each test `#[trace("TC-822", "<AC id>")]`.

## Expected Results

Each refuses `undefined_expression`/`unproved-exhaustiveness` naming
`Shape`'s identity and the arm set:

- Step 1: `duplicate-arm`, at the second `Empty` arm.
- Step 2: `unknown-member`, at `Other::X`.
- Step 3: `arm-arity`, at `Circle(a, b)`.
- Step 4: `missing-arm`, at the `case`.
- Step 5: TC-264's verdicts.

## Status

Planned.
