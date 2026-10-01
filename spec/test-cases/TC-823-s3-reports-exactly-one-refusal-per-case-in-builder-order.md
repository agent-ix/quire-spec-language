---
id: TC-823
title: "S3 reports exactly one refusal per case, in builder order"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-318
    type: verifies
---
# TC-823: S3 reports exactly one refusal per case, in builder order

## Description

Scope: FR-318-AC-4.

## Test Procedure

1. A `case` with two `Empty` arms and no `Rect` arm.
2. A `case` with an `unknown-member` arm whose body is `1 + true`.
3. A `case` whose resolved `Circle` arm body is `1 + true` and which has no
   `Rect` arm.
4. A `case` over an `Integer` scrutinee with arms naming unknown members.
5. A `case` over a scrutinee of an enum type.

Tag each test `#[trace("TC-823", "<AC id>")]`.

## Expected Results

- Step 1: only `duplicate-arm`.
- Step 2: only `unknown-member`.
- Step 3: only `ill_typed` at `1 + true`.
- Steps 4 and 5: only `ill_typed`/`type-mismatch` at the scrutinee,
  expected "a declared union".

## Status

Planned.
