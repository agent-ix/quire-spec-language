---
id: TC-400
title: "The assembler refuses unresolved names, ill-formed bounds and alias cycles, reporting every error"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-400: The assembler refuses unresolved names, ill-formed bounds and alias cycles, reporting every error

## Description

Verify the assembler's three refusals and that one refusal carries every
error in the unit (ADR-011 §2.3 E3), with no `PackageDeclarations` value
returned.

Scope: FR-091-AC-11, FR-091-AC-12, FR-091-AC-13.

## Test Procedure

1. Assemble a unit with `function f using v(x: Missing): Boolean pure { true }`
   and `function g using v(y: Absent): Boolean pure { true }`.
2. Assemble a unit with three functions whose one parameter is typed
   `Int[9, 0]`, `Rational[0, 1; 0, 5]` and `Text[5, 1; nfc]` respectively.
3. Assemble a unit with `type A = B;` and `type B = A;`.

Tag the test `#[trace("FR-091-AC-11", "FR-091-AC-12", "FR-091-AC-13", "TC-400")]`.

## Expected Results

- Step 1: one refusal with two unresolved-type-name errors, naming
  `Missing` with `f`'s form span and `Absent` with `g`'s form span.
- Step 2: one refusal with three errors, each carrying the value type's own
  rejection cause (empty integer interval, denominator bound below one,
  text minimum above maximum) and its function form's span.
- Step 3: an alias-cycle error naming `A` and `B`.
- No step returns a `PackageDeclarations` value.

## Status

Planned; no test backs this case.
