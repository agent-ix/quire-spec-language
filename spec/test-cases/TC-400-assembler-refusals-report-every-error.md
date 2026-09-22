---
id: TC-400
title: "The assembler refuses unresolved and ambiguous names, ill-formed bounds and alias cycles, reporting every error"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-400: The assembler refuses unresolved and ambiguous names, ill-formed bounds and alias cycles, reporting every error

## Description

Verify the assembler's name-resolution and bounds refusals. One refusal
carries every error in the unit (ADR-011 §2.3 E3; ADR-013 O-11), and no
`PackageDeclarations` value comes back.

Scope: FR-091-AC-14, FR-091-AC-15, FR-091-AC-16, FR-091-AC-17.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. Assemble a unit with `function f using v(x: Missing): Boolean pure { true }`
   and `function g using v(y: Absent): Boolean pure { true }`.
2. Assemble a unit with `type A = Int[0, 1];`, `type A = Int[0, 2];` and
   `function f using v(x: A): Boolean pure { true }`.
3. Assemble a unit with three functions, whose single parameters are typed
   `Int[9, 0]`, `Rational[0, 1; 0, 5]` and `Text[5, 1; nfc]`.
4. Assemble a unit with `type A = B;` and `type B = A;`.

Tag the test `#[trace("FR-091-AC-14", "FR-091-AC-15", "FR-091-AC-16", "FR-091-AC-17", "TC-400")]`.

## Expected Results

- Step 1 gives one refusal with two unresolved-type-name errors, naming
  `Missing` and `Absent`, each with its type form's span.
- Step 2 gives an ambiguous-type-name error naming `A`, the span of `x`'s
  type form, and both `type A` declaration spans.
- Step 3 gives one refusal with three errors. Each carries the value type's
  own rejection cause (an empty integer interval, a denominator bound below
  one, a text minimum above its maximum) and its type form's span.
- Step 4 gives an alias-cycle error naming `A` and `B`.
- No step returns a `PackageDeclarations` value.

## Status

Planned; no test backs this case.
